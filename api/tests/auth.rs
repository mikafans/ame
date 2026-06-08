use std::net::SocketAddr;

use axum::{
    Json, Router, middleware,
    routing::{get, post},
};
use reqwest::{StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;

use ame_api::{
    auth::{
        extractor::AuthenticatedUser, scope::RequireScope, scope::ScopeConstraint,
        token::hash_secret,
    },
    domain::user::Scope,
    http::{AppState, idempotency::idempotency_middleware},
};

struct WriteQuestionsScope;
impl ScopeConstraint for WriteQuestionsScope {
    const SCOPE: Scope = Scope::AssessmentWrite;
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../db/migrations");

fn create_test_router(pool: PgPool) -> Router {
    let config = ame_api::config::Config::load().unwrap_or(ame_api::config::Config {
        server: ame_api::config::ServerConfig {
            port: 28080,
            production: false,
            cors_origins: "http://localhost:23000".to_string(),
            log_format: "compact".to_string(),
            database_url: None,
            valkey_url: None,
        },
        ratelimit: ame_api::config::RateLimitConfig {
            free: ame_api::config::TierConfig { burst: 60, rate: 1 },
            premium: ame_api::config::TierConfig {
                burst: 600,
                rate: 10,
            },
            public: ame_api::config::PublicConfig {
                burst: 10,
                period_secs: 2,
            },
            export: ame_api::config::ExportConfig {
                burst: 1,
                period_secs: 60,
            },
            cost: ame_api::config::CostConfig { read: 1, write: 5 },
            trusted_proxies: Some(1),
        },
        quota: ame_api::config::QuotaConfig {
            agents: ame_api::config::TierQuotaConfig {
                free: 1,
                premium: 100,
            },
            assessments: ame_api::config::TierQuotaConfig {
                free: 50,
                premium: 5000,
            },
            questions: ame_api::config::TierQuotaConfig {
                free: 50,
                premium: 5000,
            },
        },
        batch: ame_api::config::BatchConfig {
            free: 50,
            premium: 500,
        },
        login: ame_api::config::LoginConfig::default(),
    });
    let valkey = ame_api::config::create_valkey_pool(&config).unwrap();
    let limiter = std::sync::Arc::new(ame_api::ratelimit::RateLimiter::new(valkey.clone()));
    let state = AppState {
        pool,
        config,
        valkey,
        limiter,
    };
    Router::new()
        .route("/v1/auth/register", post(ame_api::http::auth::register))
        .route("/v1/auth/login", post(ame_api::http::auth::login))
        .route("/v1/auth/logout", post(ame_api::http::auth::logout))
        .route(
            "/protected",
            get(|user: AuthenticatedUser| async move { Json(json!({ "user_id": user.user.id })) }),
        )
        .route(
            "/scoped",
            get(|_user: RequireScope<WriteQuestionsScope>| async move {
                Json(json!({ "status": "ok" }))
            }),
        )
        .route(
            "/idempotent",
            post(|_user: AuthenticatedUser, body: Json<Value>| async move {
                // Return exactly what was sent
                Json(body.0)
            }),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency_middleware,
        ))
        .with_state(state)
}

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

#[tokio::test]
async fn test_auth_and_idempotency() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;

    let app = create_test_router(pool.clone());
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

    // Test 1: No token
    let res = client
        .get(format!("{base_url}/protected"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Setup User and Token
    let user_id = uuid::Uuid::now_v7();
    let agent_id = uuid::Uuid::now_v7();
    let token_id = uuid::Uuid::now_v7();
    let secret = "my_secret_token_123";

    let hash = hash_secret(secret);

    sqlx::query(
        "INSERT INTO tb_users (id, display_name, email, role) \
         VALUES ($1, 'Test User', $2, 'user')",
    )
    .bind(user_id)
    .bind(format!("auth-{user_id}@example.com"))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO tb_agents (id, owner_user_id, label) VALUES ($1, $2, 'test-agent')")
        .bind(agent_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let scopes = vec!["assessment.read".to_string(), "attempt.write".to_string()];
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(7);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes, expires_at) VALUES ($1, $2, 'test-key', $3, $4, $5)",
    )
    .bind(token_id)
    .bind(agent_id)
    .bind(hash)
    .bind(&scopes)
    .bind(expires_at)
    .execute(&pool)
    .await
    .unwrap();

    let bearer_token = format!("agt_{}_{}", token_id, secret);

    // Test 2: Valid token
    let res = client
        .get(format!("{base_url}/protected"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["user_id"].as_str().unwrap(), agent_id.to_string());

    // Test 3: Scope required but missing
    let res = client
        .get(format!("{base_url}/scoped"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN); // Missing "assessment.write"
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["error"]["details"]["scope"], "assessment.write");

    // Test 4: Idempotency
    let idem_key = "test-key-1";
    let payload = json!({ "question": "test" });

    let res1 = client
        .post(format!("{base_url}/idempotent"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .header("Idempotency-Key", idem_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res1.status(), StatusCode::OK);

    // Same request -> should succeed (replay)
    let res2 = client
        .post(format!("{base_url}/idempotent"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .header("Idempotency-Key", idem_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body2: Value = res2.json().await.unwrap();
    assert_eq!(body2, payload);

    // Same token id and idempotency key with the wrong secret must not replay
    // the stored response before authentication runs.
    let invalid_bearer_token = format!("{}_wrong_secret", token_id);
    let res = client
        .post(format!("{base_url}/idempotent"))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", invalid_bearer_token),
        )
        .header("Idempotency-Key", idem_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Different body -> conflict
    let new_payload = json!({ "question": "different" });
    let res3 = client
        .post(format!("{base_url}/idempotent"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .header("Idempotency-Key", idem_key)
        .json(&new_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res3.status(), StatusCode::CONFLICT);
}

/// Tokens with `revoked_at IS NOT NULL` must be rejected even when the bearer
/// secret matches. This guards the revocation branch in
/// `api/src/auth/extractor.rs`; without this test, a regression that flipped
/// the conditional would only surface in production.
#[tokio::test]
async fn revoked_token_returns_unauthorized() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let app = create_test_router(pool.clone());
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

    let user_id = uuid::Uuid::now_v7();
    let token_id = uuid::Uuid::now_v7();
    let secret = "revoked_secret_xyz";

    let hash = hash_secret(secret);

    sqlx::query(
        "INSERT INTO tb_users (id, display_name, email, role) \
         VALUES ($1, 'Revoked User', $2, 'user')",
    )
    .bind(user_id)
    .bind(format!("revoked-{user_id}@example.com"))
    .execute(&pool)
    .await
    .unwrap();

    // revoked_at set at insert time — the secret hash is still valid, so any
    // 200 here would prove the extractor stopped checking revocation.
    let agent_id = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO tb_agents (id, owner_user_id, label) VALUES ($1, $2, 'test')")
        .bind(agent_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let scopes = vec!["assessment.read".to_string()];
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(7);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes, expires_at, revoked_at) \
         VALUES ($1, $2, 'test-key', $3, $4, $5, now())",
    )
    .bind(token_id)
    .bind(agent_id)
    .bind(hash)
    .bind(&scopes)
    .bind(expires_at)
    .execute(&pool)
    .await
    .unwrap();

    let bearer_token = format!("agt_{}_{}", token_id, secret);
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://{addr}/protected"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_caching_and_invalidation() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let config = ame_api::config::Config::load().unwrap();
    let valkey = ame_api::config::create_valkey_pool(&config).unwrap();

    let app = create_test_router(pool.clone());
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

    // Setup user and token
    let user_id = uuid::Uuid::now_v7();
    let token_id = uuid::Uuid::now_v7();
    let secret = "cached_secret_123";
    let hash = hash_secret(secret);

    sqlx::query(
        "INSERT INTO tb_users (id, display_name, email, role) \
         VALUES ($1, 'Cache Test User', $2, 'user')",
    )
    .bind(user_id)
    .bind(format!("cache-{user_id}@example.com"))
    .execute(&pool)
    .await
    .unwrap();

    let agent_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_agents (id, owner_user_id, label) VALUES ($1, $2, 'cache_test_agent')",
    )
    .bind(agent_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let scopes = vec!["assessment.read".to_string()];
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::days(7);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes, expires_at) VALUES ($1, $2, 'test-key', $3, $4, $5)",
    )
    .bind(token_id)
    .bind(agent_id)
    .bind(hash)
    .bind(&scopes)
    .bind(expires_at)
    .execute(&pool)
    .await
    .unwrap();

    let bearer_token = format!("agt_{}_{}", token_id, secret);

    // 1. Initial request: should populate cache
    let res = client
        .get(format!("{base_url}/protected"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Modify database directly (revoke token) to bypass normal API pathways
    sqlx::query("UPDATE tb_api_tokens SET revoked_at = now() WHERE id = $1")
        .bind(token_id)
        .execute(&pool)
        .await
        .unwrap();

    // 3. Request should STILL succeed because it hits the cached (non-revoked) info
    let res = client
        .get(format!("{base_url}/protected"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Manually invalidate token cache key
    ame_api::auth::extractor::invalidate_token(&valkey, token_id).await;

    // 5. Subsequent request should fail (cache miss, loads revoked token from DB)
    let res = client
        .get(format!("{base_url}/protected"))
        .header(header::AUTHORIZATION, format!("Bearer {}", bearer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_email_canonical_dedup() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let app = create_test_router(pool.clone());
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

    let rand_id = uuid::Uuid::now_v7();
    let email1 = format!("bob-{rand_id}@example.com");
    let email2 = format!("bob-{rand_id}+alias@example.com");

    // 1. Register a user with email1
    let payload = json!({
        "email": email1,
        "name": "Bob",
        "password": "password123",
        "role": "user"
    });
    let res = client
        .post(format!("{base_url}/v1/auth/register"))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 2. Register email2 (should conflict via email_canonical)
    let payload_alias = json!({
        "email": email2,
        "name": "Bob Alias",
        "password": "password123",
        "role": "user"
    });
    let res = client
        .post(format!("{base_url}/v1/auth/register"))
        .json(&payload_alias)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body: Value = res.json().await.unwrap();
    assert_eq!(
        body["error"]["details"]["fields"][0]["message"],
        "email already registered"
    );

    // 3. Login using email2 (canonical alias) should succeed and authenticate as Bob
    let login_payload = json!({
        "email": email2,
        "password": "password123"
    });
    let res = client
        .post(format!("{base_url}/v1/auth/login"))
        .json(&login_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let login_body: Value = res.json().await.unwrap();
    assert_eq!(login_body["user"]["email"], email1);
}

#[tokio::test]
async fn test_per_account_login_rate_limiting() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        return;
    }

    let pool = setup_db().await;

    // Set up a custom config with production = true to enable the login rate limiter
    let config = ame_api::config::Config {
        server: ame_api::config::ServerConfig {
            port: 28080,
            production: true, // Enable prod rate limits
            cors_origins: "http://localhost:23000".to_string(),
            log_format: "compact".to_string(),
            database_url: None,
            valkey_url: None,
        },
        ratelimit: ame_api::config::RateLimitConfig {
            free: ame_api::config::TierConfig {
                burst: 1000,
                rate: 1000,
            },
            premium: ame_api::config::TierConfig {
                burst: 1000,
                rate: 1000,
            },
            public: ame_api::config::PublicConfig {
                burst: 1000,
                period_secs: 1,
            },
            export: ame_api::config::ExportConfig {
                burst: 1000,
                period_secs: 1,
            },
            cost: ame_api::config::CostConfig { read: 1, write: 1 },
            trusted_proxies: Some(1),
        },
        quota: ame_api::config::QuotaConfig {
            agents: ame_api::config::TierQuotaConfig {
                free: 1000,
                premium: 1000,
            },
            assessments: ame_api::config::TierQuotaConfig {
                free: 1000,
                premium: 1000,
            },
            questions: ame_api::config::TierQuotaConfig {
                free: 1000,
                premium: 1000,
            },
        },
        batch: ame_api::config::BatchConfig {
            free: 1000,
            premium: 1000,
        },
        login: ame_api::config::LoginConfig::default(),
    };

    let valkey = ame_api::config::create_valkey_pool(&config).unwrap();
    let limiter = std::sync::Arc::new(ame_api::ratelimit::RateLimiter::new(valkey.clone()));
    let state = ame_api::http::AppState {
        pool: ame_api::http::db::convert_pool_to_ame_app(&pool),
        config,
        valkey,
        limiter,
    };

    // We can merge the auth routes for testing
    let app = Router::new()
        .route("/v1/auth/login", post(ame_api::http::auth::login))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
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

    let rand_id = uuid::Uuid::now_v7();
    let login_payload = json!({
        "email": format!("bruteforce-{}@example.com", rand_id),
        "password": "wrongpassword"
    });

    // Make 5 attempts (burst of 5)
    for _ in 0..5 {
        let res = client
            .post(format!("{base_url}/v1/auth/login"))
            .json(&login_payload)
            .send()
            .await
            .unwrap();
        // Since the user is not found, we get 401
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // 6th attempt should return 429 Too Many Requests
    let res = client
        .post(format!("{base_url}/v1/auth/login"))
        .json(&login_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}
