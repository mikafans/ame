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
        panic!("failed to run migrations for agent_tools integration test: {error}");
    }

    pool
}

#[tokio::test]
async fn test_agent_behavioral_tools() {
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

    // 1. Create owner and seed some ratings
    let owner_id = Uuid::now_v7();
    let owner_email = format!("agent-owner-{}@example.com", owner_id);
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, $3, 'user')")
        .bind(owner_id)
        .bind(&owner_email)
        .bind("Alice Owner")
        .execute(&pool)
        .await
        .unwrap();

    let tag_id = Uuid::now_v7();
    let tag_name = format!("rust-{}", tag_id);
    sqlx::query("INSERT INTO tb_tags (id, name) VALUES ($1, $2)")
        .bind(tag_id)
        .bind(&tag_name)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tb_user_tag_ratings (user_id, tag_id, rating, attempts_count) VALUES ($1, $2, $3, $4)")
        .bind(owner_id)
        .bind(tag_id)
        .bind(1550.5f64)
        .bind(10i32)
        .execute(&pool)
        .await
        .unwrap();

    // 2. Create agent linked to owner
    let agent_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, owner_user_id, display_name, role) VALUES ($1, $2, $3, 'agent')",
    )
    .bind(agent_id)
    .bind(owner_id)
    .bind("Agent Rust")
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tb_agent_profiles (agent_user_id, label, focus_tags) VALUES ($1, $2, $3)",
    )
    .bind(agent_id)
    .bind("Agent Rust")
    .bind(vec![tag_name.clone()])
    .execute(&pool)
    .await
    .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "agent-secret-123";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)")
        .bind(token_id)
        .bind(agent_id)
        .bind("agent-key")
        .bind(&hash)
        .bind(vec!["assessment.read", "assessment.write"])
        .execute(&pool)
        .await
        .unwrap();

    let agent_auth = format!("{token_id}_{secret}");

    // 3. Test profile.get (Shared Truth check)
    let res = client
        .post(format!("{base_url}/v1/agents/run"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({
            "tool": "profile.get",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    if !body["ok"].as_bool().unwrap_or(false) {
        panic!("Tool run failed: {}", body["error"]);
    }

    let result = &body["result"];
    assert_eq!(result["agent"]["label"], "Agent Rust");
    assert_eq!(result["owner"]["id"], owner_id.to_string());

    let ratings = result["owner"]["ratings"].as_array().unwrap();
    let rust_rating = ratings.iter().find(|r| r["tag"] == tag_name).unwrap();
    assert_eq!(rust_rating["rating"], 1550.5);

    // 4. Test memory.set and memory.append
    let res = client
        .post(format!("{base_url}/v1/agents/run"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({
            "tool": "memory.set",
            "params": { "memory": { "key1": "val1" } }
        }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    let res = client
        .post(format!("{base_url}/v1/agents/run"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({
            "tool": "memory.append",
            "params": { "append": { "key2": "val2" } }
        }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Verify memory merge via profile.get
    let res = client
        .post(format!("{base_url}/v1/agents/run"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({ "tool": "profile.get" }))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    let mem = &body["result"]["agent"]["memory"];
    assert_eq!(mem["key1"], "val1");
    assert_eq!(mem["key2"], "val2");

    // 5. Test target.set
    let res = client
        .post(format!("{base_url}/v1/agents/run"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({
            "tool": "target.set",
            "params": { "currentGoal": "Master async Rust", "nextTarget": "Read Tokio docs" }
        }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Verify targets
    let res = client
        .post(format!("{base_url}/v1/agents/run"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({ "tool": "profile.get" }))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    let agent = &body["result"]["agent"];
    assert_eq!(agent["currentGoal"], "Master async Rust");
    assert_eq!(agent["nextTarget"], "Read Tokio docs");
}

/// Regression: an agent token reading learner-record endpoints (`/v1/me/stats`,
/// `/v1/me/attempts`) must see the OWNER's data, not the agent sub-account's
/// (which has none). Previously these scoped by the caller's own id and returned
/// empty for agents.
#[tokio::test]
async fn test_agent_token_reads_owner_stats_and_attempts() {
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

    // Owner with a finished session + attempt (the learning record an agent reads).
    let owner_id = Uuid::now_v7();
    let owner_email = format!("stats-owner-{}@example.com", owner_id);
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, $3, 'user')")
        .bind(owner_id)
        .bind(&owner_email)
        .bind("Stats Owner")
        .execute(&pool)
        .await
        .unwrap();

    let question_id: Uuid = sqlx::query_scalar(
        "INSERT INTO tb_questions (owner_id, kind, prompt, payload, status, points, created_by) \
         VALUES ($1, 'mc', 'Q', $2, 'live', 1, $1) RETURNING id",
    )
    .bind(owner_id)
    .bind(json!({ "options": ["a", "b"], "correct_index": 0 }))
    .fetch_one(&pool)
    .await
    .unwrap();

    let assessment_id: Uuid = sqlx::query_scalar(
        "INSERT INTO tb_assessments (title, mode, status, created_by) \
         VALUES ('A', 'practice', 'active', $1) RETURNING id",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let session_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_sessions (id, user_id, kind, assessment_id, question_plan, status, \
         affects_rating, rating_snapshot, result, finished_at) \
         VALUES ($1, $2, 'assessment', $3, $4, 'finished', false, '{}'::jsonb, $5, now())",
    )
    .bind(session_id)
    .bind(owner_id)
    .bind(assessment_id)
    .bind(json!({ "items": [{ "question_id": question_id, "version": 1 }] }))
    .bind(json!({ "points_awarded": 1, "max_points": 1, "percent": 1.0, "graded_count": 1, "pending_manual_count": 0 }))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tb_attempts (user_id, question_id, question_version, session_id, response, \
         is_correct, score, rating_before_user_avg, rating_before_question, user_tag_deltas, \
         question_delta, time_to_answer_ms) \
         VALUES ($1, $2, 1, $3, $4::jsonb, true, 1.0, 1200, 1400, '{}'::jsonb, 0, 5000)",
    )
    .bind(owner_id)
    .bind(question_id)
    .bind(session_id)
    .bind(json!({ "selected_position": 0 }))
    .execute(&pool)
    .await
    .unwrap();

    // Agent linked to the owner, with a token scoped to read the learning record.
    let agent_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, owner_user_id, display_name, role) VALUES ($1, $2, $3, 'agent')",
    )
    .bind(agent_id)
    .bind(owner_id)
    .bind("Reader Agent")
    .execute(&pool)
    .await
    .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "reader-secret-123";
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)")
        .bind(token_id)
        .bind(agent_id)
        .bind("reader-key")
        .bind(ame_api::auth::token::hash_secret(secret))
        .bind(vec!["stats.read", "attempt.read"])
        .execute(&pool)
        .await
        .unwrap();
    let agent_auth = format!("{token_id}_{secret}");

    // /v1/me/stats — agent must see the owner's aggregate, not an empty record.
    let stats: serde_json::Value = client
        .get(format!("{base_url}/v1/me/stats"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        stats["attempts_total"], 1,
        "agent should see the owner's attempt count, got {stats}"
    );

    // /v1/me/attempts — agent must see the owner's attempt history.
    let attempts: serde_json::Value = client
        .get(format!("{base_url}/v1/me/attempts"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        attempts["total"], 1,
        "agent should see the owner's attempts, got {attempts}"
    );
    assert_eq!(attempts["attempts"].as_array().unwrap().len(), 1);
}
