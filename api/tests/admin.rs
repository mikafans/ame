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
}
