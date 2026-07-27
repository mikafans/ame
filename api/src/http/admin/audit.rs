use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::admin::RequireAdmin, domain::error::ApiError, http::AppState};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub target_email: Option<String>,
    pub target_name: Option<String>,
    pub metadata: serde_json::Value,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogsResponse {
    pub logs: Vec<AuditLogEntry>,
    pub total: i64,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub action: Option<String>,
    pub actor_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
}

/// GET /api/v1/admin/audit — retrieve append-only audit trail logs
#[utoipa::path(
    get,
    path = "/api/v1/admin/audit",
    params(ListAuditLogsQuery),
    responses(
        (status = 200, description = "Audit trail successfully retrieved", body = ListAuditLogsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn list_audit_logs(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Query(query): Query<ListAuditLogsQuery>,
) -> Result<Json<ListAuditLogsResponse>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let (records, total) = ame_platform_postgres::audit_query::list(
        &state.pool,
        limit,
        offset,
        query.action.as_deref(),
        query.actor_id,
        query.target_id,
    )
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let logs = records
        .into_iter()
        .map(|record| AuditLogEntry {
            id: record.id,
            actor_user_id: record.actor_user_id,
            actor_email: record.actor_email,
            actor_name: record.actor_name,
            action: record.action,
            target_type: record.target_type,
            target_id: record.target_id,
            target_email: record.target_email,
            target_name: record.target_name,
            metadata: record.metadata,
            created_at: record.created_at,
        })
        .collect();

    Ok(Json(ListAuditLogsResponse { logs, total }))
}
