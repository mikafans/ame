use axum::{Router, middleware, routing::post};
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::net::SocketAddr;
use uuid::Uuid;

use ame_api::http::{
    AppState, admin, auth, auth_extract_middleware, db::convert_pool_to_ame_app, me,
};

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
        panic!("failed to run migrations for auth integration test: {error}");
    }

    pool
}

fn create_test_router(pool: PgPool, ttl_seconds: u64) -> Router {
    let pool = convert_pool_to_ame_app(&pool);
    let mut config = ame_api::config::Config::load().expect("failed to load config");
    config.login.ttl_seconds = ttl_seconds;
    let valkey =
        ame_api::config::create_valkey_pool(&config).expect("failed to create valkey pool");
    let limiter = std::sync::Arc::new(ame_api::ratelimit::RateLimiter::new(valkey.clone()));
    let state = AppState {
        pool,
        config,
        valkey,
        limiter,
    };

    let logged_router = Router::new().merge(me::router(state.clone()));

    let api_routes = Router::new()
        .merge(admin::router(state.clone()))
        .route("/v1/auth/register", post(auth::register))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/logout", post(auth::logout))
        .merge(logged_router)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_extract_middleware,
        ));

    Router::new().merge(api_routes).with_state(state)
}

#[tokio::test]
async fn test_login_sessions_flow() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;

    // Build app with 5 seconds TTL
    let app = create_test_router(pool.clone(), 5);
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

    // 1. Register a user
    let email = format!("test-login-{}@example.com", Uuid::now_v7());
    let password = "password123";
    let register_res = client
        .post(format!("{base_url}/v1/auth/register"))
        .json(&json!({
            "email": email,
            "name": "Test Login User",
            "password": password,
            "role": "user"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(register_res.status(), StatusCode::CREATED);
    let reg_json: Value = register_res.json().await.unwrap();
    let _user_id = reg_json["user"]["id"].as_str().unwrap().to_string();

    // The register response returns cookie or token. Let's try login to fetch cookie/token cleanly.
    let login_res = client
        .post(format!("{base_url}/v1/auth/login"))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let login_json: Value = login_res.json().await.unwrap();
    let token = login_json["token"].as_str().unwrap().to_string();

    // 2. Make authenticated request to /v1/me -> succeeds
    let me_res = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me_res.status(), StatusCode::OK);

    // 3. Sliding expiration test:
    // Wait 2 seconds (less than 5s TTL) and make request to refresh
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let me_res2 = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me_res2.status(), StatusCode::OK);

    // Wait another 2 seconds. Total elapsed time since login is 4 seconds.
    // Make another request to refresh.
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let me_res2b = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me_res2b.status(), StatusCode::OK);

    // Wait another 2 seconds. Total time since login is 6 seconds (which exceeds initial 5s TTL).
    // But since we refreshed, the sliding TTL keeps it alive.
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let me_res3 = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me_res3.status(), StatusCode::OK);

    // 4. Idle past TTL test:
    // Now wait 6 seconds (idle past TTL of 5s)
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
    let me_res4 = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me_res4.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_sessions_logout_and_admin_demote() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }

    let pool = setup_db().await;

    // Build app with longer TTL
    let app = create_test_router(pool.clone(), 3600);
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

    // Create an admin user to perform deactivation/demotion
    let admin_id = Uuid::now_v7();
    let admin_email = format!("valkey-admin-{}@example.com", admin_id);
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin User', 'admin', 'premium')",
    )
    .bind(admin_id)
    .bind(&admin_email)
    .execute(&pool)
    .await
    .unwrap();

    let admin_token_id = Uuid::now_v7();
    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_id)
    .bind(&hash)
    .bind(vec!["admin".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{secret}");

    // Register user
    let email = format!("test-logout-{}@example.com", Uuid::now_v7());
    let password = "password123";
    let register_res = client
        .post(format!("{base_url}/v1/auth/register"))
        .json(&json!({
            "email": email,
            "name": "Test Logout User",
            "password": password,
            "role": "user"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(register_res.status(), StatusCode::CREATED);
    let reg_json: Value = register_res.json().await.unwrap();
    let user_id = Uuid::parse_str(reg_json["user"]["id"].as_str().unwrap()).unwrap();

    // Login user 1 (first device)
    let login_res = client
        .post(format!("{base_url}/v1/auth/login"))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();
    let token1 = login_res.json::<Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Login user 1 (second device)
    let login_res2 = client
        .post(format!("{base_url}/v1/auth/login"))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();
    let token2 = login_res2.json::<Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();
    let token1_id = Uuid::parse_str(token1.split('_').next().unwrap()).unwrap();
    let token2_id = Uuid::parse_str(token2.split('_').next().unwrap()).unwrap();

    // Assert rows exist in tb_login_sessions
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_login_sessions WHERE id = $1")
        .bind(token1_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "Login session 1 must exist in database");

    let count2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_login_sessions WHERE id = $1")
        .bind(token2_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count2, 1, "Login session 2 must exist in database");

    // Verify both tokens work
    let me1 = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token1}"))
        .send()
        .await
        .unwrap();
    let me2 = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token2}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me1.status(), StatusCode::OK);
    assert_eq!(me2.status(), StatusCode::OK);

    // Logout token 1
    let logout_res = client
        .post(format!("{base_url}/v1/auth/logout"))
        .header("Authorization", format!("Bearer {token1}"))
        .send()
        .await
        .unwrap();
    assert_eq!(logout_res.status(), StatusCode::OK);

    // Assert login session 1 is gone from database
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_login_sessions WHERE id = $1")
        .bind(token1_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 0,
        "Login session 1 must be deleted from database after logout"
    );

    // Verify token 1 no longer works
    let me1_after = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token1}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me1_after.status(), StatusCode::UNAUTHORIZED);

    // Verify token 2 still works (concurrency check)
    let me2_after = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token2}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me2_after.status(), StatusCode::OK);

    // Now disable/deactivate user via Admin PATCH endpoint
    let patch_res = client
        .patch(format!("{base_url}/v1/admin/users/{user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({
            "disabled": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(patch_res.status(), StatusCode::NO_CONTENT);

    // Verify token 2 is immediately revoked/invalidated
    let me2_after_disable = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {token2}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me2_after_disable.status(), StatusCode::UNAUTHORIZED);

    // Verify admin token audit excludes logins (neither token1 nor token2 is in the DB)
    let audit_res = client
        .get(format!("{base_url}/v1/admin/tokens"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(audit_res.status(), StatusCode::OK);
    let audit_json: Value = audit_res.json().await.unwrap();
    let tokens = audit_json["tokens"].as_array().unwrap();

    let id1 = token1.split('_').next().unwrap();
    let id2 = token2.split('_').next().unwrap();

    for t in tokens {
        let id = t["id"].as_str().unwrap();
        assert_ne!(
            id, id1,
            "Login token 1 must not appear in admin token audit"
        );
        assert_ne!(
            id, id2,
            "Login token 2 must not appear in admin token audit"
        );
    }
}

#[tokio::test]
async fn test_admin_plan_change_preserves_login_session() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }

    let pool = setup_db().await;

    // Build app with longer TTL
    let app = create_test_router(pool.clone(), 3600);
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

    // Create an admin user to perform the plan change
    let admin_id = Uuid::now_v7();
    let admin_email = format!("plan-change-admin-{}@example.com", admin_id);
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin User', 'admin', 'premium')",
    )
    .bind(admin_id)
    .bind(&admin_email)
    .execute(&pool)
    .await
    .unwrap();

    let admin_token_id = Uuid::now_v7();
    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_id)
    .bind(&hash)
    .bind(vec!["admin".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{secret}");

    // Register a normal user
    let email = format!("plan-user-{}@example.com", Uuid::now_v7());
    let password = "password123";
    let register_res = client
        .post(format!("{base_url}/v1/auth/register"))
        .json(&json!({
            "email": email,
            "name": "Plan Test User",
            "password": password,
            "role": "user"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(register_res.status(), StatusCode::CREATED);
    let reg_json: Value = register_res.json().await.unwrap();
    let user_id = Uuid::parse_str(reg_json["user"]["id"].as_str().unwrap()).unwrap();

    // User logs in (creates login session)
    let login_res = client
        .post(format!("{base_url}/v1/auth/login"))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let login_json: Value = login_res.json().await.unwrap();
    let user_token = login_json["token"].as_str().unwrap().to_string();
    let user_token_id = Uuid::parse_str(user_token.split('_').next().unwrap()).unwrap();

    // Verify user can authenticate with their token
    let me_before = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {user_token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me_before.status(), StatusCode::OK);

    // Verify login session exists in database
    let session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tb_login_sessions WHERE id = $1")
            .bind(user_token_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        session_count, 1,
        "Login session must exist before plan change"
    );

    // Admin upgrades user from free to premium (plan-only change, no role/disable)
    let patch_res = client
        .patch(format!("{base_url}/v1/admin/users/{user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({
            "plan": "premium"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(patch_res.status(), StatusCode::NO_CONTENT);

    // User's original token should STILL work (plan change does not force re-login)
    let me_after = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {user_token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(me_after.status(), StatusCode::OK);

    // Login session should still exist in database
    let session_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tb_login_sessions WHERE id = $1")
            .bind(user_token_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        session_count_after, 1,
        "Login session must persist after plan change"
    );
}
