use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
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

    // 1. Get total count
    let mut count_qb = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM tb_audit_log");
    let mut has_where = false;

    if let Some(action) = query.action.as_deref().filter(|s| !s.trim().is_empty()) {
        count_qb.push(" WHERE action = ");
        count_qb.push_bind(action.trim());
        has_where = true;
    }
    if let Some(actor_id) = query.actor_id {
        if has_where {
            count_qb.push(" AND actor_user_id = ");
        } else {
            count_qb.push(" WHERE actor_user_id = ");
            has_where = true;
        }
        count_qb.push_bind(actor_id);
    }
    if let Some(target_id) = query.target_id {
        if has_where {
            count_qb.push(" AND target_id = ");
        } else {
            count_qb.push(" WHERE target_id = ");
        }
        count_qb.push_bind(target_id);
    }

    let total: i64 = count_qb
        .build_query_as::<(i64,)>()
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .0;

    // 2. Get paginated audit logs
    let mut logs_qb = sqlx::QueryBuilder::new(
        "SELECT a.id, a.actor_user_id, u_actor.email as actor_email, u_actor.display_name as actor_name, \
                a.action, a.target_type, a.target_id, u_target.email as target_email, u_target.display_name as target_name, \
                a.metadata, a.created_at \
         FROM tb_audit_log a \
         LEFT JOIN tb_users u_actor ON a.actor_user_id = u_actor.id \
         LEFT JOIN tb_users u_target ON (a.target_type = 'user' AND a.target_id = u_target.id)",
    );
    let mut has_where = false;

    if let Some(action) = query.action.as_deref().filter(|s| !s.trim().is_empty()) {
        logs_qb.push(" WHERE a.action = ");
        logs_qb.push_bind(action.trim());
        has_where = true;
    }
    if let Some(actor_id) = query.actor_id {
        if has_where {
            logs_qb.push(" AND a.actor_user_id = ");
        } else {
            logs_qb.push(" WHERE a.actor_user_id = ");
            has_where = true;
        }
        logs_qb.push_bind(actor_id);
    }
    if let Some(target_id) = query.target_id {
        if has_where {
            logs_qb.push(" AND a.target_id = ");
        } else {
            logs_qb.push(" WHERE a.target_id = ");
        }
        logs_qb.push_bind(target_id);
    }

    logs_qb.push(" ORDER BY a.created_at DESC, a.id DESC LIMIT ");
    logs_qb.push_bind(limit);
    logs_qb.push(" OFFSET ");
    logs_qb.push_bind(offset);

    let rows = logs_qb
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let logs = rows
        .into_iter()
        .map(|r| AuditLogEntry {
            id: r.get("id"),
            actor_user_id: r.get("actor_user_id"),
            actor_email: r.get("actor_email"),
            actor_name: r.get("actor_name"),
            action: r.get("action"),
            target_type: r.get("target_type"),
            target_id: r.get("target_id"),
            target_email: r.get("target_email"),
            target_name: r.get("target_name"),
            metadata: r.get("metadata"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListAuditLogsResponse { logs, total }))
}
