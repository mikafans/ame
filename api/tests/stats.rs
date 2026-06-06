//! Stats, messages, keys, and admin integration tests.
//!
//! Gated behind `AME_RUN_DB_TESTS=1`.

use ame_api::auth::token::hash_secret;
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
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
        eprintln!("skipping stats DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
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

/// Create a user + token with given scopes. Returns (user_id, bearer_string).
async fn make_user_with_scopes(pool: &PgPool, scopes: &[&str]) -> (Uuid, String) {
    let _ = sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, 'Test Owner', 'test-owner@example.com', 'user') ON CONFLICT DO NOTHING")
        .bind(TEST_OWNER_ID)
        .execute(pool)
        .await;

    let user_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let secret = format!("secret_{}", token_id.simple());
    let hash = hash_secret(&secret);
    sqlx::query("INSERT INTO tb_users (id, owner_user_id, display_name, email, role) VALUES ($1, $2, $3, $4, 'user')")
        .bind(user_id)
        .bind(TEST_OWNER_ID)
        .bind(format!("test-user-{user_id}"))
        .bind(format!("stats-{user_id}@example.com"))
        .execute(pool)
        .await
        .unwrap();
    let scopes_vec: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind("test token")
    .bind(&hash)
    .bind(&scopes_vec)
    .execute(pool)
    .await
    .unwrap();
    (user_id, format!("{token_id}_{secret}"))
}

async fn make_live_question(pool: &PgPool, kind: &str, author: Uuid) -> Uuid {
    let payload = match kind {
        "mc" => json!({ "options": ["a", "b", "c"], "correct_index": 0 }),
        "tf" => json!({ "correct": true }),
        _ => json!({ "accepted": ["42"], "normalize": "exact", "judge": "exact" }),
    };
    sqlx::query(
        "INSERT INTO tb_questions (owner_id, kind, prompt, payload, status, points, created_by) \
         VALUES ($1, $2, $3, $4, 'live', 1, $5) RETURNING id",
    )
    .bind(TEST_OWNER_ID)
    .bind(kind)
    .bind(format!("Q {kind}"))
    .bind(payload)
    .bind(author)
    .fetch_one(pool)
    .await
    .unwrap()
    .get("id")
}

async fn make_assessment(pool: &PgPool, created_by: Uuid) -> Uuid {
    // Resolve the owner: if created_by is a sub-account, use their owner; else use created_by
    let owner_id: Uuid =
        sqlx::query_scalar("SELECT COALESCE(owner_user_id, id) FROM tb_users WHERE id = $1")
            .bind(created_by)
            .fetch_one(pool)
            .await
            .unwrap();

    sqlx::query(
        "INSERT INTO tb_assessments (title, mode, status, created_by, owner_id) VALUES ($1, 'practice', 'active', $2, $3) RETURNING id",
    )
    .bind("Test Assessment")
    .bind(created_by)
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .unwrap()
    .get("id")
}

async fn make_finished_session_for_assessment(
    pool: &PgPool,
    user_id: Uuid,
    assessment_id: Uuid,
    question_id: Uuid,
    is_correct: bool,
) {
    let session_id = Uuid::now_v7();
    let question_plan = json!({
        "items": [{ "question_id": question_id, "version": 1 }]
    });
    let result = json!({
        "points_awarded": if is_correct { 1 } else { 0 },
        "max_points": 1,
        "percent": if is_correct { 1.0 } else { 0.0 },
        "graded_count": 1,
        "pending_manual_count": 0
    });
    sqlx::query(
        "INSERT INTO tb_sessions (id, user_id, kind, assessment_id, question_plan, status, affects_rating, \
         rating_snapshot, result, finished_at) \
         VALUES ($1, $2, 'assessment', $3, $4, 'finished', false, '{}'::jsonb, $5, now())",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(assessment_id)
    .bind(question_plan)
    .bind(result)
    .execute(pool)
    .await
    .unwrap();

    // insert attempt
    sqlx::query(
        "INSERT INTO tb_attempts (user_id, question_id, question_version, session_id, response, \
         is_correct, score, rating_before_user_avg, rating_before_question, user_tag_deltas, \
         question_delta, time_to_answer_ms) \
         VALUES ($1, $2, 1, $3, $4::jsonb, $5, $6, 1200, 1400, '{}'::jsonb, 0, 5000)",
    )
    .bind(user_id)
    .bind(question_id)
    .bind(session_id)
    .bind(json!({ "selected_position": 0 }))
    .bind(is_correct)
    .bind(if is_correct { 1.0f64 } else { 0.0f64 })
    .execute(pool)
    .await
    .unwrap();
}

// ── Task 1 & 6: assessment stats shape ────────────────────────────────────────────

#[tokio::test]
async fn assessment_stats_returns_correct_shape() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (user_id, bearer) = make_user_with_scopes(&pool, &["stats.read"]).await;
    let q = make_live_question(&pool, "mc", user_id).await;
    let assessment_id = make_assessment(&pool, user_id).await;

    // two sessions: one correct, one incorrect
    let (u2, _) = make_user_with_scopes(&pool, &["stats.read"]).await;
    make_finished_session_for_assessment(&pool, user_id, assessment_id, q, true).await;
    make_finished_session_for_assessment(&pool, u2, assessment_id, q, false).await;

    let base = serve(pool).await;
    let client = reqwest::Client::new();

    let resp: Value = client
        .get(format!("{base}/v1/assessments/{assessment_id}/stats"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp["avg"].is_number(), "avg should be a number");
    assert!(resp["median"].is_number(), "median should be a number");
    assert!(
        resp["distribution"].is_array(),
        "distribution should be array"
    );
    assert!(resp["items"].is_array(), "items should be array");

    let items = resp["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "one question in the assessment");
    let item = &items[0];
    assert!(item["questionId"].is_string());
    assert!(item["correctRate"].is_number());
    assert!(item["avgTimeMs"].is_number());
    // discriminationIdx may be null if variance is zero
    assert!(!item["discriminationIdx"].is_object());
}

// ── Task 2 & 6: exam stats shape ────────────────────────────────────────────

#[tokio::test]
async fn exam_stats_returns_correct_shape() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (user_id, bearer) = make_user_with_scopes(&pool, &["stats.read", "assessment.write"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();

    // compose exam via the unified assessment API (mode: graded, method: manual)
    let q1 = make_live_question(&pool, "mc", user_id).await;
    let composed: Value = client
        .post(format!("{base}/v1/assessments"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "title": "Stats Test Exam",
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
    let exam_id = composed["id"].as_str().unwrap();

    // Link question into the assessment's default section
    let section_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM tb_assessment_sections WHERE assessment_id = $1 ORDER BY order_index LIMIT 1",
    )
    .bind(Uuid::parse_str(exam_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tb_assessment_items (section_id, question_id, order_index) VALUES ($1, $2, 0)",
    )
    .bind(section_id)
    .bind(q1)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE tb_assessment_sections SET items_count = 1 WHERE id = $1")
        .bind(section_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tb_assessments SET status = 'active' WHERE id = $1")
        .bind(Uuid::parse_str(exam_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let resp: Value = client
        .get(format!("{base}/v1/assessments/{exam_id}/stats"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    // The unified assessment stats endpoint returns avg/median/distribution/items.
    assert!(resp["avg"].is_number(), "avg should be a number");
    assert!(resp["median"].is_number(), "median should be a number");
    assert!(
        resp["distribution"].is_array(),
        "distribution should be array"
    );
    assert!(resp["items"].is_array(), "items should be array");
}

// ── Task 6: scope enforcement ────────────────────────────────────────────────

#[tokio::test]
async fn stats_endpoints_require_stats_read_scope() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    // token can create/read source objects, but intentionally lacks stats.read.
    let (user_id, bearer) = make_user_with_scopes(
        &pool,
        &["assessment.read", "assessment.write", "attempt.write"],
    )
    .await;
    let assessment_id = make_assessment(&pool, user_id).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();

    // compose a second assessment (graded/exam-mode) via the unified API
    let _q = make_live_question(&pool, "mc", user_id).await;
    let composed: Value = client
        .post(format!("{base}/v1/assessments"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "title": "Scope Test Exam",
            "mode": "graded",
            "method": "manual",
            "objectives": []
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let exam_assessment_id = composed["id"].as_str().unwrap();

    // Both assessments must return 403 when caller lacks stats.read
    let assessment_resp = client
        .get(format!("{base}/v1/assessments/{assessment_id}/stats"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap();
    assert_eq!(assessment_resp.status(), StatusCode::FORBIDDEN);

    let exam_resp = client
        .get(format!("{base}/v1/assessments/{exam_assessment_id}/stats"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap();
    assert_eq!(exam_resp.status(), StatusCode::FORBIDDEN);
}

// ── Task 3 & 6: messages ────────────────────────────────────────────────────

#[tokio::test]
async fn post_message_in_app_creates_record() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (from_id, bearer) = make_user_with_scopes(&pool, &["feedback.write"]).await;
    let (to_id, _) = make_user_with_scopes(&pool, &["assessment.read"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();

    let resp: Value = client
        .post(format!("{base}/v1/messages"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "userId": to_id,
            "channel": "in_app",
            "body": "Great work this week!"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp["messageId"].is_string());
    assert_eq!(resp["status"], "queued");

    // verify stored
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tb_messages WHERE from_user_id = $1 AND to_user_id = $2",
    )
    .bind(from_id)
    .bind(to_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn post_message_email_channel_queues_without_sending() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (from_id, bearer) = make_user_with_scopes(&pool, &["feedback.write"]).await;
    let (to_id, _) = make_user_with_scopes(&pool, &["assessment.read"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/messages"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "userId": to_id,
            "channel": "email",
            "body": "Check your assessment results."
        }))
        .send()
        .await
        .unwrap();

    // must accept the request (not 422)
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "queued", "email channel stays queued");

    // stored in DB
    let row: (String,) = sqlx::query_as(
        "SELECT status FROM tb_messages WHERE from_user_id = $1 AND to_user_id = $2",
    )
    .bind(from_id)
    .bind(to_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, "queued");
}

// ── Task 4 & 6: key management ──────────────────────────────────────────────

#[tokio::test]
async fn key_rotation_invalidates_old_token() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (user_id, bearer) = make_user_with_scopes(&pool, &["assessment.read"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();

    // list keys — should include current key
    let keys: Value = client
        .get(format!("{base}/v1/me/keys"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let key_id = keys["keys"].as_array().unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // rotate
    let rotated: Value = client
        .post(format!("{base}/v1/me/keys/{key_id}/rotate"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        rotated["secret"].is_string(),
        "rotate should return new key"
    );

    // old bearer must now be rejected
    let protected = client
        .get(format!("{base}/v1/me/keys"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        protected.status(),
        StatusCode::UNAUTHORIZED,
        "old token must be revoked after rotation"
    );
    let _ = user_id; // bind to avoid warning
}

#[tokio::test]
async fn create_key_requires_admin_scope_for_admin_key() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    // non-admin user without admin scope
    let (_, bearer) = make_user_with_scopes(&pool, &["assessment.read"]).await;
    let base = serve(pool).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/me/keys"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({ "name": "admin key attempt", "scopes": ["admin"] }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "non-admin must not create admin-scoped keys"
    );
}
