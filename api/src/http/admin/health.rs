use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{auth::admin::RequireAdmin, domain::error::ApiError, http::AppState};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminHealthResponse {
    pub database: String,
    pub valkey: String,
    pub users_count: i64,
    pub journeys_count: i64,
    pub activities_count: i64,
    pub sessions_count: i64,
    pub audit_log_count: i64,
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/health",
    responses(
        (status = 200, description = "Current platform health", body = AdminHealthResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Administrator role required")
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn get_admin_health(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<Json<AdminHealthResponse>, ApiError> {
    let database = if sqlx::query("SELECT 1").execute(&state.pool).await.is_ok() {
        "ok"
    } else {
        "down"
    };
    let valkey = match state.valkey.get().await {
        Ok(mut connection) => {
            let result: Result<(), _> = redis::cmd("PING").query_async(&mut *connection).await;
            if result.is_ok() { "ok" } else { "degraded" }
        }
        Err(_) => "degraded",
    };

    let users_count = count(&state, "tb_users").await;
    let journeys_count = count(&state, "tb_learning_journeys").await;
    let activities_count = count(&state, "tb_activities").await;
    let sessions_count = count(&state, "tb_learning_sessions").await;
    let audit_log_count = count(&state, "tb_audit_log").await;

    Ok(Json(AdminHealthResponse {
        database: database.into(),
        valkey: valkey.into(),
        users_count,
        journeys_count,
        activities_count,
        sessions_count,
        audit_log_count,
    }))
}

async fn count(state: &AppState, table: &str) -> i64 {
    // Table names are fixed above; they are not user input.
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0)
}
