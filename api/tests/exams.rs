//! Exam composition and session integration tests.
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
        eprintln!("skipping exam DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return true;
    }
    false
}

async fn serve(pool: PgPool) -> String {
    let app = ame_api::http::router(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

async fn make_bearer(pool: &PgPool) -> String {
    let user_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let secret = "exam_secret_abc";
    let hash = hash_secret(secret);
    sqlx::query(
        "INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'learner')",
    )
    .bind(user_id)
    .bind(format!("exam-user-{user_id}"))
    .bind(format!("exam-{user_id}@example.com"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, 'exam token', $3, $4)",
    )
    .bind(token_id).bind(user_id).bind(hash).bind(vec!["quiz.write".to_string(), "attempt.write".to_string()])
    .execute(pool).await.unwrap();
    format!("{token_id}_{secret}")
}

async fn make_live_question(pool: &PgPool, kind: &str) -> Uuid {
    let author = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'learner')",
    )
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
        "INSERT INTO tb_questions (kind, prompt, payload, status, points, created_by) \
         VALUES ($1, $2, $3, 'live', 1, $4) RETURNING id",
    )
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
    let bearer = make_bearer(&pool).await;
    let q1 = make_live_question(&pool, "mc").await;
    let q2 = make_live_question(&pool, "tf").await;
    let base = serve(pool).await;
    let client = reqwest::Client::new();

    // compose a static exam
    let composed: Value = client
        .post(format!("{base}/v1/exams"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "name": "Integration Test Exam",
            "duration": 30,
            "sections": [{
                "title": "Section A",
                "weight": 1.0,
                "questionIds": [q1, q2]
            }]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let exam_id = composed["examId"].as_str().unwrap();
    assert_eq!(composed["totalPoints"], 2);
    assert_eq!(composed["sections"][0]["itemsCount"], 2);

    // get exam
    let got: Value = client
        .get(format!("{base}/v1/exams/{exam_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(got["exam"]["name"], "Integration Test Exam");
    assert_eq!(got["sections"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn dynamic_exam_pool_insufficient_returns_422() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let bearer = make_bearer(&pool).await;
    let base = serve(pool).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/exams"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "name": "Dynamic Exam",
            "sections": [{
                "title": "Impossible Section",
                "weight": 1.0,
                "items": 9999,
                "tags": ["__nonexistent_tag__"]
            }]
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
    let bearer = make_bearer(&pool).await;
    let base = serve(pool).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/exams"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "name": "",
            "sections": [{ "title": "S", "weight": 1.0, "questionIds": [] }]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
