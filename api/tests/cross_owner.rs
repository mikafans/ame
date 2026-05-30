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
        panic!("failed to run migrations for cross_owner integration test: {error}");
    }

    pool
}

#[tokio::test]
async fn test_cross_owner_agent_visibility() {
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

    // 1. Create owner and an agent
    let owner_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, 'Owner', 'user')",
    )
    .bind(owner_id)
    .bind(format!("owner-{}@example.com", owner_id))
    .execute(&pool)
    .await
    .unwrap();

    let agent_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, owner_user_id, display_name, role) VALUES ($1, $2, 'Agent', 'agent')")
        .bind(agent_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();

    // 2. Agent creates a private quiz
    let token_id = Uuid::now_v7();
    let secret = "agent-secret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(token_id)
        .bind(agent_id)
        .bind("agent-key")
        .bind(&hash)
        .bind(vec!["quiz.read".to_string(), "quiz.write".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let agent_auth = format!("{token_id}_{secret}");

    let res = client
        .post(format!("{base_url}/v1/quizzes"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({"title": "Agent Quiz", "visibility": "private"}))
        .send()
        .await
        .unwrap();
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap();
        panic!("Quiz creation failed ({status}): {body}");
    }
    let json: serde_json::Value = res.json().await.unwrap();
    let qid = json["id"].as_str().unwrap().to_string();

    // 3. Test: Owner GET agent's private quiz should succeed
    let owner_token_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(owner_token_id)
        .bind(owner_id)
        .bind("owner-key")
        .bind(&hash) // Same hash for simplicity
        .bind(vec!["quiz.read".to_string()])
        .execute(&pool)
        .await
        .unwrap();

    let owner_auth = format!("{owner_token_id}_{secret}");

    let res = client
        .get(format!("{base_url}/v1/quizzes/{qid}"))
        .header("Authorization", format!("Bearer {owner_auth}"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 4. Test: Unrelated user GET agent's private quiz should 404
    let stranger_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, 'Stranger', 'user')",
    )
    .bind(stranger_id)
    .bind(format!("stranger-{}@example.com", stranger_id))
    .execute(&pool)
    .await
    .unwrap();
    let stranger_token_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(stranger_token_id)
        .bind(stranger_id)
        .bind("stranger-key")
        .bind(&hash)
        .bind(vec!["quiz.read".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let stranger_auth = format!("{stranger_token_id}_{secret}");

    let res = client
        .get(format!("{base_url}/v1/quizzes/{qid}"))
        .header("Authorization", format!("Bearer {stranger_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 5. Test: Agent creates a session for its own private quiz
    let res = client
        .post(format!("{base_url}/v1/sessions"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({"quizId": qid}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 6. Test: Owner creates a session for agent's private quiz
    let res = client
        .post(format!("{base_url}/v1/sessions"))
        .header("Authorization", format!("Bearer {owner_auth}"))
        .json(&json!({"quizId": qid}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}
