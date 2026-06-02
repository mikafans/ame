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
        panic!("failed to run migrations for quota test: {error}");
    }

    pool
}

#[tokio::test]
async fn test_quota_enforcement() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
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

    // 1. Create a free user
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Free User', 'user', 'free')")
        .bind(user_id)
        .bind(format!("free-{}@example.com", user_id))
        .execute(&pool)
        .await
        .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "free-secret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(token_id)
        .bind(user_id)
        .bind("free-key")
        .bind(&hash)
        .bind(vec!["assessment.read".to_string(), "assessment.write".to_string(), "admin".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let auth = format!("{token_id}_{secret}");

    // 2. Test Agent Creation Quota (Free plan: 1 limit)
    let res = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({"label": "Agent 1", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res2 = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({"label": "Agent 2", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
    let error_body = res2.json::<serde_json::Value>().await.unwrap();
    assert_eq!(error_body["error"]["code"], "quota_exceeded");
    assert_eq!(error_body["error"]["details"]["kind"], "agent_creation");
    assert_eq!(error_body["error"]["details"]["limit"], 1);

    // Test that Premium user has higher limits (up to 100 agents)
    // Create a premium user
    let prem_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Prem User', 'user', 'premium')")
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
        .bind(vec!["assessment.read".to_string(), "assessment.write".to_string(), "admin".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let prem_auth = format!("{prem_token_id}_{secret}");

    // Create 2nd agent (should succeed because limit is 100)
    client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {prem_auth}"))
        .json(&json!({"label": "Agent 1", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {prem_auth}"))
        .json(&json!({"label": "Agent 2", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}
