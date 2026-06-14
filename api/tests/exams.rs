//! Exam composition and session integration tests.
//!
//! Gated behind `AME_RUN_DB_TESTS=1`.

use ame_api::auth::token::hash_secret;
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../db/migrations");

async fn setup_db() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connect to Postgres");
    MIGRATOR.run(&pool).await.expect("run migrations");
    pool
}

fn skip_if_no_db() -> bool {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping exam DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return true;
    }
    false
}

async fn serve(pool: PgPool) -> String {
    let app = ame_api::http::router(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap()
    });
    format!("http://{addr}")
}

static TEST_OWNER_ID: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000001");

async fn ensure_test_owner(pool: &PgPool) {
    let _ = sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, 'Test Owner', 'test-owner@example.com', 'user') ON CONFLICT DO NOTHING")
        .bind(TEST_OWNER_ID)
        .execute(pool)
        .await;
}

async fn make_bearer(pool: &PgPool) -> (String, Uuid) {
    ensure_test_owner(pool).await;
    let user_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let secret = "exam_secret_abc";
    let hash = hash_secret(secret);
    sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'user')")
        .bind(user_id)
        .bind(format!("exam-user-{user_id}"))
        .bind(format!("exam-{user_id}@example.com"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO tb_login_sessions (id, user_id, token_hash, scopes, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(hash)
    .bind(vec!["assessment.write".to_string(), "attempt.write".to_string()])
    .bind(OffsetDateTime::now_utc() + Duration::days(7))
    .execute(pool)
    .await
    .unwrap();
    (format!("lgn_{token_id}_{secret}"), user_id)
}

async fn make_live_question(pool: &PgPool, owner_id: Uuid, kind: &str) -> Uuid {
    ensure_test_owner(pool).await;
    let author = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'user')")
        .bind(author)
        .bind(format!("author-{author}"))
        .bind(format!("author-{author}@example.com"))
        .execute(pool)
        .await
        .unwrap();
    let payload = match kind {
        "mc" => json!({ "options": ["a", "b", "c"], "correct_index": 0 }),
        "tf" => json!({ "correct": true }),
        _ => json!({ "accepted": ["42"], "normalize": "exact", "judge": "exact" }),
    };
    sqlx::query(
        "INSERT INTO tb_questions (owner_id, kind, prompt, payload, status, points, created_by) \
         VALUES ($1, $2, $3, $4, 'live', 1, $5) RETURNING id",
    )
    .bind(owner_id)
    .bind(kind)
    .bind(format!("Q {kind}"))
    .bind(payload)
    .bind(author)
    .fetch_one(pool)
    .await
    .unwrap()
    .get("id")
}

#[tokio::test]
async fn static_exam_compose_get_and_session_roundtrip() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (bearer, owner_id) = make_bearer(&pool).await;
    let q1 = make_live_question(&pool, owner_id, "mc").await;
    let q2 = make_live_question(&pool, owner_id, "tf").await;
    let base = serve(pool).await;
    let client = reqwest::Client::new();

    // Create a graded assessment via the unified API (replaces old POST /v1/exams)
    let created: Value = client
        .post(format!("{base}/v1/assessments"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "title": "Integration Test Exam",
            "mode": "graded",
            "method": "manual",
            "objectives": [],
            "duration_min": 30
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let exam_id = created["id"].as_str().unwrap();
    assert!(created["id"].is_string(), "id should be present");

    // Link the two questions into the default section
    for qid in [q1, q2] {
        println!("Linking question ID: {}", qid);
        let res = client
            .post(format!("{base}/v1/assessments/{exam_id}/questions"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
            .json(&json!({ "questionId": qid }))
            .send()
            .await
            .unwrap();
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap();
            panic!("Expected success, got {status}: {body}");
        }
    }

    // get exam via the unified assessment endpoint
    let got: Value = client
        .get(format!("{base}/v1/assessments/{exam_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    // AssessmentDetail flattens the assessment fields, so title is at the top level.
    assert_eq!(got["title"], "Integration Test Exam");
    assert_eq!(got["sections"].as_array().unwrap().len(), 1);
    assert_eq!(got["sections"][0]["itemsCount"], 2);
}

/// The old dynamic-exam-pool check is now surfaced via the practice session
/// planner: requesting more questions than the bank contains returns 422
/// exam_pool_insufficient.
#[tokio::test]
async fn dynamic_exam_pool_insufficient_returns_422() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (bearer, _) = make_bearer(&pool).await;
    let base = serve(pool).await;
    let client = reqwest::Client::new();

    // Request a practice session filtered to a tag that has zero questions.
    let resp = client
        .post(format!("{base}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "tags": ["__nonexistent_tag_xyz_impossible__"],
            "count": 9999,
            "mode": "practice"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "exam_pool_insufficient");
}

#[tokio::test]
async fn compose_body_validation_rejects_empty_name() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (bearer, _) = make_bearer(&pool).await;
    let base = serve(pool).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/assessments"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "title": "",
            "mode": "graded",
            "method": "manual",
            "objectives": []
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
