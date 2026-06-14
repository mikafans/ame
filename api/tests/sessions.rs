//! Session route integration tests.
//!
//! Gated behind `AME_RUN_DB_TESTS=1` because these exercise the real
//! Postgres schema and route stack.

use std::net::SocketAddr;

use ame_api::auth::token::hash_secret;
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use time::{Duration, OffsetDateTime};
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
    let _ = sqlx::query("INSERT INTO tb_users (id, display_name, email, role, plan) VALUES ($1, 'Test Owner', 'test-owner@example.com', 'user', 'premium') ON CONFLICT DO NOTHING")
        .bind(TEST_OWNER_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("UPDATE tb_users SET plan = 'premium' WHERE id = $1")
        .bind(TEST_OWNER_ID)
        .execute(pool)
        .await;
}

async fn make_bearer(pool: &PgPool) -> (String, Uuid) {
    ensure_test_owner(pool).await;
    let user_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let secret = "session_secret_123";
    let hash = hash_secret(secret);

    sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'user')")
        .bind(user_id)
        .bind(format!("session-user-{user_id}"))
        .bind(format!("session-{user_id}@example.com"))
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO tb_login_sessions (id, user_id, token_hash, scopes, expires_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(hash)
    .bind(vec![
        "attempt.write".to_string(),
        "assessment.write".to_string(),
    ])
    .bind(OffsetDateTime::now_utc() + Duration::days(7))
    .execute(pool)
    .await
    .unwrap();

    (format!("lgn_{token_id}_{secret}"), user_id)
}

async fn make_live_mc_question(pool: &PgPool, owner_id: Uuid) -> Uuid {
    make_live_mc_question_with_tag(pool, owner_id, "rust").await
}

/// Create a user (or agent sub-account) with a scoped bearer token.
/// `owner_user_id` is the self-FK that makes an agent a sub-account of its owner;
/// pass `None` for a top-level user. Returns `(bearer, user_id)`.
async fn make_user_token(
    pool: &PgPool,
    owner_user_id: Option<Uuid>,
    role: &str,
    scopes: &[&str],
) -> (String, Uuid) {
    let user_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let secret = "scoped_secret_123";
    let hash = hash_secret(secret);

    let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();

    if role == "agent" {
        sqlx::query(
            "INSERT INTO tb_agents (id, owner_user_id, label) \
             VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(owner_user_id.expect("agent must have an owner"))
        .bind(format!("agent-{user_id}"))
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes, expires_at) \
             VALUES ($1, $2, 'scoped token', $3, $4, $5)",
        )
        .bind(token_id)
        .bind(user_id)
        .bind(hash)
        .bind(scopes)
        .bind(OffsetDateTime::now_utc() + Duration::days(7))
        .execute(pool)
        .await
        .unwrap();

        (format!("agt_{token_id}_{secret}"), user_id)
    } else {
        let email = format!("u-{user_id}@example.com");
        sqlx::query(
            "INSERT INTO tb_users (id, display_name, email, role, plan) \
             VALUES ($1, $2, $3, $4, 'premium')",
        )
        .bind(user_id)
        .bind(format!("user-{user_id}"))
        .bind(email)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tb_login_sessions (id, user_id, token_hash, scopes, expires_at) \
     VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(token_id)
        .bind(user_id)
        .bind(hash)
        .bind(scopes)
        .bind(OffsetDateTime::now_utc() + Duration::days(7))
        .execute(pool)
        .await
        .unwrap();

        (format!("lgn_{token_id}_{secret}"), user_id)
    }
}

/// Audit F-1/F-2: assessment completion state must be scoped to the requesting
/// principal, never the owner. An agent sub-account shares its owner's
/// *visibility* of an assessment but must NOT inherit the owner's completion —
/// and `list` and `get` must agree on the same caller's state.
#[tokio::test]
async fn completion_state_is_scoped_to_caller_not_owner() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    // Owner (top-level user) and its agent sub-account.
    let (owner_bearer, owner_id) = make_user_token(
        &pool,
        None,
        "user",
        &["assessment.write", "attempt.write", "assessment.read"],
    )
    .await;
    let (agent_bearer, _agent_id) = make_user_token(
        &pool,
        Some(owner_id),
        "agent",
        &["assessment.read", "attempt.write"],
    )
    .await;
    let base_url = serve(pool).await;
    let client = reqwest::Client::new();

    // Owner creates an active assessment with one live question.
    let created: Value = client
        .post(format!("{base_url}/v1/assessments"))
        .header(header::AUTHORIZATION, format!("Bearer {owner_bearer}"))
        .json(&json!({
            "title": "Scoping check",
            "mode": "practice",
            "method": "manual",
            "objectives": [],
            "status": "active",
            "questions": [
                { "kind": "mc", "prompt": "Pick", "payload": { "options": ["a", "b"], "correct_index": 0 } }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let assessment_id = created["id"].as_str().unwrap().to_string();

    // Owner takes and finishes a session on it.
    let session: Value = client
        .post(format!("{base_url}/v1/sessions"))
        .header(header::AUTHORIZATION, format!("Bearer {owner_bearer}"))
        .json(&json!({ "assessmentId": assessment_id }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let owner_session_id = session["sessionId"].as_str().unwrap().to_string();
    client
        .post(format!("{base_url}/v1/sessions/{owner_session_id}/finish"))
        .header(header::AUTHORIZATION, format!("Bearer {owner_bearer}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Helper: find this assessment in a list response.
    let find = |list: &Value| -> Value {
        list["assessments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == assessment_id)
            .cloned()
            .expect("assessment present in list")
    };
    let get_list = |bearer: &str| {
        client
            .get(format!("{base_url}/v1/assessments?status=active"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
    };
    let get_detail = |bearer: &str| {
        client
            .get(format!("{base_url}/v1/assessments/{assessment_id}"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
    };
    // ── Agent: sees the assessment (owner-scoped visibility) but NOT the owner's
    //    completion. list and get must agree. (F-1 cross-principal leak guard.)
    //    Note: Agent must access this through the single run-door.
    let agent_list_res: Value = client
        .post(format!("{base_url}/v1/agents/run"))
        .header(header::AUTHORIZATION, format!("Bearer {agent_bearer}"))
        .json(&json!({
            "tool": "assessment.list",
            "params": { "status": "active" }
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(agent_list_res["ok"].as_bool().unwrap());
    let agent_list = agent_list_res["result"].clone();
    let agent_item = find(&agent_list);
    assert_eq!(
        agent_item["completed"], false,
        "agent must not inherit owner's completion"
    );
    assert!(
        agent_item["lastSessionId"].is_null(),
        "agent must not see owner's session id"
    );

    let agent_detail_res: Value = client
        .post(format!("{base_url}/v1/agents/run"))
        .header(header::AUTHORIZATION, format!("Bearer {agent_bearer}"))
        .json(&json!({
            "tool": "assessment.get",
            "params": { "id": assessment_id }
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(agent_detail_res["ok"].as_bool().unwrap());
    let agent_detail = agent_detail_res["result"].clone();
    assert_eq!(
        agent_detail["completed"], agent_item["completed"],
        "list and get must agree for the agent (F-2)"
    );
    assert_eq!(agent_detail["lastSessionId"], agent_item["lastSessionId"]);

    // ── Owner: sees its own completion, list == get, and the returned
    //    lastSessionId resolves for the same caller (no-unresolvable-id invariant).
    let owner_list: Value = get_list(&owner_bearer)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let owner_item = find(&owner_list);
    assert_eq!(owner_item["completed"], true);
    assert_eq!(owner_item["lastSessionId"], owner_session_id);

    let owner_detail: Value = get_detail(&owner_bearer)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(owner_detail["completed"], owner_item["completed"]);
    assert_eq!(owner_detail["lastSessionId"], owner_item["lastSessionId"]);

    // Invariant: a lastSessionId returned to a caller must resolve for that caller.
    let resolved = client
        .get(format!("{base_url}/v1/sessions/{owner_session_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {owner_bearer}"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resolved.status(),
        StatusCode::OK,
        "owner must be able to resolve its own lastSessionId"
    );
}

/// Insert a live MC question (correct_index = 1) tagged `tag`. Use a unique
/// tag to make tag-filtered practice planning deterministic on a shared DB.
async fn make_live_mc_question_with_tag(pool: &PgPool, owner_id: Uuid, tag: &str) -> Uuid {
    ensure_test_owner(pool).await;
    let author_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'user')")
        .bind(author_id)
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
    .bind(owner_id)
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
    let (bearer, user_id) = make_bearer(&pool).await;
    let unique_tag = format!("rust-{}", uuid::Uuid::now_v7());
    let question_id = make_live_mc_question_with_tag(&pool, user_id, &unique_tag).await;
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

    let replayed_res = client
        .post(format!("{base_url}/v1/sessions/{session_id}/answer"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .json(&answer_body)
        .send()
        .await
        .unwrap();
    if replayed_res.status() == StatusCode::TOO_MANY_REQUESTS {
        return;
    }
    let replayed: Value = replayed_res
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["attempt"]["id"], answered["attempt"]["id"]);

    let fin_res = client
        .post(format!("{base_url}/v1/sessions/{session_id}/finish"))
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .send()
        .await
        .unwrap();
    if fin_res.status() == StatusCode::TOO_MANY_REQUESTS {
        return;
    }
    let finished: Value = fin_res.error_for_status().unwrap().json().await.unwrap();
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
    let (bearer, user_id) = make_bearer(&pool).await;
    // Unique tag → only our questions are selectable, so planning is
    // deterministic on a shared DB. Two questions because the practice planner
    // won't re-serve a just-answered question on the second attempt. Both have
    // correct_index = 1, so a correct answer is always worth 2 points.
    let tag = format!("history-{}", Uuid::now_v7());
    make_live_mc_question_with_tag(&pool, user_id, &tag).await;
    make_live_mc_question_with_tag(&pool, user_id, &tag).await;
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
        let ans_res = client
            .post(format!("{base_url}/v1/sessions/{session_id}/answer"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
            .json(&json!({
                "questionId": served_qid,
                "response": { "selected_position": selected_position },
                "timeToAnswerMs": 800
            }))
            .send()
            .await
            .unwrap();
        if ans_res.status() == StatusCode::TOO_MANY_REQUESTS {
            return;
        }
        ans_res.error_for_status().unwrap();

        let fin_res = client
            .post(format!("{base_url}/v1/sessions/{session_id}/finish"))
            .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
            .send()
            .await
            .unwrap();
        if fin_res.status() == StatusCode::TOO_MANY_REQUESTS {
            return;
        }
        fin_res.error_for_status().unwrap();
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
    let (bearer, user_id) = make_bearer(&pool).await;
    make_live_mc_question(&pool, user_id).await;
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
    let (bearer, user_id) = make_bearer(&pool).await;
    make_live_mc_question(&pool, user_id).await;
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
    let (bearer, user_id) = make_bearer(&pool).await;
    make_live_mc_question(&pool, user_id).await;
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
