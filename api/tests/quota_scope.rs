use ame_api::http::router;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::net::SocketAddr;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../db/migrations");

async fn setup_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("set DATABASE_URL to a reachable Postgres database before AME_RUN_DB_TESTS=1");

    if let Err(error) = MIGRATOR.run(&pool).await {
        panic!("failed to run migrations: {error}");
    }

    pool
}

#[tokio::test]
async fn test_public_publish_scope_and_ceiling() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }

    let pool = setup_db().await;
    let app = router(pool.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{addr}");

    // 1. Create a free human user
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Free Human', 'user', 'free')")
        .bind(user_id)
        .bind(format!("user-{}@example.com", user_id))
        .execute(&pool)
        .await
        .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "secret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(token_id)
        .bind(user_id)
        .bind("key")
        .bind(&hash)
        .bind(vec![
            "assessment.read".to_string(),
            "assessment.write".to_string(),
            "attempt.read".to_string(),
            "attempt.write".to_string(),
            "stats.read".to_string(),
            "feedback.write".to_string(),
            "plan.read".to_string(),
            "plan.write".to_string(),
            "public.publish".to_string(),
        ])
        .execute(&pool)
        .await
        .unwrap();
    let auth = format!("{token_id}_{secret}");

    // 2. Assert the Free human owner can successfully publish a public assessment within quota (limit is 5)
    let res = client
        .post(format!("{base_url}/v1/assessments"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({"title": "Free User Public Assessment", "visibility": "public"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 3. Assert Free owner cannot grant 'public.publish' scope to an agent (ceiling enforcement)
    let res = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({
            "label": "My Public Agent",
            "scopes": ["assessment.read", "public.publish"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let body = res.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["error"]["code"], "scope_required");
    assert_eq!(body["error"]["details"]["scope"], "public.publish");

    // 4. Create a premium human user
    let prem_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Premium Human', 'user', 'premium')")
        .bind(prem_id)
        .bind(format!("prem-{}@example.com", prem_id))
        .execute(&pool)
        .await
        .unwrap();

    let prem_token_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(prem_token_id)
        .bind(prem_id)
        .bind("prem-key")
        .bind(&hash)
        .bind(vec![
            "assessment.read".to_string(),
            "assessment.write".to_string(),
            "attempt.read".to_string(),
            "attempt.write".to_string(),
            "stats.read".to_string(),
            "feedback.write".to_string(),
            "plan.read".to_string(),
            "plan.write".to_string(),
            "public.publish".to_string(),
        ])
        .execute(&pool)
        .await
        .unwrap();
    let prem_auth = format!("{prem_token_id}_{secret}");

    // 5. Assert Premium owner CAN grant 'public.publish' scope to an agent
    let res = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {prem_auth}"))
        .json(&json!({
            "label": "My Prem Agent",
            "scopes": ["assessment.read", "public.publish"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}
