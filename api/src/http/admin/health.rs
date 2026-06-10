use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{auth::scope::RequireScope, domain::error::ApiError, http::AppState};

use super::AdminScope;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminHealthResponse {
    pub database: String,
    pub valkey: String,
    pub users_count: i64,
    pub agents_count: i64,
    pub assessments_count: i64,
    pub sessions_count: i64,
    pub questions_count: i64,
    pub audit_log_count: i64,
    pub quota_rejections_total: i64,
}

/// GET /v1/admin/health — retrieve cluster system & health metrics
#[utoipa::path(
    get,
    path = "/v1/admin/health",
    responses(
        (status = 200, description = "Admin health metrics retrieved successfully", body = AdminHealthResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn get_admin_health(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
) -> Result<Json<AdminHealthResponse>, ApiError> {
    // 1. Check Database
    let db_res = sqlx::query("SELECT 1").execute(&state.pool).await;
    let database = if db_res.is_ok() { "ok" } else { "down" };

    // 2. Check Valkey
    let valkey_status = match state.valkey.get().await {
        Ok(mut conn) => {
            let ping_res: Result<(), _> = redis::cmd("PING").query_async(&mut *conn).await;
            if ping_res.is_ok() { "ok" } else { "degraded" }
        }
        Err(_) => "degraded",
    };

    // 3. Query table row counts
    let (
        users_count,
        agents_count,
        assessments_count,
        sessions_count,
        questions_count,
        audit_log_count,
    ) = if database == "ok" {
        let mut conn = state
            .pool
            .acquire()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
        crate::http::db::set_rls_guc(&mut conn, admin.0.user.id, true).await?;

        let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_users")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);
        let agents: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_agents")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);
        let assessments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_assessments")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);
        let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_sessions")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);
        let questions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_questions")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);
        let audit: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_audit_log")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(0);
        (users, agents, assessments, sessions, questions, audit)
    } else {
        (0, 0, 0, 0, 0, 0)
    };

    // 4. Retrieve recent quota-rejection count from Valkey
    let quota_rejections_total = if valkey_status == "ok" {
        match state.valkey.get().await {
            Ok(mut conn) => {
                let val: Option<String> = redis::cmd("GET")
                    .arg("ame:quota_rejections_count")
                    .query_async(&mut *conn)
                    .await
                    .unwrap_or(None);
                val.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0)
            }
            Err(_) => 0,
        }
    } else {
        0
    };

    Ok(Json(AdminHealthResponse {
        database: database.to_string(),
        valkey: valkey_status.to_string(),
        users_count,
        agents_count,
        assessments_count,
        sessions_count,
        questions_count,
        audit_log_count,
        quota_rejections_total,
    }))
}
