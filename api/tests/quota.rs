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
        panic!("failed to run migrations for quota test: {error}");
    }

    pool
}

#[tokio::test]
async fn test_quota_enforcement() {
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

    // 1. Create a free user
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Free User', 'user', 'free')")
        .bind(user_id)
        .bind(format!("free-{}@example.com", user_id))
        .execute(&pool)
        .await
        .unwrap();

    let token_id = Uuid::now_v7();
    let secret = "free-secret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(token_id)
        .bind(user_id)
        .bind("free-key")
        .bind(&hash)
        .bind(vec!["assessment.read".to_string(), "assessment.write".to_string(), "admin".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let auth = format!("{token_id}_{secret}");

    // 2. Test Agent Creation Quota (Free plan: 1 limit)
    let res = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({"label": "Agent 1", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res2 = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {auth}"))
        .json(&json!({"label": "Agent 2", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
    let error_body = res2.json::<serde_json::Value>().await.unwrap();
    assert_eq!(error_body["error"]["code"], "quota_exceeded");
    assert_eq!(error_body["error"]["details"]["kind"], "agent_creation");
    assert_eq!(error_body["error"]["details"]["limit"], 1);

    // Test that Premium user has higher limits (up to 100 agents)
    // Create a premium user
    let prem_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Prem User', 'user', 'premium')")
        .bind(prem_id)
        .bind(format!("prem-{}@example.com", prem_id))
        .execute(&pool)
        .await
        .unwrap();

    let prem_token_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(prem_token_id)
        .bind(prem_id)
        .bind("prem-key")
        .bind(&hash)
        .bind(vec!["assessment.read".to_string(), "assessment.write".to_string(), "admin".to_string()])
        .execute(&pool)
        .await
        .unwrap();
    let prem_auth = format!("{prem_token_id}_{secret}");

    // Create 2nd agent (should succeed because limit is 100)
    client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {prem_auth}"))
        .json(&json!({"label": "Agent 1", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{base_url}/v1/me/agents"))
        .header("Authorization", format!("Bearer {prem_auth}"))
        .json(&json!({"label": "Agent 2", "scopes": ["assessment.read"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_assessment_and_question_quota_check() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return;
    }

    let pool = setup_db().await;
    let app_pool = ame_api::http::db::convert_pool_to_ame_app(&pool);
    let mut config = ame_api::config::Config::load().unwrap();
    // Set low quotas for testing
    config.quota.assessments.free = 2;
    config.quota.questions.free = 5;

    // Create free user
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Free User', 'user', 'free')")
        .bind(user_id)
        .bind(format!("quota-test-{}@example.com", user_id))
        .execute(&pool)
        .await
        .unwrap();

    // 1. Initial checks should pass
    assert!(
        ame_api::http::quota::check_quota(
            &app_pool,
            None,
            &config,
            user_id,
            ame_api::http::quota::QuotaKind::Assessment,
            1
        )
        .await
        .is_ok()
    );
    assert!(
        ame_api::http::quota::check_quota(
            &app_pool,
            None,
            &config,
            user_id,
            ame_api::http::quota::QuotaKind::Question,
            3
        )
        .await
        .is_ok()
    );

    // 2. Let's create some assessments in the DB for this user
    for i in 0..2 {
        sqlx::query("INSERT INTO tb_assessments (id, title, mode, status, created_by, total_points) VALUES ($1, $2, 'practice', 'draft', $3, 0)")
            .bind(Uuid::now_v7())
            .bind(format!("Assessment {i}"))
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    // 3. Check quota: now usage is 2, adding 1 should fail (limit is 2)
    let res = ame_api::http::quota::check_quota(
        &app_pool,
        None,
        &config,
        user_id,
        ame_api::http::quota::QuotaKind::Assessment,
        1,
    )
    .await;
    assert!(res.is_err());
    if let Err(ame_api::domain::error::ApiError::QuotaExceeded { kind, limit, usage }) = res {
        assert_eq!(kind, "assessment");
        assert_eq!(limit, 2);
        assert_eq!(usage, 2);
    } else {
        panic!("expected QuotaExceeded error");
    }

    // 4. Create 4 questions in the DB
    for _ in 0..4 {
        sqlx::query("INSERT INTO tb_questions (id, owner_id, kind, prompt, payload, status, points, created_by) VALUES ($1, $2, 'mc', 'Prompt', '{}', 'draft', 1, $2)")
            .bind(Uuid::now_v7())
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    // 5. Check questions quota: usage is 4, limit is 5. Adding 1 is ok, adding 2 should fail.
    assert!(
        ame_api::http::quota::check_quota(
            &app_pool,
            None,
            &config,
            user_id,
            ame_api::http::quota::QuotaKind::Question,
            1
        )
        .await
        .is_ok()
    );
    let res_q = ame_api::http::quota::check_quota(
        &app_pool,
        None,
        &config,
        user_id,
        ame_api::http::quota::QuotaKind::Question,
        2,
    )
    .await;
    assert!(res_q.is_err());
    if let Err(ame_api::domain::error::ApiError::QuotaExceeded { kind, limit, usage }) = res_q {
        assert_eq!(kind, "question");
        assert_eq!(limit, 5);
        assert_eq!(usage, 4);
    } else {
        panic!("expected QuotaExceeded error");
    }
}
