use ame_api::http::router;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::net::SocketAddr;

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
        panic!("failed to run migrations for me integration test: {error}");
    }

    pool
}

#[tokio::test]
async fn test_me_agents_crud() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let app = router(pool.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{addr}");

    // 1. Create a human user
    let alice_id = uuid::Uuid::now_v7();
    let alice_email = format!("agent-alice-{}@example.com", alice_id);
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, $3, 'user')")
        .bind(alice_id)
        .bind(&alice_email)
        .bind("Alice")
        .execute(&pool)
        .await
        .unwrap();

    let token_id = uuid::Uuid::now_v7();
    let secret = "very-secret-token";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, 'test-key', $3, $4)")
        .bind(token_id)
        .bind(alice_id)
        .bind(&hash)
        .bind(vec!["quiz.read", "quiz.write", "admin"]) // Admin for registration/creation
        .execute(&pool)
        .await
        .unwrap();

    let auth_header = format!("{token_id}_{secret}");

    // 2. Create agent
    let res = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth_header}"))
        .json(&json!({
            "label": "test-agent",
            "scopes": ["quiz.read"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body: serde_json::Value = res.json().await.unwrap();
    let agent_id = body["id"].as_str().unwrap();
    let agent_token = body["apiKey"].as_str().unwrap();

    // 3. List agents
    let res = client
        .get(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth_header}"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let agents = body["agents"].as_array().unwrap();
    assert!(
        agents
            .iter()
            .any(|a| a["id"] == agent_id && a["label"] == "test-agent")
    );

    // 4. Update agent (patch label)
    let res = client
        .patch(format!("{base_url}/v1/me/agents/{agent_id}"))
        .header("Authorization", format!("Bearer {auth_header}"))
        .json(&json!({
            "label": "updated-agent"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Verify update
    let res = client
        .get(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth_header}"))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(
        body["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == agent_id && a["label"] == "updated-agent")
    );

    // 5. Update agent (empty patch) -> should 204
    let res = client
        .patch(format!("{base_url}/v1/me/agents/{agent_id}"))
        .header("Authorization", format!("Bearer {auth_header}"))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 6. Update agent (wrong owner) -> should 404
    let bob_id = uuid::Uuid::now_v7();
    let bob_email = format!("agent-bob-{}@example.com", bob_id);
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, $3, 'user')")
        .bind(bob_id)
        .bind(&bob_email)
        .bind("Bob")
        .execute(&pool)
        .await
        .unwrap();
    let bob_token_id = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, 'bob-key', $3, $4)")
        .bind(bob_token_id)
        .bind(bob_id)
        .bind(&hash)
        .bind(vec!["quiz.read"])
        .execute(&pool)
        .await
        .unwrap();
    let bob_auth = format!("{bob_token_id}_{secret}");

    let res = client
        .patch(format!("{base_url}/v1/me/agents/{agent_id}"))
        .header("Authorization", format!("Bearer {bob_auth}"))
        .json(&json!({"label": "hacked"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 7. Delete agent
    let res = client
        .delete(format!("{base_url}/v1/me/agents/{agent_id}"))
        .header("Authorization", format!("Bearer {auth_header}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 8. Verify zombie prevention: list agents should be empty and NOT panic
    let res = client
        .get(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth_header}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(
        !body["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == agent_id)
    );

    // 9. Verify token is revoked/removed
    let res = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {agent_token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
