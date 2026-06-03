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

#[tokio::test]
async fn test_admin_flow_and_audit() {
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

    // 1. Create a regular human user and token
    let reg_user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Regular Human', 'user', 'free')",
    )
    .bind(reg_user_id)
    .bind(format!("reg-{}@example.com", reg_user_id))
    .execute(&pool)
    .await
    .unwrap();

    let reg_token_id = Uuid::now_v7();
    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'reg-key', $3, $4::text[])",
    )
    .bind(reg_token_id)
    .bind(reg_user_id)
    .bind(&hash)
    .bind(vec![
        "assessment.read".to_string(),
        "assessment.write".to_string(),
        "attempt.read".to_string(),
        "attempt.write".to_string(),
    ])
    .execute(&pool)
    .await
    .unwrap();
    let reg_auth = format!("{reg_token_id}_{secret}");

    // 2. Assert regular human cannot access admin endpoints
    let res = client
        .get(format!("{base_url}/v1/admin/users"))
        .header("Authorization", format!("Bearer {reg_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .patch(format!("{base_url}/v1/admin/users/{reg_user_id}"))
        .header("Authorization", format!("Bearer {reg_auth}"))
        .json(&json!({"plan": "premium"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .get(format!("{base_url}/v1/admin/audit"))
        .header("Authorization", format!("Bearer {reg_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .get(format!("{base_url}/v1/admin/health"))
        .header("Authorization", format!("Bearer {reg_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Create an admin user and token
    let admin_user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin Human', 'admin', 'premium')",
    )
    .bind(admin_user_id)
    .bind(format!("admin-{}@example.com", admin_user_id))
    .execute(&pool)
    .await
    .unwrap();

    let admin_token_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_user_id)
    .bind(&hash)
    .bind(vec!["admin".to_string(), "assessment.read".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{secret}");

    // 4. Admin lists users
    let res = client
        .get(format!("{base_url}/v1/admin/users"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let users = body["users"].as_array().unwrap();
    assert!(users.iter().any(|u| u["id"] == reg_user_id.to_string()));
    assert!(users.iter().any(|u| u["id"] == admin_user_id.to_string()));

    // 5. Admin updates user plan to premium
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{reg_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"plan": "premium"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Verify in database
    let plan: String = sqlx::query_scalar("SELECT plan FROM tb_users WHERE id = $1")
        .bind(reg_user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(plan, "premium");

    // Admin updates user plan back to free
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{reg_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"plan": "free"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let plan: String = sqlx::query_scalar("SELECT plan FROM tb_users WHERE id = $1")
        .bind(reg_user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(plan, "free");

    // 6. Revoke/Deactivate Flow: Admin disables regular user
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{reg_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"disabled": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Verify that the regular user is now rejected
    let res = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {reg_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Admin re-enables regular user
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{reg_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"disabled": false}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Create a new token for the re-enabled user and check it works
    let reg_token_id2 = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'reg-key2', $3, $4::text[])",
    )
    .bind(reg_token_id2)
    .bind(reg_user_id)
    .bind(&hash)
    .bind(vec!["assessment.read".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let reg_auth2 = format!("{reg_token_id2}_{secret}");

    let res = client
        .get(format!("{base_url}/v1/me"))
        .header("Authorization", format!("Bearer {reg_auth2}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Admin updates regular user's role to admin
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{reg_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"role": "admin"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 7. Verify the Audit Trail
    // Wait slightly to let Tokios background spawns finish insertion
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let res = client
        .get(format!("{base_url}/v1/admin/audit"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let all_logs = body["logs"].as_array().unwrap();

    // Filter to only include logs relevant to our test run users
    let logs: Vec<&serde_json::Value> = all_logs
        .iter()
        .filter(|log| {
            let actor = log["actorUserId"].as_str();
            let target = log["targetId"].as_str();
            actor == Some(&admin_user_id.to_string())
                || actor == Some(&reg_user_id.to_string())
                || target == Some(&reg_user_id.to_string())
        })
        .collect();

    // The logs should be in newest-first order.
    // Order of audit events we triggered:
    // 1. user.update_plan (to premium)
    // 2. user.update_plan (to free)
    // 3. user.disable (disabling the user by admin)
    // 4. user.enable (re-enabling the user by admin)
    // 5. user.update_role (updating the user's role to admin)
    // Therefore, newest first: user.update_role, user.enable, user.disable, user.update_plan, user.update_plan

    assert_eq!(
        logs.len(),
        5,
        "Expected exactly 5 audit logs for this test run. Found: {logs:#?}"
    );

    // Validate user.update_role
    assert_eq!(logs[0]["action"], "user.update_role");
    assert_eq!(logs[0]["actorUserId"], admin_user_id.to_string());
    assert_eq!(logs[0]["targetType"], "user");
    assert_eq!(logs[0]["targetId"], reg_user_id.to_string());
    assert_eq!(logs[0]["metadata"]["role"], "admin");

    // Validate user.enable
    assert_eq!(logs[1]["action"], "user.enable");
    assert_eq!(logs[1]["actorUserId"], admin_user_id.to_string());
    assert_eq!(logs[1]["targetType"], "user");
    assert_eq!(logs[1]["targetId"], reg_user_id.to_string());
    assert_eq!(logs[1]["metadata"]["disabled"], false);

    // Validate user.disable
    assert_eq!(logs[2]["action"], "user.disable");
    assert_eq!(logs[2]["actorUserId"], admin_user_id.to_string());
    assert_eq!(logs[2]["targetType"], "user");
    assert_eq!(logs[2]["targetId"], reg_user_id.to_string());
    assert_eq!(logs[2]["metadata"]["disabled"], true);

    // Validate user.update_plan (to free)
    assert_eq!(logs[3]["action"], "user.update_plan");
    assert_eq!(logs[3]["actorUserId"], admin_user_id.to_string());
    assert_eq!(logs[3]["targetType"], "user");
    assert_eq!(logs[3]["targetId"], reg_user_id.to_string());
    assert_eq!(logs[3]["metadata"]["plan"], "free");

    // Validate user.update_plan (to premium)
    assert_eq!(logs[4]["action"], "user.update_plan");
    assert_eq!(logs[4]["actorUserId"], admin_user_id.to_string());
    assert_eq!(logs[4]["targetType"], "user");
    assert_eq!(logs[4]["targetId"], reg_user_id.to_string());
    assert_eq!(logs[4]["metadata"]["plan"], "premium");

    // 8. Test Pagination and Filtering on Users
    let res = client
        .get(format!("{base_url}/v1/admin/users?limit=1"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["users"].as_array().unwrap().len(), 1);
    assert!(body["total"].as_i64().unwrap() >= 2);

    let res = client
        .get(format!("{base_url}/v1/admin/users?q=reg-{}", reg_user_id))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["total"].as_i64().unwrap(), 1);
    assert_eq!(body["users"][0]["displayName"], "Regular Human");

    // 9. Test Pagination and Filtering on Audit Logs
    let res = client
        .get(format!(
            "{base_url}/v1/admin/audit?limit=2&actorId={admin_user_id}"
        ))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["logs"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"].as_i64().unwrap(), 5);

    let res = client
        .get(format!(
            "{base_url}/v1/admin/audit?action=user.update_role&actorId={admin_user_id}"
        ))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["total"].as_i64().unwrap(), 1);
    assert_eq!(body["logs"][0]["action"], "user.update_role");

    // 10. Test Admin Health
    let res = client
        .get(format!("{base_url}/v1/admin/health"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let health: serde_json::Value = res.json().await.unwrap();
    assert_eq!(health["database"], "ok");
    assert!(health["usersCount"].as_i64().unwrap() >= 2);
    assert!(health["auditLogCount"].as_i64().unwrap() >= 5);

    // 11. Test self-demotion protection
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{admin_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"role": "user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], "validation_failed");

    // 12. Test self-disable protection
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{admin_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"disabled": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], "validation_failed");

    // Demote reg_user_id back to user (succeeds because active_admin_count is 2)
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{reg_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"role": "user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Now admin_user_id is the last remaining active admin
    // Try to demote admin_user_id (fails with validation error)
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{admin_user_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"role": "user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_admin_health() {
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

    // 1. Create a non-admin user token
    let user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'User', 'user', 'free')",
    )
    .bind(user_id)
    .bind(format!("user-{}@example.com", user_id))
    .execute(&pool)
    .await
    .unwrap();

    let user_token_id = Uuid::now_v7();
    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'user-key', $3, $4::text[])",
    )
    .bind(user_token_id)
    .bind(user_id)
    .bind(&hash)
    .bind(vec!["assessment.read".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let user_auth = format!("{user_token_id}_{secret}");

    // 2. Non-admin cannot access health endpoint
    let res = client
        .get(format!("{base_url}/v1/admin/health"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Create an admin user token
    let admin_user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin', 'admin', 'premium')",
    )
    .bind(admin_user_id)
    .bind(format!("admin-{}@example.com", admin_user_id))
    .execute(&pool)
    .await
    .unwrap();

    let admin_token_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_user_id)
    .bind(&hash)
    .bind(vec!["admin".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{secret}");

    // 4. Admin can access health endpoint
    let res = client
        .get(format!("{base_url}/v1/admin/health"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let health: serde_json::Value = res.json().await.unwrap();
    assert_eq!(health["database"], "ok");
    assert!(health["usersCount"].as_i64().is_some());
    assert!(health["assessmentsCount"].as_i64().is_some());
    assert!(health["sessionsCount"].as_i64().is_some());
    assert!(health["questionsCount"].as_i64().is_some());
    assert!(health["auditLogCount"].as_i64().is_some());
    assert!(health["quotaRejectionsTotal"].as_i64().is_some());
}

#[tokio::test]
async fn test_admin_user_filters() {
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

    // 1. Create admin and seed test users
    let admin_user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin', 'admin', 'premium')",
    )
    .bind(admin_user_id)
    .bind(format!("admin-{}@example.com", admin_user_id))
    .execute(&pool)
    .await
    .unwrap();

    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    let admin_token_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_user_id)
    .bind(&hash)
    .bind(vec!["admin".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{secret}");

    // Clean up existing test users first to avoid unique key violations
    sqlx::query("DELETE FROM tb_users WHERE email IN ('alice@example.com', 'bob@example.com', 'charlie@another.com')")
        .execute(&pool)
        .await
        .unwrap();

    // 2. Seed multiple test users with specific patterns
    let alice_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Alice Smith', 'user', 'free')",
    )
    .bind(alice_id)
    .bind("alice@example.com")
    .execute(&pool)
    .await
    .unwrap();

    let bob_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Bob Jones', 'user', 'free')",
    )
    .bind(bob_id)
    .bind("bob@example.com")
    .execute(&pool)
    .await
    .unwrap();

    let charlie_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Charlie Brown', 'user', 'premium')",
    )
    .bind(charlie_id)
    .bind("charlie@another.com")
    .execute(&pool)
    .await
    .unwrap();

    // 3. Test substring search by email
    let res = client
        .get(format!("{base_url}/v1/admin/users?q=alice"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let users = body["users"].as_array().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["email"], "alice@example.com");
    assert_eq!(body["total"].as_i64().unwrap(), 1);

    // 4. Test substring search by display name
    let res = client
        .get(format!("{base_url}/v1/admin/users?q=Jones"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let users = body["users"].as_array().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["displayName"], "Bob Jones");

    // 5. Test limit parameter
    let res = client
        .get(format!("{base_url}/v1/admin/users?limit=1"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let users = body["users"].as_array().unwrap();
    assert_eq!(users.len(), 1);
    assert!(body["total"].as_i64().unwrap() >= 3);

    // 6. Test offset parameter
    let res = client
        .get(format!("{base_url}/v1/admin/users?limit=1&offset=0"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body1: serde_json::Value = res.json().await.unwrap();
    let user1 = &body1["users"][0];

    let res = client
        .get(format!("{base_url}/v1/admin/users?limit=1&offset=1"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body2: serde_json::Value = res.json().await.unwrap();
    let user2 = &body2["users"][0];

    // Users should be different (ordered by created_at DESC)
    assert_ne!(user1["id"], user2["id"]);
}

#[tokio::test]
async fn test_admin_audit_filters() {
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

    // 1. Create admin
    let admin_user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin', 'admin', 'premium')",
    )
    .bind(admin_user_id)
    .bind(format!("admin-{}@example.com", admin_user_id))
    .execute(&pool)
    .await
    .unwrap();

    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    let admin_token_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_user_id)
    .bind(&hash)
    .bind(vec!["admin".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{secret}");

    // 2. Create two regular users
    let user1_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'User 1', 'user', 'free')",
    )
    .bind(user1_id)
    .bind(format!("user1-{}@example.com", user1_id))
    .execute(&pool)
    .await
    .unwrap();

    let user2_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'User 2', 'user', 'free')",
    )
    .bind(user2_id)
    .bind(format!("user2-{}@example.com", user2_id))
    .execute(&pool)
    .await
    .unwrap();

    // 3. Admin updates both users' plans to generate audit logs
    let res = client
        .patch(format!("{base_url}/v1/admin/users/{user1_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"plan": "premium"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = client
        .patch(format!("{base_url}/v1/admin/users/{user2_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .json(&json!({"plan": "premium"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Wait for audit logs to be inserted
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // 4. Test action filter
    let res = client
        .get(format!("{base_url}/v1/admin/audit?action=user.update_plan"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let logs = body["logs"].as_array().unwrap();

    // Should have at least 2 logs (user1 and user2 plan updates)
    assert!(logs.len() >= 2);
    assert!(logs.iter().all(|log| log["action"] == "user.update_plan"));

    // 5. Test target_id filter
    let res = client
        .get(format!("{base_url}/v1/admin/audit?targetId={user1_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let logs = body["logs"].as_array().unwrap();

    // All logs should be for user1
    assert!(
        logs.iter()
            .all(|log| log["targetId"] == user1_id.to_string())
    );
    assert!(body["total"].as_i64().unwrap() >= 1);
}

#[tokio::test]
async fn test_admin_assessments_moderation() {
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

    // 1. Create admin and regular user
    let admin_user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin', 'admin', 'premium')",
    )
    .bind(admin_user_id)
    .bind(format!("admin-{}@example.com", admin_user_id))
    .execute(&pool)
    .await
    .unwrap();

    let secret = "supersecret";
    let hash = ame_api::auth::token::hash_secret(secret);
    let admin_token_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_user_id)
    .bind(&hash)
    .bind(vec!["admin".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{secret}");

    let user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'User', 'user', 'free')",
    )
    .bind(user_id)
    .bind(format!("user-{}@example.com", user_id))
    .execute(&pool)
    .await
    .unwrap();

    let user_token_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'user-key', $3, $4::text[])",
    )
    .bind(user_token_id)
    .bind(user_id)
    .bind(&hash)
    .bind(vec!["assessment.read".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let user_auth = format!("{user_token_id}_{secret}");

    // 2. Seed an assessment
    let assessment_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_assessments (id, title, description, mode, status, created_by, objectives)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(assessment_id)
    .bind("Test Assessment")
    .bind("A test assessment for moderation")
    .bind("practice")
    .bind("active")
    .bind(user_id)
    .bind(vec!["objective1".to_string(), "objective2".to_string()])
    .execute(&pool)
    .await
    .unwrap();

    // 3. Non-admin cannot list assessments
    let res = client
        .get(format!("{base_url}/v1/admin/assessments"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 4. Admin can list assessments
    let res = client
        .get(format!("{base_url}/v1/admin/assessments"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let assessments = body["assessments"].as_array().unwrap();
    assert!(
        assessments
            .iter()
            .any(|a| a["id"] == assessment_id.to_string())
    );

    // 5. Non-admin cannot delete assessments
    let res = client
        .delete(format!("{base_url}/v1/admin/assessments/{assessment_id}"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 6. Admin can delete assessment
    let res = client
        .delete(format!("{base_url}/v1/admin/assessments/{assessment_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 7. Assessment should still be in admin list but with deletedAt set (soft delete)
    let res = client
        .get(format!("{base_url}/v1/admin/assessments"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let assessments = body["assessments"].as_array().unwrap();
    let deleted_entry = assessments
        .iter()
        .find(|a| a["id"] == assessment_id.to_string())
        .expect("Soft-deleted assessment should still appear in admin list");
    assert!(
        deleted_entry["deletedAt"].is_string(),
        "deletedAt should be set for soft-deleted assessment"
    );

    // 8. Deleting again should return 404
    let res = client
        .delete(format!("{base_url}/v1/admin/assessments/{assessment_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 9. Verify cascade: seed a session referencing the assessment, then delete
    let assessment2_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_assessments (id, title, description, mode, status, created_by, objectives)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(assessment2_id)
    .bind("Assessment with Session")
    .bind(None::<String>)
    .bind("practice")
    .bind("active")
    .bind(user_id)
    .bind(vec![] as Vec<String>)
    .execute(&pool)
    .await
    .unwrap();

    let session_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_sessions (id, user_id, assessment_id, kind, question_plan, status)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(assessment2_id)
    .bind("assessment")
    .bind(serde_json::json!({}))
    .bind("in_progress")
    .execute(&pool)
    .await
    .unwrap();

    // Verify session exists
    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(session_count, 1);

    // Delete assessment
    let res = client
        .delete(format!("{base_url}/v1/admin/assessments/{assessment2_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Verify session is still intact (soft delete = no cascade)
    // With soft delete, sessions referencing the deleted assessment should still exist
    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(session_count, 1);
}

#[tokio::test]
async fn test_admin_assessment_soft_delete() {
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

    // 1. Create admin user and token
    let admin_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Admin User', 'admin', 'free')",
    )
    .bind(admin_id)
    .bind(format!("admin-{}@example.com", admin_id))
    .execute(&pool)
    .await
    .unwrap();

    let admin_token_id = Uuid::now_v7();
    let admin_secret = "admin_secret";
    let admin_hash = ame_api::auth::token::hash_secret(admin_secret);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'admin-key', $3, $4::text[])",
    )
    .bind(admin_token_id)
    .bind(admin_id)
    .bind(&admin_hash)
    .bind(vec!["admin".to_string()])
    .execute(&pool)
    .await
    .unwrap();
    let admin_auth = format!("{admin_token_id}_{admin_secret}");

    // 2. Create a regular user and assessment
    let user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, plan)
         VALUES ($1, $2, 'Regular User', 'user', 'free')",
    )
    .bind(user_id)
    .bind(format!("user-{}@example.com", user_id))
    .execute(&pool)
    .await
    .unwrap();

    let assessment_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_assessments (id, title, description, mode, status, created_by, objectives)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(assessment_id)
    .bind("Test Assessment")
    .bind("A test assessment")
    .bind("practice")
    .bind("active")
    .bind(user_id)
    .bind(vec![] as Vec<String>)
    .execute(&pool)
    .await
    .unwrap();

    // 3. Create a session referencing the assessment
    let session_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_sessions (id, user_id, assessment_id, kind, question_plan, status)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(assessment_id)
    .bind("assessment")
    .bind(serde_json::json!({}))
    .bind("in_progress")
    .execute(&pool)
    .await
    .unwrap();

    // 4. Soft-delete the assessment via admin API
    let res = client
        .delete(format!("{base_url}/v1/admin/assessments/{assessment_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 5. Verify assessment is not in learner list (has deleted_at set)
    let user_token_id = Uuid::now_v7();
    let user_secret = "user_secret";
    let user_hash = ame_api::auth::token::hash_secret(user_secret);
    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, 'user-key', $3, $4::text[])",
    )
    .bind(user_token_id)
    .bind(user_id)
    .bind(&user_hash)
    .bind(vec![
        "assessment.read".to_string(),
        "assessment.write".to_string(),
        "attempt.read".to_string(),
        "attempt.write".to_string(),
    ])
    .execute(&pool)
    .await
    .unwrap();
    let user_auth = format!("{user_token_id}_{user_secret}");

    let res = client
        .get(format!("{base_url}/v1/assessments/{assessment_id}"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 6. Assessment still appears in admin list with deletedAt set
    let res = client
        .get(format!("{base_url}/v1/admin/assessments"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let assessments = body["assessments"].as_array().unwrap();
    let deleted_entry = assessments
        .iter()
        .find(|a| a["id"] == assessment_id.to_string())
        .expect("Assessment should appear in admin list even when deleted");
    assert!(
        deleted_entry["deletedAt"].is_string(),
        "deletedAt should be set"
    );

    // 7. Session still exists (no cascade)
    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        session_count, 1,
        "Session should still exist after soft delete"
    );

    // 8. Restore the assessment
    let res = client
        .post(format!(
            "{base_url}/v1/admin/assessments/{assessment_id}/restore"
        ))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 9. Learner can now see the assessment again
    let res = client
        .get(format!("{base_url}/v1/assessments/{assessment_id}"))
        .header("Authorization", format!("Bearer {user_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 10. Admin list shows no deletedAt
    let res = client
        .get(format!("{base_url}/v1/admin/assessments"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    let assessments = body["assessments"].as_array().unwrap();
    let restored_entry = assessments
        .iter()
        .find(|a| a["id"] == assessment_id.to_string())
        .expect("Restored assessment should appear in admin list");
    assert!(
        restored_entry["deletedAt"].is_null(),
        "deletedAt should be null after restore"
    );

    // 11. Deleting non-existent assessment returns 404
    let fake_id = Uuid::now_v7();
    let res = client
        .delete(format!("{base_url}/v1/admin/assessments/{fake_id}"))
        .header("Authorization", format!("Bearer {admin_auth}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
