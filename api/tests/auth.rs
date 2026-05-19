use std::net::SocketAddr;

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::{
    Json, Router, middleware,
    routing::{get, post},
};
use rand::rngs::OsRng;
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;

use ame_api::{
    auth::{extractor::AuthenticatedUser, scope::RequireScope, scope::ScopeConstraint},
    http::{AppState, idempotency::idempotency_middleware},
};

struct WriteQuestionsScope;
impl ScopeConstraint for WriteQuestionsScope {
    const SCOPE: &'static str = "agent:write-questions";
}

fn create_test_router(pool: PgPool) -> Router<AppState> {
    let state = AppState { pool };
    Router::new()
        .route(
            "/protected",
            get(|user: AuthenticatedUser| async move { Json(json!({ "user_id": user.user.id })) }),
        )
        .route(
            "/scoped",
            get(|_user: RequireScope<WriteQuestionsScope>| async move {
                Json(json!({ "status": "ok" }))
            }),
        )
        .route(
            "/idempotent",
            post(|_user: AuthenticatedUser, body: Json<Value>| async move {
                // Return exactly what was sent
                Json(body.0)
            }),
        )
        .route_layer(middleware::from_fn(idempotency_middleware))
        .with_state(state)
}

async fn setup_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .unwrap()
}

#[tokio::test]
async fn test_auth_and_idempotency() {
    let pool = setup_db().await;

    let app = create_test_router(pool.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{addr}");

    // Test 1: No token
    let res = client
        .get(format!("{base_url}/protected"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Setup User and Token
    let user_id = uuid::Uuid::now_v7();
    let token_id = uuid::Uuid::now_v7();
    let secret = "my_secret_token_123";

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query!(
        "INSERT INTO users (id, display_name, role) VALUES ($1, 'Test User', 'user')",
        user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let scopes = vec!["human".to_string(), "agent:read-only".to_string()];
    sqlx::query!(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, 'test token', $3, $4)",
        token_id,
        user_id,
        hash,
        &scopes
    )
    .execute(&pool)
    .await
    .unwrap();

    let bearer_token = format!("{}_{}", token_id, secret);

    // Test 2: Valid token
    let res = client
        .get(format!("{base_url}/protected"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["user_id"].as_str().unwrap(), user_id.to_string());

    // Test 3: Scope required but missing
    let res = client
        .get(format!("{base_url}/scoped"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN); // Missing "agent:write-questions"

    // Test 4: Idempotency
    let idem_key = "test-key-1";
    let payload = json!({ "question": "test" });

    let res1 = client
        .post(format!("{base_url}/idempotent"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .header("Idempotency-Key", idem_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res1.status(), StatusCode::OK);

    // Give background task time to save to DB
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Same request -> should succeed (replay)
    let res2 = client
        .post(format!("{base_url}/idempotent"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .header("Idempotency-Key", idem_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body2: Value = res2.json().await.unwrap();
    assert_eq!(body2, payload);

    // Different body -> conflict
    let new_payload = json!({ "question": "different" });
    let res3 = client
        .post(format!("{base_url}/idempotent"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .header("Idempotency-Key", idem_key)
        .json(&new_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res3.status(), StatusCode::CONFLICT);
}
