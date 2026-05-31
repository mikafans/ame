//! Share API and MCP manifest integration tests.
//!
//! Gated behind `AME_RUN_DB_TESTS=1`.

use ame_api::auth::token::hash_secret;
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
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
        eprintln!("skipping shares DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
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

async fn make_user_with_scopes(pool: &PgPool, scopes: &[&str]) -> (Uuid, String) {
    let user_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let secret = format!("secret_{}", token_id.simple());
    let hash = hash_secret(&secret);
    sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'user')")
        .bind(user_id)
        .bind(format!("test-user-{user_id}"))
        .bind(format!("share-{user_id}@example.com"))
        .execute(pool)
        .await
        .unwrap();
    let scopes_vec: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind("test-token")
    .bind(&hash)
    .bind(&scopes_vec)
    .execute(pool)
    .await
    .unwrap();
    (user_id, format!("{token_id}_{secret}"))
}

/// Serve with a lazy pool — does not connect to real DB, only usable for
/// endpoints that don't touch the database (e.g. MCP manifest).
async fn serve_lazy() -> String {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .unwrap();
    serve(pool).await
}

// ── MCP manifest ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn skill_manifest_contains_all_share_tools() {
    let base = serve_lazy().await;
    let client = reqwest::Client::new();

    let manifest: Value = client
        .get(format!("{base}/v1/agents/skill.json"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let tools = manifest["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Tools exposed in the unified assessment manifest (post agent-identity refactor)
    let all_expected = [
        "assessment.list",
        "assessment.get",
        "assessment.create",
        "assessment.update",
        "assessment.delete",
        "question.list",
        "question.create",
        "session.create",
        "session.finish",
    ];
    for expected in &all_expected {
        assert!(
            tool_names.contains(expected),
            "skill manifest missing tool: {expected}. Got: {tool_names:?}"
        );
    }
}

#[tokio::test]
async fn skill_manifest_strict_drops_ame_fields() {
    let base = serve_lazy().await;
    let client = reqwest::Client::new();

    let manifest: Value = client
        .get(format!("{base}/v1/agents/skill.json?strict=1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let tools = manifest["tools"].as_array().unwrap();
    for tool in tools {
        assert!(
            tool.get("method").is_none(),
            "strict manifest should not have 'method'"
        );
        assert!(
            tool.get("path").is_none(),
            "strict manifest should not have 'path'"
        );
        assert!(
            tool.get("scope").is_none(),
            "strict manifest should not have 'scope'"
        );
    }
}

// ── Share golden path ─────────────────────────────────────────────────────────

#[tokio::test]
async fn share_golden_path_create_get_revoke_404() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (_user_id, token) = make_user_with_scopes(&pool, &["quiz.read"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();

    let target_id = Uuid::now_v7();
    let bearer = format!("Bearer {token}");

    // 1. Create share
    let created: Value = client
        .post(format!("{base}/v1/shares"))
        .header(header::AUTHORIZATION, &bearer)
        .json(&json!({
            "kind": "quiz",
            "id": target_id,
            "visibility": "public"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let share_id = created["shareId"].as_str().unwrap();
    assert!(!share_id.is_empty());

    // 2. GET (public — no auth)
    let resp = client
        .get(format!("{base}/v1/shares/{share_id}"))
        .send()
        .await
        .unwrap();

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap();
        panic!("GET share failed ({status}): {body}");
    }

    let view: Value = resp.json().await.unwrap();

    assert_eq!(view["kind"].as_str().unwrap(), "quiz");
    assert_eq!(view["targetId"].as_str().unwrap(), target_id.to_string());

    // 3. Revoke
    let revoke_status = client
        .delete(format!("{base}/v1/shares/{share_id}"))
        .header(header::AUTHORIZATION, &bearer)
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(revoke_status, StatusCode::NO_CONTENT);

    // 4. 404 on next GET
    let status = client
        .get(format!("{base}/v1/shares/{share_id}"))
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── Idempotency ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn share_tuple_dedup_returns_same_share_id() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (_user_id, token) = make_user_with_scopes(&pool, &["quiz.read"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();
    let bearer = format!("Bearer {token}");
    let target_id = Uuid::now_v7();
    let body = json!({ "kind": "exam", "id": target_id, "visibility": "public" });

    let r1: Value = client
        .post(format!("{base}/v1/shares"))
        .header(header::AUTHORIZATION, &bearer)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let r2: Value = client
        .post(format!("{base}/v1/shares"))
        .header(header::AUTHORIZATION, &bearer)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(
        r1["shareId"].as_str().unwrap(),
        r2["shareId"].as_str().unwrap(),
        "same natural-tuple → same shareId"
    );
}

// ── Embed route precedence ────────────────────────────────────────────────────

#[tokio::test]
async fn embed_route_resolves_without_auth() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (_user_id, token) = make_user_with_scopes(&pool, &["quiz.read"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();
    let bearer = format!("Bearer {token}");
    let target_id = Uuid::now_v7();

    // Create a share first
    client
        .post(format!("{base}/v1/shares"))
        .header(header::AUTHORIZATION, &bearer)
        .json(&json!({ "kind": "quiz", "id": target_id }))
        .send()
        .await
        .unwrap();

    // GET /v1/quizzes/{id}/embed — no auth, should hit embed handler not quiz handler
    let resp = client
        .get(format!("{base}/v1/quizzes/{target_id}/embed"))
        .send()
        .await
        .unwrap();

    // Should be 200 (embed handler found the share) not 401/403 (quiz handler would require auth)
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "embed route should be public"
    );
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["kind"].as_str().unwrap(), "quiz");
}

// ── Revoke by non-owner ───────────────────────────────────────────────────────

#[tokio::test]
async fn share_revoke_rejected_for_non_owner() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let (_uid1, token1) = make_user_with_scopes(&pool, &["quiz.read"]).await;
    let (_uid2, token2) = make_user_with_scopes(&pool, &["quiz.read"]).await;
    let base = serve(pool.clone()).await;
    let client = reqwest::Client::new();
    let target_id = Uuid::now_v7();

    // User1 creates the share
    let created: Value = client
        .post(format!("{base}/v1/shares"))
        .header(header::AUTHORIZATION, format!("Bearer {token1}"))
        .json(&json!({ "kind": "item", "id": target_id }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let share_id = created["shareId"].as_str().unwrap();

    // User2 tries to revoke — should fail
    let status = client
        .delete(format!("{base}/v1/shares/{share_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token2}"))
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::FORBIDDEN);
}
