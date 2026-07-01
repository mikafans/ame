//! Tests for `fn_prune_stale_data`. The function prunes table-wide, so a live
//! prune in one test deletes stale rows seeded by another. These tests MUST run
//! single-threaded — `make test-db` invokes this binary with `--test-threads=1`.

use sqlx::PgPool;
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
        panic!("failed to run migrations: {error}");
    }

    pool
}

async fn seed_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    let email = format!("user-{}@example.com", id);
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(email)
    .bind("Test User")
    .bind("user")
    .bind("free")
    .execute(pool)
    .await
    .expect("seed_user failed");
    id
}

async fn seed_agent(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_agents (id, owner_user_id, label) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(owner_id)
        .bind("test agent")
        .execute(pool)
        .await
        .expect("seed_agent failed");
    id
}

async fn seed_api_token(pool: &PgPool, agent_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes, expires_at) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(id)
    .bind(agent_id)
    .bind("test token")
    .bind("hash")
    .bind(vec!["assessment.read"])
    .bind(time::OffsetDateTime::now_utc() + time::Duration::days(30))
    .execute(pool)
    .await
    .expect("seed_api_token failed");
    id
}

async fn seed_webhook(pool: &PgPool, user_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_webhooks (id, user_id, url, events, secret_hash) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(id)
    .bind(user_id)
    .bind("https://example.com/webhook")
    .bind(vec!["attempt.submitted"])
    .bind("hash")
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Test dry-run is read-only: would_delete > 0 but deleted = 0 and rows persist.
#[tokio::test]
async fn test_dry_run_read_only() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    let user_id = seed_user(&pool).await;
    let agent_id = seed_agent(&pool, user_id).await;
    let token_id = seed_api_token(&pool, agent_id).await;

    // Seed a stale idempotency key (49 hours old)
    let stale_time = time::OffsetDateTime::now_utc() - time::Duration::hours(49);
    sqlx::query(
        "INSERT INTO tb_idempotency_keys (token_id, key, request_hash, response_status, response_body, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(token_id)
    .bind("idempotency-stale")
    .bind("hash")
    .bind(200i16)
    .bind(serde_json::json!({}))
    .bind(stale_time)
    .execute(&pool)
    .await
    .unwrap();

    // Run dry-run prune
    let result: Vec<(String, i64, i64)> =
        sqlx::query_as("SELECT table_name, would_delete, deleted FROM fn_prune_stale_data(true)")
            .fetch_all(&pool)
            .await
            .unwrap();

    let idempotency_result = result
        .iter()
        .find(|(t, _, _)| t == "tb_idempotency_keys")
        .unwrap();
    assert!(
        idempotency_result.1 > 0,
        "would_delete should be > 0 for stale idempotency keys"
    );
    assert_eq!(idempotency_result.2, 0, "deleted should be 0 in dry-run");

    // Verify stale row still exists after dry-run
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tb_idempotency_keys WHERE token_id = $1")
            .bind(token_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 1,
        "stale idempotency key should still exist after dry-run"
    );
}

/// Test live deletes only the stale side for each table.
#[tokio::test]
async fn test_live_deletes_only_stale() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    let user_id = seed_user(&pool).await;
    let agent_id = seed_agent(&pool, user_id).await;
    let token_id = seed_api_token(&pool, agent_id).await;
    let webhook_id = seed_webhook(&pool, user_id).await;

    // ─── tb_idempotency_keys: stale (49h old) and fresh (1h old) ────────────────
    let stale_time = time::OffsetDateTime::now_utc() - time::Duration::hours(49);
    let fresh_time = time::OffsetDateTime::now_utc() - time::Duration::hours(1);

    sqlx::query(
        "INSERT INTO tb_idempotency_keys (token_id, key, request_hash, response_status, response_body, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(token_id)
    .bind("stale-key")
    .bind("hash")
    .bind(200i16)
    .bind(serde_json::json!({}))
    .bind(stale_time)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tb_idempotency_keys (token_id, key, request_hash, response_status, response_body, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(token_id)
    .bind("fresh-key")
    .bind("hash")
    .bind(200i16)
    .bind(serde_json::json!({}))
    .bind(fresh_time)
    .execute(&pool)
    .await
    .unwrap();

    // ─── tb_messages: status='read' (stale) vs 'queued' (protected) ──────────────
    let stale_msg_time = time::OffsetDateTime::now_utc() - time::Duration::days(91);
    let fresh_msg_time = time::OffsetDateTime::now_utc() - time::Duration::days(30);

    sqlx::query(
        "INSERT INTO tb_messages (id, from_user_id, to_user_id, channel, body, status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(user_id)
    .bind("email")
    .bind("old read message")
    .bind("read")
    .bind(stale_msg_time)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tb_messages (id, from_user_id, to_user_id, channel, body, status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(user_id)
    .bind("email")
    .bind("queued message")
    .bind("queued")
    .bind(fresh_msg_time)
    .execute(&pool)
    .await
    .unwrap();

    // ─── tb_webhook_deliveries: terminal (failed) vs pending ────────────────────
    let stale_webhook_time = time::OffsetDateTime::now_utc() - time::Duration::days(31);
    let fresh_webhook_time = time::OffsetDateTime::now_utc() - time::Duration::days(10);

    sqlx::query(
        "INSERT INTO tb_webhook_deliveries (id, webhook_id, event, payload, status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(webhook_id)
    .bind("test.event")
    .bind(serde_json::json!({}))
    .bind("failed")
    .bind(stale_webhook_time)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tb_webhook_deliveries (id, webhook_id, event, payload, status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(webhook_id)
    .bind("test.event")
    .bind(serde_json::json!({}))
    .bind("pending")
    .bind(fresh_webhook_time)
    .execute(&pool)
    .await
    .unwrap();

    // Run live prune
    sqlx::query("SELECT fn_prune_stale_data(false)")
        .execute(&pool)
        .await
        .unwrap();

    // Assert idempotency: stale gone, fresh remains
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_idempotency_keys WHERE token_id = $1 AND key = 'stale-key'",
    )
    .bind(token_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "stale idempotency key should be deleted");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_idempotency_keys WHERE token_id = $1 AND key = 'fresh-key'",
    )
    .bind(token_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "fresh idempotency key should remain");

    // Assert messages: read+stale gone, queued remains
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_messages WHERE from_user_id = $1 AND body = 'old read message'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "stale read message should be deleted");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_messages WHERE from_user_id = $1 AND body = 'queued message'",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "queued message should remain");

    // Assert webhooks: failed+stale gone, pending remains
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_webhook_deliveries WHERE webhook_id = $1 AND status = 'failed'",
    )
    .bind(webhook_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "stale failed webhook should be deleted");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_webhook_deliveries WHERE webhook_id = $1 AND status = 'pending'",
    )
    .bind(webhook_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "pending webhook should remain");
}

/// Test idempotency: multiple prune calls find nothing after first pass completes.
#[tokio::test]
async fn test_idempotent_prune() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    let user_id = seed_user(&pool).await;
    let agent_id = seed_agent(&pool, user_id).await;
    let token_id = seed_api_token(&pool, agent_id).await;

    let stale_time = time::OffsetDateTime::now_utc() - time::Duration::hours(49);

    // Seed two stale keys to ensure batch deletion completes
    for i in 0..2 {
        sqlx::query(
            "INSERT INTO tb_idempotency_keys (token_id, key, request_hash, response_status, response_body, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(token_id)
        .bind(format!("idempotent-key-{i}"))
        .bind("hash")
        .bind(200i16)
        .bind(serde_json::json!({}))
        .bind(stale_time)
        .execute(&pool)
        .await
        .expect("insert stale key failed");
    }

    // Count stale rows before prune
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tb_idempotency_keys WHERE token_id = $1")
            .bind(token_id)
            .fetch_one(&pool)
            .await
            .expect("count before failed");
    assert_eq!(count_before, 2, "should have 2 stale keys before prune");

    // First prune should delete all stale rows
    sqlx::query("SELECT fn_prune_stale_data(false)")
        .execute(&pool)
        .await
        .expect("first prune failed");

    // Verify all rows are gone
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tb_idempotency_keys WHERE token_id = $1")
            .bind(token_id)
            .fetch_one(&pool)
            .await
            .expect("count after failed");
    assert_eq!(
        count_after, 0,
        "all stale keys should be deleted after prune"
    );

    // Second prune should find nothing to delete (no stale rows remain)
    let result2: Vec<(String, i64, i64)> =
        sqlx::query_as("SELECT table_name, would_delete, deleted FROM fn_prune_stale_data(false)")
            .fetch_all(&pool)
            .await
            .expect("second prune failed");

    let idempotency_result2 = result2
        .iter()
        .find(|(t, _, _)| t == "tb_idempotency_keys")
        .unwrap();
    assert_eq!(
        idempotency_result2.2, 0,
        "tb_idempotency_keys: second prune should have deleted=0 (no stale rows remain)"
    );
}

/// Test soft-deleted assessment deletion respects age cutoff.
#[tokio::test]
async fn test_assessment_cascade_soft_delete() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    let user_id = seed_user(&pool).await;

    // ─── Create stale soft-deleted assessment ────────────────────
    let old_assess_id = Uuid::now_v7();
    let old_delete_time = time::OffsetDateTime::now_utc() - time::Duration::days(91);
    sqlx::query(
        "INSERT INTO tb_assessments (id, title, mode, status, created_by, owner_id, deleted_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(old_assess_id)
    .bind("Old Assessment")
    .bind("practice")
    .bind("draft")
    .bind(user_id)
    .bind(user_id)
    .bind(old_delete_time)
    .execute(&pool)
    .await
    .expect("insert old assessment failed");

    // Create recent soft-deleted assessment (should survive)
    let recent_assess_id = Uuid::now_v7();
    let recent_delete_time = time::OffsetDateTime::now_utc() - time::Duration::days(30);
    sqlx::query(
        "INSERT INTO tb_assessments (id, title, mode, status, created_by, owner_id, deleted_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(recent_assess_id)
    .bind("Recent Assessment")
    .bind("practice")
    .bind("draft")
    .bind(user_id)
    .bind(user_id)
    .bind(recent_delete_time)
    .execute(&pool)
    .await
    .expect("insert recent assessment failed");

    // Run live prune
    sqlx::query("SELECT fn_prune_stale_data(false)")
        .execute(&pool)
        .await
        .expect("prune failed");

    // Assert old assessment is deleted
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_assessments WHERE id = $1")
        .bind(old_assess_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(
        count, 0,
        "stale soft-deleted assessment (>90d) should be deleted"
    );

    // Assert recent soft-deleted assessment survives
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_assessments WHERE id = $1")
        .bind(recent_assess_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(
        count, 1,
        "recent soft-deleted assessment (<90d) should survive"
    );
}

/// Test grade-count exemption: attempt.grade rows survive, others are deleted.
#[tokio::test]
async fn test_grade_count_exemption() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    let owner_id = seed_user(&pool).await;
    let agent_id = seed_agent(&pool, owner_id).await;

    let stale_time = time::OffsetDateTime::now_utc() - time::Duration::days(91);

    // Seed stale activity_log with tool_name='attempt.grade' (should survive)
    sqlx::query(
        "INSERT INTO tb_activity_log (id, ts, actor_id, tool_name, method, path, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(stale_time)
    .bind(agent_id)
    .bind("attempt.grade")
    .bind("POST")
    .bind("/v1/attempts/grade")
    .bind(200i32)
    .execute(&pool)
    .await
    .expect("insert attempt.grade activity failed");

    // Seed stale activity_log with tool_name='question.read' (should be deleted)
    sqlx::query(
        "INSERT INTO tb_activity_log (id, ts, actor_id, tool_name, method, path, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(stale_time)
    .bind(agent_id)
    .bind("question.read")
    .bind("GET")
    .bind("/v1/questions")
    .bind(200i32)
    .execute(&pool)
    .await
    .unwrap();

    // Run live prune
    sqlx::query("SELECT fn_prune_stale_data(false)")
        .execute(&pool)
        .await
        .unwrap();

    // Assert attempt.grade survives
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_activity_log WHERE actor_id = $1 AND tool_name = 'attempt.grade'",
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .expect("failed to count attempt.grade");
    assert_eq!(
        count, 1,
        "stale attempt.grade activity should survive exemption"
    );

    // Assert question.read is deleted
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_activity_log WHERE actor_id = $1 AND tool_name = 'question.read'",
    )
    .bind(agent_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "stale question.read activity should be deleted");
}
