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

/// Boot the router on an ephemeral port and return its base URL.
async fn spawn_app(pool: PgPool) -> String {
    let app = router(pool);
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

/// Insert an owner + an agent sub-account linked to it, and mint an agent token
/// with the given scopes. Returns `(owner_id, agent_id, "{token_id}_{secret}")`.
async fn seed_agent(pool: &PgPool, scopes: &[&str]) -> (Uuid, Uuid, String) {
    let owner_id = Uuid::now_v7();
    let owner_email = format!("loop-owner-{owner_id}@example.com");
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, $3, 'user')")
        .bind(owner_id)
        .bind(&owner_email)
        .bind("Loop Owner")
        .execute(pool)
        .await
        .unwrap();

    let agent_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_agents (id, owner_user_id, label) VALUES ($1, $2, $3)")
        .bind(agent_id)
        .bind(owner_id)
        .bind("Loop Agent")
        .execute(pool)
        .await
        .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "loop-secret-123";
    let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
    sqlx::query("INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)")
        .bind(token_id)
        .bind(agent_id)
        .bind("loop-key")
        .bind(ame_api::auth::token::hash_secret(secret))
        .bind(&scopes)
        .execute(pool)
        .await
        .unwrap();

    (owner_id, agent_id, format!("{token_id}_{secret}"))
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
        "INSERT INTO tb_agents (id, owner_user_id, label, focus_tags) VALUES ($1, $2, $3, $4)",
    )
    .bind(agent_id)
    .bind(owner_id)
    .bind("Agent Rust")
    .bind(vec![tag_name.clone()])
    .execute(&pool)
    .await
    .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "agent-secret-123";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)")
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

/// Regression: an agent reads the learning record through the single door
/// (`stats.user` / `attempt.list` via POST /v1/agents/run) and must see the
/// OWNER's data, not the agent sub-account's (which has none). The old direct
/// REST reads (`GET /v1/me/stats`, `/v1/me/attempts`) are now 403 for agents —
/// see `test_agent_token_blocked_on_learner_rest`.
#[tokio::test]
async fn test_agent_reads_owner_record_via_run() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let base_url = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

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
        "INSERT INTO tb_assessments (title, mode, status, created_by, owner_id) \
         VALUES ('A', 'practice', 'active', $1, $2) RETURNING id",
    )
    .bind(owner_id)
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
    sqlx::query("INSERT INTO tb_agents (id, owner_user_id, label) VALUES ($1, $2, $3)")
        .bind(agent_id)
        .bind(owner_id)
        .bind("Reader Agent")
        .execute(&pool)
        .await
        .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "reader-secret-123";
    sqlx::query("INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)")
        .bind(token_id)
        .bind(agent_id)
        .bind("reader-key")
        .bind(ame_api::auth::token::hash_secret(secret))
        .bind(vec!["stats.read", "attempt.read"])
        .execute(&pool)
        .await
        .unwrap();
    let agent_auth = format!("{token_id}_{secret}");

    // stats.user — agent must see the owner's aggregate, not an empty record.
    let stats = run_tool(&client, &base_url, &agent_auth, "stats.user", json!({})).await;
    assert_eq!(
        stats["result"]["attempts_total"], 1,
        "agent should see the owner's attempt count, got {stats}"
    );

    // attempt.list — agent must see the owner's attempt history.
    let attempts = run_tool(&client, &base_url, &agent_auth, "attempt.list", json!({})).await;
    assert_eq!(
        attempts["result"]["total"], 1,
        "agent should see the owner's attempts, got {attempts}"
    );
    assert_eq!(attempts["result"]["attempts"].as_array().unwrap().len(), 1);
}

/// Single-door boundary: an agent-role token is confined to POST /v1/agents/run.
/// Any direct learner/authoring REST endpoint must be rejected with 403 by the
/// agent guard middleware (task-4), regardless of the token's scopes.
#[tokio::test]
async fn test_agent_token_blocked_on_learner_rest() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let base_url = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    // A generously-scoped agent token — the 403 comes from the role guard, not
    // from a missing scope.
    let (_owner_id, _agent_id, agent_auth) = seed_agent(
        &pool,
        &[
            "assessment.read",
            "assessment.write",
            "stats.read",
            "attempt.read",
        ],
    )
    .await;
    let bearer = format!("Bearer {agent_auth}");

    // POST /v1/sessions — start a session (agents never take assessments).
    let res = client
        .post(format!("{base_url}/v1/sessions"))
        .header("Authorization", &bearer)
        .json(&json!({ "assessmentId": Uuid::now_v7() }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "agent token must be 403 on POST /v1/sessions"
    );

    // GET /v1/me/stats — learner record read is run-only for agents now.
    let res = client
        .get(format!("{base_url}/v1/me/stats"))
        .header("Authorization", &bearer)
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "agent token must be 403 on GET /v1/me/stats"
    );

    // POST /v1/assessments — authoring is run-only for agents.
    let res = client
        .post(format!("{base_url}/v1/assessments"))
        .header("Authorization", &bearer)
        .json(&json!({ "title": "Direct", "mode": "practice" }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "agent token must be 403 on POST /v1/assessments"
    );
}

/// The agent completes the full author → publish → analyze → archive loop using
/// POST /v1/agents/run exclusively (no direct REST), acting for its owner.
#[tokio::test]
async fn test_agent_full_authoring_loop_via_run() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let base_url = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let (_owner_id, _agent_id, agent_auth) = seed_agent(
        &pool,
        &[
            "assessment.read",
            "assessment.write",
            "stats.read",
            "attempt.read",
        ],
    )
    .await;

    // 1. Author a question (live so it can back a published assessment).
    let created = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "question.create",
        json!({
            "questions": [{
                "kind": "mc",
                "prompt": "2 + 2 = ?",
                "payload": { "options": ["3", "4"], "correct_index": 1 },
                "tags": ["arithmetic"],
                "status": "live",
            }]
        }),
    )
    .await;
    println!("DEBUG question.create response: {:#?}", created);
    assert_eq!(created["result"]["created"], 1);
    let question_id = created["result"]["questions"][0]["id"]
        .as_str()
        .expect("created question id")
        .to_string();

    // 2. Create the assessment as a draft.
    let assessment = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "assessment.create",
        json!({
            "title": "Agent-authored quiz",
            "mode": "practice",
            "status": "draft",
        }),
    )
    .await;
    let assessment_id = assessment["result"]["id"]
        .as_str()
        .expect("created assessment id")
        .to_string();

    // 3. Attach the question to the assessment.
    let attached = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "assessment.addQuestion",
        json!({ "id": assessment_id, "questionId": question_id }),
    )
    .await;
    assert!(
        attached["ok"].as_bool().unwrap_or(false),
        "addQuestion: {attached}"
    );

    // 4. Publish (draft -> active).
    let published = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "assessment.update",
        json!({ "id": assessment_id, "status": "active" }),
    )
    .await;
    assert!(
        published["ok"].as_bool().unwrap_or(false),
        "publish: {published}"
    );

    // 5. Analyze: read owner stats + the assessment's own stats via run.
    let stats = run_tool(&client, &base_url, &agent_auth, "stats.user", json!({})).await;
    assert!(
        stats["ok"].as_bool().unwrap_or(false),
        "stats.user: {stats}"
    );
    let a_stats = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "assessment.stats",
        json!({ "id": assessment_id }),
    )
    .await;
    assert!(
        a_stats["ok"].as_bool().unwrap_or(false),
        "assessment.stats: {a_stats}"
    );

    // 6. Archive (active -> archived) and confirm the new state via run.
    let archived = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "assessment.update",
        json!({ "id": assessment_id, "status": "archived" }),
    )
    .await;
    assert!(
        archived["ok"].as_bool().unwrap_or(false),
        "archive: {archived}"
    );

    let fetched = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "assessment.get",
        json!({ "id": assessment_id }),
    )
    .await;
    assert_eq!(
        fetched["result"]["status"], "archived",
        "assessment should be archived, got {fetched}"
    );
}

/// Regression: activity.list works after agents were moved from tb_users to tb_agents.
/// The agent performs a tool call that writes an activity log entry, then calls
/// activity.list to verify the entry appears with the agent's label.
#[tokio::test]
async fn test_activity_list_after_agent_table_migration() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let base_url = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let (_owner_id, _agent_id, agent_auth) =
        seed_agent(&pool, &["assessment.read", "assessment.write"]).await;

    // 1. Perform a tool call that writes to tb_activity_log (e.g., question.create).
    let created = run_tool(
        &client,
        &base_url,
        &agent_auth,
        "question.create",
        json!({
            "questions": [{
                "kind": "mc",
                "prompt": "Test question for activity log",
                "payload": { "options": ["a", "b"], "correct_index": 0 },
                "tags": ["activity"],
                "status": "live",
            }]
        }),
    )
    .await;
    assert!(
        created["ok"].as_bool().unwrap_or(false),
        "question.create should succeed: {created}"
    );

    // 2. Call activity.list and verify the entry appears with the agent's label.
    // The run-door writes the activity row via a fire-and-forget tokio::spawn,
    // so the feed is eventually consistent — poll briefly for the entry.
    let mut items = Vec::new();
    for _ in 0..40 {
        let activity = run_tool(&client, &base_url, &agent_auth, "activity.list", json!({})).await;
        assert!(
            activity["ok"].as_bool().unwrap_or(false),
            "activity.list should succeed: {activity}"
        );
        items = activity["result"]["items"]
            .as_array()
            .expect("activity list should have items array")
            .clone();
        if items
            .iter()
            .any(|item| item["toolName"] == "question.create")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Feed must be non-empty (regression: it returned empty when the query still
    // joined tb_users) and every entry must resolve agentName from
    // tb_agents.label (regression: it was null when joined to the deleted user).
    assert!(!items.is_empty(), "activity log should not be empty");
    assert!(
        items.iter().all(|item| item["agentName"] == "Loop Agent"),
        "every activity entry should resolve agentName to the agent's label: {items:?}"
    );

    // The run-door must record the *inner* tool name, not "POST /v1/agents/run"
    // (regression: path-based logging collapsed every run-door call to the same
    // opaque label, breaking the feed and attempt.grade stats).
    let question_create_entry = items
        .iter()
        .find(|item| item["toolName"] == "question.create")
        .expect("activity log should contain a question.create entry");
    assert_eq!(question_create_entry["agentName"], "Loop Agent");
}

/// Regression: agents must never hold the `admin` scope. POST /v1/me/agents with
/// scopes:["admin"] is rejected even for an admin owner — admin power is
/// role-gated and agents are confined to the run-door, so the scope would be a
/// dormant privilege-escalation footgun.
#[tokio::test]
async fn test_create_agent_rejects_admin_scope() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let base_url = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    // Seed an ADMIN human owner + a user token for them (admin role is the only
    // role the old code let through; it must be rejected now regardless).
    let owner_id = Uuid::now_v7();
    let owner_email = format!("admin-owner-{owner_id}@example.com");
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role) VALUES ($1, $2, $3, 'admin')",
    )
    .bind(owner_id)
    .bind(&owner_email)
    .bind("Admin Owner")
    .execute(&pool)
    .await
    .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "admin-secret-123";
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)")
        .bind(token_id)
        .bind(owner_id)
        .bind("admin-key")
        .bind(ame_api::auth::token::hash_secret(secret))
        .bind(vec!["admin".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let auth = format!("{token_id}_{secret}");

    // Even an admin owner cannot mint an agent carrying the admin scope.
    let resp = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({ "label": "rogue", "scopes": ["admin"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "agent creation with admin scope must be rejected"
    );

    // A normal authoring scope still works.
    let resp = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({ "label": "helper", "scopes": ["assessment.read"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "agent creation with a normal scope should succeed"
    );
}

/// POST a single run-tool call and return the parsed JSON envelope
/// (`{ ok, tool, result, error }`). Panics on transport failure.
async fn run_tool(
    client: &reqwest::Client,
    base_url: &str,
    agent_auth: &str,
    tool: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    client
        .post(format!("{base_url}/v1/agents/run"))
        .header("Authorization", format!("Bearer {agent_auth}"))
        .json(&json!({ "tool": tool, "params": params }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}
