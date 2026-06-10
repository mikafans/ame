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
        panic!("failed to run migrations: {error}");
    }

    pool
}

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

/// Seed a login session for `user_id` with the given scopes, returning the
/// bearer value (`lgn_{id}_{secret}`).
async fn seed_login(pool: &PgPool, user_id: Uuid, scopes: &[&str]) -> String {
    let token_id = Uuid::now_v7();
    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(7);
    sqlx::query(
        "INSERT INTO tb_login_sessions (id, user_id, token_hash, scopes, expires_at)
         VALUES ($1, $2, $3, $4::text[], $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(&hash)
    .bind(scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    .bind(expires_at)
    .execute(pool)
    .await
    .unwrap();
    format!("lgn_{token_id}_{secret}")
}

async fn seed_user(pool: &PgPool, role: &str, plan: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(format!("settings-{id}@example.com"))
    .bind(format!("Settings {role}"))
    .bind(role)
    .bind(plan)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Reset the maintenance flag so a leftover row from a prior run can't make the
/// whole suite (which shares one DB) start in maintenance mode.
async fn reset_settings(pool: &PgPool) {
    sqlx::query("UPDATE tb_settings SET value = 'false'::jsonb WHERE key = 'maintenance_mode'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tb_settings WHERE key IN ('ratelimit', 'quota')")
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_settings_get_defaults_and_admin_guard() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    reset_settings(&pool).await;
    let base = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let admin_id = seed_user(&pool, "admin", "premium").await;
    let admin_auth = seed_login(&pool, admin_id, &["admin"]).await;
    let user_id = seed_user(&pool, "user", "free").await;
    let user_auth = seed_login(&pool, user_id, &["assessment.read"]).await;

    // Non-admin is forbidden on both verbs.
    let res = client
        .get(format!("{base}/v1/admin/settings"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .put(format!("{base}/v1/admin/settings"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .json(&json!({"maintenanceMode": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Admin GET returns the config defaults with maintenance off.
    let res = client
        .get(format!("{base}/v1/admin/settings"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["maintenanceMode"], false);
    // Tier values are present and mirror the config defaults.
    assert!(body["ratelimit"]["free"]["burst"].as_u64().is_some());
    assert!(body["quota"]["assessments"]["free"].as_i64().is_some());
}

#[tokio::test]
async fn test_settings_put_persists_and_audits() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    reset_settings(&pool).await;
    let base = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let admin_id = seed_user(&pool, "admin", "premium").await;
    let admin_auth = seed_login(&pool, admin_id, &["admin"]).await;

    // PUT turns maintenance on; response reflects the new effective state.
    let res = client
        .put(format!("{base}/v1/admin/settings"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"maintenanceMode": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["maintenanceMode"], true);

    // Persisted to tb_settings with the acting admin recorded.
    let (value, updated_by): (serde_json::Value, Option<Uuid>) =
        sqlx::query_as("SELECT value, updated_by FROM tb_settings WHERE key = 'maintenance_mode'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(value, json!(true));
    assert_eq!(updated_by, Some(admin_id));

    // One audit row was written for the change.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_audit_log WHERE action = 'settings.update' AND actor_user_id = $1",
    )
    .bind(admin_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(count >= 1, "expected a settings.update audit row");

    // Restore so we don't leave the shared DB in maintenance mode.
    reset_settings(&pool).await;
}

#[tokio::test]
async fn test_maintenance_mode_blocks_non_admin_but_allows_admin() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    reset_settings(&pool).await;
    let base = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let admin_id = seed_user(&pool, "admin", "premium").await;
    let admin_auth = seed_login(&pool, admin_id, &["admin"]).await;
    let user_id = seed_user(&pool, "user", "free").await;
    let user_auth = seed_login(&pool, user_id, &["assessment.read", "assessment.write"]).await;

    // Flip maintenance on via the API.
    let res = client
        .put(format!("{base}/v1/admin/settings"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"maintenanceMode": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Non-admin learner request is rejected with 503.
    let res = client
        .get(format!("{base}/v1/me"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], "maintenance");

    // Admin still passes through.
    let res = client
        .get(format!("{base}/v1/admin/health"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Login stays reachable so an operator can authenticate during maintenance.
    let res = client
        .post(format!("{base}/v1/auth/login"))
        .json(&json!({"email": "nobody@example.com", "password": "wrong"}))
        .send()
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Turn it back off and confirm the learner is served again.
    let res = client
        .put(format!("{base}/v1/admin/settings"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"maintenanceMode": false}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .get(format!("{base}/v1/me"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    reset_settings(&pool).await;
}

#[tokio::test]
async fn test_quota_override_via_settings_triggers_429() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }
    let pool = setup_db().await;
    reset_settings(&pool).await;
    let base = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let admin_id = seed_user(&pool, "admin", "premium").await;
    let admin_auth = seed_login(&pool, admin_id, &["admin"]).await;
    let user_id = seed_user(&pool, "user", "free").await;
    let user_auth = seed_login(&pool, user_id, &["assessment.read", "assessment.write"]).await;

    // Override the free assessment quota to 0 — the override must be honored by
    // check_quota (proving the quota path reads settings, not just config).
    let res = client
        .put(format!("{base}/v1/admin/settings"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({
            "quota": {
                "agents": {"free": 1, "premium": 100},
                "assessments": {"free": 0, "premium": 5000},
                "questions": {"free": 50, "premium": 5000}
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // A free user creating an assessment now hits the quota and gets 429.
    let res = client
        .post(format!("{base}/v1/assessments"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .json(&json!({"title": "Quota probe"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], "quota_exceeded");

    reset_settings(&pool).await;
}
