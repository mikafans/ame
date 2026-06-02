//! Session route integration tests.
//!
//! Gated behind `AME_RUN_DB_TESTS=1` because these exercise the real
//! Postgres schema and route stack.

use std::net::SocketAddr;

use ame_api::auth::token::hash_secret;
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../db/migrations");

async fn setup_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("set DATABASE_URL to a reachable Postgres before AME_RUN_DB_TESTS=1");

    MIGRATOR
        .run(&pool)
        .await
        .expect("run migrations for session integration tests");

    pool
}

fn skip_if_no_db() -> bool {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "skipping session DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up"
        );
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
        .unwrap();
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

async fn make_bearer(pool: &PgPool) -> String {
    ensure_test_owner(pool).await;
    let user_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let secret = "session_secret_123";
    let hash = hash_secret(secret);

    sqlx::query("INSERT INTO tb_users (id, owner_user_id, display_name, email, role) VALUES ($1, $2, $3, $4, 'user')")
        .bind(user_id)
        .bind(TEST_OWNER_ID)
        .bind(format!("session-user-{user_id}"))
        .bind(format!("session-{user_id}@example.com"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) \
         VALUES ($1, $2, 'session token', $3, $4)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(hash)
    .bind(vec![
        "attempt.write".to_string(),
        "assessment.write".to_string(),
    ])
    .execute(pool)
    .await
    .unwrap();

    format!("{token_id}_{secret}")
}

async fn make_live_mc_question(pool: &PgPool) -> Uuid {
    make_live_mc_question_with_tag(pool, "rust").await
}

/// Insert a live MC question (correct_index = 1) tagged `tag`. Use a unique
/// tag to make tag-filtered practice planning deterministic on a shared DB.
async fn make_live_mc_question_with_tag(pool: &PgPool, tag: &str) -> Uuid {
    ensure_test_owner(pool).await;
    let author_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, owner_user_id, display_name, email, role) VALUES ($1, $2, $3, $4, 'user')")
        .bind(author_id)
        .bind(TEST_OWNER_ID)
        .bind(format!("author-{author_id}"))
        .bind(format!("author-{author_id}@example.com"))
        .execute(pool)
        .await
        .unwrap();

    let question_id: Uuid = sqlx::query(
        "INSERT INTO tb_questions (owner_id, kind, prompt, payload, status, points, created_by) \
         VALUES ($1, 'mc', 'What color?', $2, 'live', 2, $3) \
          RETURNING id",
    )
    .bind(TEST_OWNER_ID)
    .bind(json!({ "options": ["red", "green", "blue"], "correct_index": 1 }))
    .bind(author_id)
    .fetch_one(pool)
    .await
    .unwrap()
    .get("id");
    let tag_id: Uuid = sqlx::query(
        "INSERT INTO tb_tags (name) VALUES ($1) \
         ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name \
         RETURNING id",
    )
    .bind(tag)
    .fetch_one(pool)
    .await
    .unwrap()
    .get("id");
    sqlx::query(
        "INSERT INTO tb_question_tags (question_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(question_id)
    .bind(tag_id)
    .execute(pool)
    .await
    .unwrap();

    question_id
}

#[tokio::test]
async fn practice_session_answer_replay_and_finish_roundtrip() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    let bearer = make_bearer(&pool).await;
    let unique_tag = format!("rust-{}", uuid::Uuid::now_v7());
    let question_id = make_live_mc_question_with_tag(&pool, &unique_tag).await;
    let base_url = serve(pool).await;
    let client = reqwest::Client::new();

    let created: Value = client
        .post(format!("{base_url}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "tags": [unique_tag],
            "types": ["mc"],
            "count": 1,
            "duration": 30,
            "mode": "practice"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let session_id = created["sessionId"].as_str().unwrap();
    assert_eq!(created["questions"][0]["id"], question_id.to_string());
    let selected_position = created["questions"][0]["option_order"]
        .as_array()
        .unwrap()
        .iter()
        .position(|value| value.as_u64() == Some(1))
        .unwrap();

    let answer_body = json!({
        "questionId": question_id,
        "response": { "selected_position": selected_position },
        "timeToAnswerMs": 800
    });
    let answered: Value = client
        .post(format!("{base_url}/v1/sessions/{session_id}/answer"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&answer_body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(answered["grade"]["correct"], true);
    assert_eq!(answered["grade"]["points_awarded"], 2);
    assert_eq!(answered["replayed"], false);

    let replayed: Value = client
        .post(format!("{base_url}/v1/sessions/{session_id}/answer"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&answer_body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["attempt"]["id"], answered["attempt"]["id"]);

    let finished: Value = client
        .post(format!("{base_url}/v1/sessions/{session_id}/finish"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(finished["session"]["status"], "finished");
    assert_eq!(finished["result"]["points_awarded"], 2);
    assert_eq!(finished["result"]["max_points"], 2);

    let late = client
        .post(format!("{base_url}/v1/sessions/{session_id}/answer"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&answer_body)
        .send()
        .await
        .unwrap();
    assert_eq!(late.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn list_my_sessions_numbers_finished_attempts_newest_first() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    let bearer = make_bearer(&pool).await;
    // Unique tag → only our questions are selectable, so planning is
    // deterministic on a shared DB. Two questions because the practice planner
    // won't re-serve a just-answered question on the second attempt. Both have
    // correct_index = 1, so a correct answer is always worth 2 points.
    let tag = format!("history-{}", Uuid::now_v7());
    make_live_mc_question_with_tag(&pool, &tag).await;
    make_live_mc_question_with_tag(&pool, &tag).await;
    let base_url = serve(pool).await;
    let client = reqwest::Client::new();

    // Run two full practice attempts (start → answer → finish).
    for _ in 0..2 {
        let created: Value = client
            .post(format!("{base_url}/v1/sessions"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
            .json(&json!({
                "tags": [tag],
                "types": ["mc"],
                "count": 1,
                "duration": 30,
                "mode": "practice"
            }))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        let session_id = created["sessionId"].as_str().unwrap();
        // Answer whichever question was actually served.
        let served_qid = created["questions"][0]["id"].as_str().unwrap();
        let selected_position = created["questions"][0]["option_order"]
            .as_array()
            .unwrap()
            .iter()
            .position(|value| value.as_u64() == Some(1))
            .unwrap();
        client
            .post(format!("{base_url}/v1/sessions/{session_id}/answer"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
            .json(&json!({
                "questionId": served_qid,
                "response": { "selected_position": selected_position },
                "timeToAnswerMs": 800
            }))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        client
            .post(format!("{base_url}/v1/sessions/{session_id}/finish"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    let history: Value = client
        .get(format!("{base_url}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let sessions = history["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 2, "two finished attempts expected");
    // Newest first, but attempt numbers are chronological.
    assert_eq!(sessions[0]["attemptNumber"], 2);
    assert_eq!(sessions[0]["totalAttempts"], 2);
    assert_eq!(sessions[1]["attemptNumber"], 1);
    assert_eq!(sessions[1]["totalAttempts"], 2);
    // Score is surfaced from the stored result.
    assert_eq!(sessions[0]["pointsAwarded"], 2.0);
    assert_eq!(sessions[0]["maxPoints"], 2.0);
}

#[tokio::test]
async fn list_my_sessions_excludes_in_progress() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    let bearer = make_bearer(&pool).await;
    make_live_mc_question(&pool).await;
    let base_url = serve(pool).await;
    let client = reqwest::Client::new();

    // Start a session but never finish it.
    client
        .post(format!("{base_url}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({ "count": 1, "mode": "practice" }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let history: Value = client
        .get(format!("{base_url}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(
        history["sessions"].as_array().unwrap().len(),
        0,
        "in-progress sessions must not appear in history"
    );
}

#[tokio::test]
async fn abandon_session_roundtrip() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    let bearer = make_bearer(&pool).await;
    make_live_mc_question(&pool).await;
    let base_url = serve(pool).await;
    let client = reqwest::Client::new();

    let created: Value = client
        .post(format!("{base_url}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "count": 1,
            "mode": "practice"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let session_id = created["sessionId"].as_str().unwrap();

    let patched: Value = client
        .patch(format!("{base_url}/v1/sessions/{session_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({ "status": "abandoned" }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(patched["status"], "abandoned");

    let revive = client
        .patch(format!("{base_url}/v1/sessions/{session_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({ "status": "in_progress" }))
        .send()
        .await
        .unwrap();
    assert_eq!(revive.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn patch_session_rejects_finished_target() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    let bearer = make_bearer(&pool).await;
    make_live_mc_question(&pool).await;
    let base_url = serve(pool).await;
    let client = reqwest::Client::new();

    let created: Value = client
        .post(format!("{base_url}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({
            "count": 1,
            "mode": "practice"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let session_id = created["sessionId"].as_str().unwrap();

    let finished = client
        .patch(format!("{base_url}/v1/sessions/{session_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&json!({ "status": "finished" }))
        .send()
        .await
        .unwrap();
    assert_eq!(finished.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
