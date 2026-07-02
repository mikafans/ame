use ame_api::http::router;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::io::Read;
use std::net::SocketAddr;
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
        .expect("set DATABASE_URL to a reachable Postgres database before AME_RUN_DB_TESTS=1");

    if let Err(error) = MIGRATOR.run(&pool).await {
        panic!("failed to run migrations: {error}");
    }

    pool
}

async fn create_test_user(pool: &PgPool, email_prefix: &str) -> (Uuid, String) {
    let user_id = Uuid::now_v7();
    let email = format!("{}-{}@example.com", email_prefix, user_id);
    sqlx::query("INSERT INTO tb_users (id, email, display_name, role, plan) VALUES ($1, $2, $3, 'user', 'premium')")
        .bind(user_id)
        .bind(&email)
        .bind(email_prefix)
        .execute(pool)
        .await
        .unwrap();

    let session_id = Uuid::now_v7();
    let secret = "test-secret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query("INSERT INTO tb_login_sessions (id, user_id, token_hash, scopes, expires_at) VALUES ($1, $2, $3, $4, $5)")
        .bind(session_id)
        .bind(user_id)
        .bind(&hash)
        .bind(vec![
            "assessment.read".to_string(),
            "assessment.write".to_string(),
            "attempt.read".to_string(),
            "attempt.write".to_string(),
            "stats.read".to_string(),
            "feedback.write".to_string(),
            "plan.read".to_string(),
            "plan.write".to_string(),
        ])
        .bind(OffsetDateTime::now_utc() + Duration::days(7))
        .execute(pool)
        .await
        .unwrap();

    (user_id, format!("lgn_{session_id}_{secret}"))
}

#[tokio::test]
async fn test_deep_dives_kb_operations() {
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

    // 1. Create User A (Owner) and User B (Stranger)
    let (user_a_id, auth_a) = create_test_user(&pool, "user-a").await;
    let (user_b_id, auth_b) = create_test_user(&pool, "user-b").await;

    // 2. Create questions owned by User A and User B
    let q_a_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_questions (id, owner_id, kind, prompt, payload, status, points, created_by) \
         VALUES ($1, $2, 'mc', 'What is the speed of light?', $3, 'live', 1, $2)",
    )
    .bind(q_a_id)
    .bind(user_a_id)
    .bind(json!({ "options": ["299,792 km/s", "150,000 km/s"], "correct_index": 0 }))
    .execute(&pool)
    .await
    .unwrap();

    let q_b_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_questions (id, owner_id, kind, prompt, payload, status, points, created_by) \
         VALUES ($1, $2, 'mc', 'What is the atomic number of Hydrogen?', $3, 'live', 1, $2)",
    )
    .bind(q_b_id)
    .bind(user_b_id)
    .bind(json!({ "options": ["1", "2"], "correct_index": 0 }))
    .execute(&pool)
    .await
    .unwrap();

    // 3. User A requests a deep dive on q_a_id
    let res = client
        .post(format!("{base_url}/v1/deep-dives"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .json(&json!({
            "questionId": q_a_id,
            "reason": "Explain the speed of light"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dive_a: serde_json::Value = res.json().await.unwrap();
    let dive_a_id = dive_a["id"].as_str().unwrap().to_string();

    // 4. Verify Owner Isolation: User B cannot read, write, publish or export User A's deep dive
    // Read:
    let res = client
        .get(format!("{base_url}/v1/deep-dives/{dive_a_id}"))
        .header("Authorization", format!("Bearer {auth_b}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Patch status:
    let res = client
        .patch(format!("{base_url}/v1/deep-dives/{dive_a_id}"))
        .header("Authorization", format!("Bearer {auth_b}"))
        .json(&json!({ "status": "archived" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Publish:
    let res = client
        .patch(format!("{base_url}/v1/deep-dives/{dive_a_id}/publish"))
        .header("Authorization", format!("Bearer {auth_b}"))
        .json(&json!({
            "bodyMarkdown": "# Light speed\n\nExplanation by unauthorized user.",
            "status": "published"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Export: User B exports, should NOT contain User A's deep dive (ZIP should contain 0 entries)
    let res = client
        .get(format!("{base_url}/v1/deep-dives/export"))
        .header("Authorization", format!("Bearer {auth_b}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("content-type").unwrap().to_str().unwrap(),
        "application/zip"
    );
    let zip_bytes = res.bytes().await.unwrap();
    let archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
    assert_eq!(archive.len(), 0);

    // 5. User A publishes the deep dive with category and body markdown
    let res = client
        .patch(format!("{base_url}/v1/deep-dives/{dive_a_id}/publish"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .json(&json!({
            "bodyMarkdown": "# Light Speed explanation\n\nLight travels fast.",
            "status": "published",
            "category": "Physics"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let published_dive: serde_json::Value = res.json().await.unwrap();
    assert_eq!(published_dive["category"], "Physics");
    assert_eq!(
        published_dive["bodyMarkdown"],
        "# Light Speed explanation\n\nLight travels fast."
    );

    // Verify first revision created
    let rev1: Option<(i32, String, String)> = sqlx::query_as(
        "SELECT revision, body_markdown, category FROM tb_deep_dive_revisions WHERE deep_dive_id = $1"
    )
    .bind(Uuid::parse_str(&dive_a_id).unwrap())
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(rev1.is_some());
    let (revision, body_markdown, category) = rev1.unwrap();
    assert_eq!(revision, 1);
    assert_eq!(
        body_markdown,
        "# Light Speed explanation\n\nLight travels fast."
    );
    assert_eq!(category, "Physics");

    // 6. User A updates user note (creates revision 2)
    let res = client
        .patch(format!("{base_url}/v1/deep-dives/{dive_a_id}"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .json(&json!({
            "userNote": "This was super helpful!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let patched_dive: serde_json::Value = res.json().await.unwrap();
    assert_eq!(patched_dive["userNote"], "This was super helpful!");
    assert!(patched_dive["noteUpdatedAt"].is_string());

    // Verify second revision created
    let rev2: Option<(i32, String, String, String)> = sqlx::query_as(
        "SELECT revision, body_markdown, category, user_note FROM tb_deep_dive_revisions WHERE deep_dive_id = $1 AND revision = 2"
    )
    .bind(Uuid::parse_str(&dive_a_id).unwrap())
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(rev2.is_some());
    let (revision2, body_markdown2, category2, user_note2) = rev2.unwrap();
    assert_eq!(revision2, 2);
    assert_eq!(
        body_markdown2,
        "# Light Speed explanation\n\nLight travels fast."
    );
    assert_eq!(category2, "Physics");
    assert_eq!(user_note2, "This was super helpful!");

    // 7. Search and Filtering: Create a second deep dive in a different category to test
    let q_a2_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_questions (id, owner_id, kind, prompt, payload, status, points, created_by) \
         VALUES ($1, $2, 'mc', 'What is Photosynthesis?', $3, 'live', 1, $2)",
    )
    .bind(q_a2_id)
    .bind(user_a_id)
    .bind(json!({ "options": ["conversion of light to energy", "conversion of water"], "correct_index": 0 }))
    .execute(&pool)
    .await
    .unwrap();

    let res = client
        .post(format!("{base_url}/v1/deep-dives"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .json(&json!({ "questionId": q_a2_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dive_a2_id = res.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = client
        .patch(format!("{base_url}/v1/deep-dives/{dive_a2_id}/publish"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .json(&json!({
            "bodyMarkdown": "# Photosynthesis\n\nPlants make food.",
            "status": "published",
            "category": "Biology"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // List deep dives with category filter
    let res = client
        .get(format!("{base_url}/v1/deep-dives?category=Physics"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: serde_json::Value = res.json().await.unwrap();
    let dives = list["deepDives"].as_array().unwrap();
    assert_eq!(dives.len(), 1);
    assert_eq!(dives[0]["id"].as_str().unwrap(), dive_a_id);

    // List deep dives with full-text search matching prompt
    let res = client
        .get(format!("{base_url}/v1/deep-dives?search=Photosynthesis"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: serde_json::Value = res.json().await.unwrap();
    let dives = list["deepDives"].as_array().unwrap();
    assert_eq!(dives.len(), 1);
    assert_eq!(dives[0]["id"].as_str().unwrap(), dive_a2_id);

    // List deep dives with full-text search matching body markdown (case insensitive)
    let res = client
        .get(format!("{base_url}/v1/deep-dives?search=TRAVELS"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: serde_json::Value = res.json().await.unwrap();
    let dives = list["deepDives"].as_array().unwrap();
    assert_eq!(dives.len(), 1);
    assert_eq!(dives[0]["id"].as_str().unwrap(), dive_a_id);

    // List deep dives with full-text search matching user note
    let res = client
        .get(format!("{base_url}/v1/deep-dives?search=helpful"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: serde_json::Value = res.json().await.unwrap();
    let dives = list["deepDives"].as_array().unwrap();
    assert_eq!(dives.len(), 1);
    assert_eq!(dives[0]["id"].as_str().unwrap(), dive_a_id);

    // 8. ZIP Export Validation
    let res = client
        .get(format!("{base_url}/v1/deep-dives/export"))
        .header("Authorization", format!("Bearer {auth_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("content-type").unwrap().to_str().unwrap(),
        "application/zip"
    );
    assert_eq!(
        res.headers()
            .get("content-disposition")
            .unwrap()
            .to_str()
            .unwrap(),
        "attachment; filename=\"deep-dives-export.zip\""
    );

    let zip_bytes = res.bytes().await.unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
    assert_eq!(archive.len(), 2);

    // Check first file contents
    let mut content1 = String::new();
    {
        let mut file1 = archive
            .by_name("Physics/What is the speed of light_.md")
            .unwrap();
        file1.read_to_string(&mut content1).unwrap();
    }
    assert!(content1.contains("category: Physics"));
    assert!(content1.contains("# What is the speed of light?"));
    assert!(content1.contains("Light travels fast."));
    assert!(content1.contains("## Personal Notes"));
    assert!(content1.contains("This was super helpful!"));

    // Check second file contents
    let mut content2 = String::new();
    {
        let mut file2 = archive
            .by_name("Biology/What is Photosynthesis_.md")
            .unwrap();
        file2.read_to_string(&mut content2).unwrap();
    }
    assert!(content2.contains("category: Biology"));
    assert!(content2.contains("# What is Photosynthesis?"));
    assert!(content2.contains("Plants make food."));
    assert!(!content2.contains("## Personal Notes")); // No user note was set for second dive
}
