use ame_api::http::router;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use sqlx::Row;
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
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
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

    // 2. Agent creates a private assessment
    let token_id = Uuid::now_v7();
    let secret = "agent-secret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(token_id)
        .bind(agent_id)
        .bind("agent-key")
        .bind(&hash)
        .bind(vec![
            "assessment.read".to_string(),
            "assessment.write".to_string(),
            "attempt.write".to_string(),
        ])
        .execute(&pool)
        .await
        .unwrap();
    let agent_auth = format!("{token_id}_{secret}");

    let res = client
        .post(format!("{base_url}/v1/assessments"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({
            "title": "Agent Assessment",
            "mode": "graded",
            "method": "agent",
            "objectives": []
        }))
        .send()
        .await
        .unwrap();
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap();
        panic!("Assessment creation failed ({status}): {body}");
    }
    let json: serde_json::Value = res.json().await.unwrap();
    let qid = json["id"].as_str().unwrap().to_string();
    let assessment_uuid: Uuid = qid.parse().unwrap();

    // A session can only be created on an active assessment with at least one live
    // question (see build_assessment_plan), so give it one and activate it. It stays
    // private — the access checks below are what we're exercising.
    let question_id: Uuid = sqlx::query(
        "INSERT INTO tb_questions (kind, prompt, payload, status, points, created_by) \
         VALUES ('mc', 'What color?', $1, 'live', 2, $2) RETURNING id",
    )
    .bind(json!({ "options": ["red", "green", "blue"], "correct_index": 1 }))
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap()
    .get("id");
    // create_assessment auto-creates a default section; reuse it.
    let section_id: Uuid =
        sqlx::query_scalar("SELECT id FROM tb_assessment_sections WHERE assessment_id = $1 ORDER BY order_index LIMIT 1")
            .bind(assessment_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query(
        "INSERT INTO tb_assessment_items (section_id, question_id, order_index) VALUES ($1, $2, 0)",
    )
    .bind(section_id)
    .bind(question_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE tb_assessment_sections SET items_count = 1 WHERE id = $1")
        .bind(section_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tb_assessments SET status = 'active' WHERE id = $1")
        .bind(assessment_uuid)
        .execute(&pool)
        .await
        .unwrap();

    // 3. Test: Owner GET agent's private assessment should succeed
    let owner_token_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(owner_token_id)
        .bind(owner_id)
        .bind("owner-key")
        .bind(&hash) // Same hash for simplicity
        .bind(vec!["assessment.read".to_string(), "attempt.write".to_string()])
        .execute(&pool)
        .await
        .unwrap();

    let owner_auth = format!("{owner_token_id}_{secret}");

    let res = client
        .get(format!("{base_url}/v1/assessments/{qid}"))
        .header("Authorization", format!("Bearer {owner_auth}"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 4. Test: Unrelated user GET agent's private assessment should 404
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
        .bind(vec!["assessment.read".to_string(), "attempt.write".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let stranger_auth = format!("{stranger_token_id}_{secret}");

    let res = client
        .get(format!("{base_url}/v1/assessments/{qid}"))
        .header("Authorization", format!("Bearer {stranger_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 5. Test: Agent creates a session for its own private assessment
    let res = client
        .post(format!("{base_url}/v1/sessions"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({"assessmentId": qid}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 6. Test: Owner creates a session for agent's private assessment
    let res = client
        .post(format!("{base_url}/v1/sessions"))
        .header("Authorization", format!("Bearer {owner_auth}"))
        .json(&json!({"assessmentId": qid}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 7. Test: Stranger creates a session for private assessment -> 404
    let res = client
        .post(format!("{base_url}/v1/sessions"))
        .header("Authorization", format!("Bearer {stranger_auth}"))
        .json(&json!({"assessmentId": qid}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// Regression: the list/count endpoints must be strictly owner-scoped. A user
/// from a different owner tree must never see another owner's assessments or
/// questions — not even a `public` + `active` assessment (which the old
/// `count_assessments` query leaked, and the old `list_assessments` /
/// `list_questions` queries surfaced).
#[tokio::test]
async fn test_cross_owner_list_isolation() {
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
    let secret = "iso-secret";
    let hash = ame_api::auth::token::hash_secret(secret);

    // Owner A with a write-capable token.
    let owner_a = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, 'OwnerA', 'user')",
    )
    .bind(owner_a)
    .bind(format!("owner-a-{}@example.com", owner_a))
    .execute(&pool)
    .await
    .unwrap();
    let a_token = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(a_token)
        .bind(owner_a)
        .bind("a-key")
        .bind(&hash)
        .bind(vec!["assessment.read".to_string(), "assessment.write".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let a_auth = format!("{a_token}_{secret}");

    // A creates an assessment, then we force it active — the exact shape
    // the old count query leaked across owners.
    let res = client
        .post(format!("{base_url}/v1/assessments"))
        .header("Authorization", format!("Bearer {a_auth}"))
        .json(&json!({
            "title": "Owner A Assessment",
            "mode": "graded",
            "method": "agent",
            "objectives": []
        }))
        .send()
        .await
        .unwrap();
    assert!(
        res.status().is_success(),
        "owner A could not create assessment"
    );
    let body: serde_json::Value = res.json().await.unwrap();
    let a_assessment_id = body["id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE tb_assessments SET status = 'active' WHERE id = $1")
        .bind(Uuid::parse_str(&a_assessment_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // A owns a question.
    let a_question_id: Uuid = sqlx::query(
        "INSERT INTO tb_questions (kind, prompt, payload, status, points, created_by) \
         VALUES ('mc', 'A private question', $1, 'live', 1, $2) RETURNING id",
    )
    .bind(json!({ "options": ["x", "y"], "correct_index": 0 }))
    .bind(owner_a)
    .fetch_one(&pool)
    .await
    .unwrap()
    .get("id");

    // Stranger B: a separate top-level owner with read scope only.
    let owner_b = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, 'OwnerB', 'user')",
    )
    .bind(owner_b)
    .bind(format!("owner-b-{}@example.com", owner_b))
    .execute(&pool)
    .await
    .unwrap();
    let b_token = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(b_token)
        .bind(owner_b)
        .bind("b-key")
        .bind(&hash)
        .bind(vec!["assessment.read".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let b_auth = format!("{b_token}_{secret}");

    // B's assessment list must not include A's public assessment.
    let res = client
        .get(format!("{base_url}/v1/assessments"))
        .header("Authorization", format!("Bearer {b_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let leaked = body["assessments"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"].as_str() == Some(a_assessment_id.as_str()));
    assert!(!leaked, "stranger saw owner A's assessment in the list");

    // B's count must not include A's public + active assessment.
    let res = client
        .get(format!("{base_url}/v1/assessments/count"))
        .header("Authorization", format!("Bearer {b_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(
        body["count"].as_i64().unwrap(),
        0,
        "count leaked cross-owner public assessments"
    );

    // B's question list must not include A's question.
    let res = client
        .get(format!("{base_url}/v1/questions"))
        .header("Authorization", format!("Bearer {b_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let leaked_q = body["questions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|q| q["id"].as_str() == Some(a_question_id.to_string().as_str()));
    assert!(!leaked_q, "stranger saw owner A's question in the list");
}
