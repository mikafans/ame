use ame_api::http::router;
use reqwest::StatusCode;
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

#[tokio::test]
async fn test_export_premium_gate_and_rate_limit() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
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
    let free_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Free Human', 'user', 'free')")
        .bind(free_id)
        .bind(format!("free-{}@example.com", free_id))
        .execute(&pool)
        .await
        .unwrap();

    let free_token_id = Uuid::now_v7();
    let secret = "secret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5::text[])")
        .bind(free_token_id)
        .bind(free_id)
        .bind("free-key")
        .bind(&hash)
        .bind(vec![
            "quiz.read".to_string(),
            "quiz.write".to_string(),
            "attempt.read".to_string(),
            "attempt.write".to_string(),
            "stats.read".to_string(),
            "feedback.write".to_string(),
            "plan.read".to_string(),
            "plan.write".to_string(),
            "public.publish".to_string(),
        ])
        .execute(&pool)
        .await
        .unwrap();
    let free_auth = format!("{free_token_id}_{secret}");

    // 2. Assert free user gets 403 Forbidden
    let res = client
        .get(format!("{base_url}/v1/me/export"))
        .header("Authorization", format!("Bearer {free_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let body = res.json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["error"]["code"], "scope_required");
    assert_eq!(body["error"]["details"]["scope"], "premium");

    // 3. Create a premium user
    let prem_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, 'Premium Human', 'user', 'premium')")
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
        .bind(vec![
            "quiz.read".to_string(),
            "quiz.write".to_string(),
            "attempt.read".to_string(),
            "attempt.write".to_string(),
            "stats.read".to_string(),
            "feedback.write".to_string(),
            "plan.read".to_string(),
            "plan.write".to_string(),
            "public.publish".to_string(),
        ])
        .execute(&pool)
        .await
        .unwrap();
    let prem_auth = format!("{prem_token_id}_{secret}");

    // Seed some data for premium owner to export
    let quiz_id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_quizzes (id, title, objectives, course, visibility, status, created_by) VALUES ($1, 'Exportable Quiz', '{}', 'Test Course', 'private', 'draft', $2)")
        .bind(quiz_id)
        .bind(prem_id)
        .execute(&pool)
        .await
        .unwrap();

    // 4. Assert Premium owner gets successful data export
    let res = client
        .get(format!("{base_url}/v1/me/export"))
        .header("Authorization", format!("Bearer {prem_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let export_bundle = res.json::<serde_json::Value>().await.unwrap();
    assert!(export_bundle["quizzes"].is_array());
    assert_eq!(export_bundle["quizzes"].as_array().unwrap().len(), 1);
    assert_eq!(export_bundle["quizzes"][0]["id"], quiz_id.to_string());
    assert!(export_bundle["questions"].is_array());
    assert!(export_bundle["sessions"].is_array());
    assert!(export_bundle["attempts"].is_array());
    assert!(export_bundle["tagRatings"].is_array());

    // 5. Assert second subsequent call gets 429 Too Many Requests (tighter rate limiting)
    let res2 = client
        .get(format!("{base_url}/v1/me/export"))
        .header("Authorization", format!("Bearer {prem_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::TOO_MANY_REQUESTS);
}
