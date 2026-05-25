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
    domain::user::Scope,
    http::{AppState, idempotency::idempotency_middleware},
};

struct WriteQuestionsScope;
impl ScopeConstraint for WriteQuestionsScope {
    const SCOPE: Scope = Scope::QuizWrite;
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../db/migrations");

fn create_test_router(pool: PgPool) -> Router {
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
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency_middleware,
        ))
        .with_state(state)
}

async fn setup_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("set DATABASE_URL to a reachable Postgres database before AME_RUN_DB_TESTS=1");

    if let Err(error) = MIGRATOR.run(&pool).await {
        panic!("failed to run migrations for auth integration test: {error}");
    }

    pool
}

#[tokio::test]
async fn test_auth_and_idempotency() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

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

    sqlx::query(
        "INSERT INTO users (id, display_name, email, role) \
         VALUES ($1, 'Test User', $2, 'learner')",
    )
    .bind(user_id)
    .bind(format!("auth-{user_id}@example.com"))
    .execute(&pool)
    .await
    .unwrap();

    let scopes = vec!["quiz.read".to_string()];
    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, 'test token', $3, $4)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(hash)
    .bind(&scopes)
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
    assert_eq!(res.status(), StatusCode::FORBIDDEN); // Missing "quiz.write"
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["error"]["details"]["scope"], "quiz.write");

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

    // Same token id and idempotency key with the wrong secret must not replay
    // the stored response before authentication runs.
    let invalid_bearer_token = format!("{}_wrong_secret", token_id);
    let res = client
        .post(format!("{base_url}/idempotent"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", invalid_bearer_token),
        )
        .header("Idempotency-Key", idem_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

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

/// Tokens with `revoked_at IS NOT NULL` must be rejected even when the bearer
/// secret matches. This guards the revocation branch in
/// `api/src/auth/extractor.rs`; without this test, a regression that flipped
/// the conditional would only surface in production.
#[tokio::test]
async fn revoked_token_returns_unauthorized() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let app = create_test_router(pool.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let user_id = uuid::Uuid::now_v7();
    let token_id = uuid::Uuid::now_v7();
    let secret = "revoked_secret_xyz";

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query(
        "INSERT INTO users (id, display_name, email, role) \
         VALUES ($1, 'Revoked User', $2, 'learner')",
    )
    .bind(user_id)
    .bind(format!("revoked-{user_id}@example.com"))
    .execute(&pool)
    .await
    .unwrap();

    // revoked_at set at insert time — the secret hash is still valid, so any
    // 200 here would prove the extractor stopped checking revocation.
    let scopes = vec!["quiz.read".to_string()];
    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes, revoked_at) \
         VALUES ($1, $2, 'revoked token', $3, $4, now())",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(hash)
    .bind(&scopes)
    .execute(&pool)
    .await
    .unwrap();

    let bearer_token = format!("{}_{}", token_id, secret);
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://{addr}/protected"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
