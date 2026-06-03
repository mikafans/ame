//! Admin-only user management and moderation routes.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::scope::{RequireScope, ScopeConstraint},
    domain::{
        error::{ApiError, FieldError},
        user::{Role, Scope, User},
    },
    http::AppState,
};

pub struct AdminScope;
impl ScopeConstraint for AdminScope {
    const SCOPE: Scope = Scope::Admin;
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersResponse {
    pub users: Vec<User>,
    pub total: i64,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchUserAdminBody {
    pub plan: Option<String>,
    pub disabled: Option<bool>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
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

/// GET /v1/admin/users — list all users
#[utoipa::path(
    get,
    path = "/v1/admin/users",
    params(ListUsersQuery),
    responses(
        (status = 200, description = "User list successfully retrieved", body = ListUsersResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn list_users(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    // 1. Get total count
    let mut count_qb = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM tb_users");
    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        count_qb.push(" WHERE email ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR display_name ILIKE ");
        count_qb.push_bind(search_pat);
    }
    let total: i64 = count_qb
        .build_query_as::<(i64,)>()
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .0;

    // 2. Get paginated users
    let mut users_qb = sqlx::QueryBuilder::new(
        "SELECT id, owner_user_id, email, display_name, role, plan, created_at FROM tb_users",
    );
    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        users_qb.push(" WHERE email ILIKE ");
        users_qb.push_bind(search_pat.clone());
        users_qb.push(" OR display_name ILIKE ");
        users_qb.push_bind(search_pat);
    }
    users_qb.push(" ORDER BY created_at DESC LIMIT ");
    users_qb.push_bind(limit);
    users_qb.push(" OFFSET ");
    users_qb.push_bind(offset);

    let rows = users_qb
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let users = rows
        .iter()
        .map(|r| {
            let role_str: String = r.get("role");
            let role = match role_str.as_str() {
                "admin" => Role::Admin,
                "agent" => Role::Agent,
                _ => Role::User,
            };
            User {
                id: r.get("id"),
                owner_user_id: r.get("owner_user_id"),
                email: r.get("email"),
                display_name: r.get("display_name"),
                role,
                plan: r.get("plan"),
                created_at: r.get("created_at"),
            }
        })
        .collect();

    Ok(Json(ListUsersResponse { users, total }))
}

/// PATCH /v1/admin/users/{id} — toggle user plan or disable/deactivate user
#[utoipa::path(
    patch,
    path = "/v1/admin/users/{id}",
    request_body = PatchUserAdminBody,
    responses(
        (status = 204, description = "User successfully updated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn patch_user_admin(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<PatchUserAdminBody>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. Verify user exists
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM tb_users WHERE id = $1)")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    if !exists {
        return Err(ApiError::NotFound { resource: "user" });
    }

    // 2. Perform plan updates if requested
    if let Some(plan) = &body.plan {
        let normalized = plan.to_lowercase();
        if normalized != "free" && normalized != "premium" {
            return Err(ApiError::Validation(vec![FieldError {
                field: "plan".into(),
                message: "must be 'free' or 'premium'".into(),
            }]));
        }

        sqlx::query("UPDATE tb_users SET plan = $1 WHERE id = $2")
            .bind(&normalized)
            .bind(user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

        crate::audit::audit(
            state.pool.clone(),
            Some(admin.0.user.id),
            "user.update_plan",
            Some("user"),
            Some(user_id),
            serde_json::json!({
                "plan": normalized,
            }),
        );
    }

    // 3. Perform disable/enable if requested
    if let Some(disabled) = body.disabled {
        if disabled {
            sqlx::query("UPDATE tb_users SET deactivated_at = now() WHERE id = $1")
                .bind(user_id)
                .execute(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;

            sqlx::query(
                "UPDATE tb_api_tokens SET revoked_at = now()
                 WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

            crate::audit::audit(
                state.pool.clone(),
                Some(admin.0.user.id),
                "user.disable",
                Some("user"),
                Some(user_id),
                serde_json::json!({
                    "disabled": true,
                }),
            );
        } else {
            sqlx::query("UPDATE tb_users SET deactivated_at = NULL WHERE id = $1")
                .bind(user_id)
                .execute(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;

            crate::audit::audit(
                state.pool.clone(),
                Some(admin.0.user.id),
                "user.enable",
                Some("user"),
                Some(user_id),
                serde_json::json!({
                    "disabled": false,
                }),
            );
        }
    }

    // 4. Perform role updates if requested
    if let Some(role) = &body.role {
        let valid_roles = ["user", "admin"];
        if !valid_roles.contains(&role.as_str()) {
            return Err(ApiError::Validation(vec![FieldError {
                field: "role".into(),
                message: "must be 'user' or 'admin'".into(),
            }]));
        }

        sqlx::query("UPDATE tb_users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

        crate::audit::audit(
            state.pool.clone(),
            Some(admin.0.user.id),
            "user.update_role",
            Some("user"),
            Some(user_id),
            serde_json::json!({
                "role": role,
            }),
        );
    }

    crate::auth::extractor::invalidate_user_tokens(&state.pool, &state.valkey, user_id).await;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /v1/admin/audit — retrieve append-only audit trail logs
#[utoipa::path(
    get,
    path = "/v1/admin/audit",
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
    _admin: RequireScope<AdminScope>,
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
        "SELECT id, actor_user_id, action, target_type, target_id, metadata, created_at FROM tb_audit_log",
    );
    let mut has_where = false;

    if let Some(action) = query.action.as_deref().filter(|s| !s.trim().is_empty()) {
        logs_qb.push(" WHERE action = ");
        logs_qb.push_bind(action.trim());
        has_where = true;
    }
    if let Some(actor_id) = query.actor_id {
        if has_where {
            logs_qb.push(" AND actor_user_id = ");
        } else {
            logs_qb.push(" WHERE actor_user_id = ");
            has_where = true;
        }
        logs_qb.push_bind(actor_id);
    }
    if let Some(target_id) = query.target_id {
        if has_where {
            logs_qb.push(" AND target_id = ");
        } else {
            logs_qb.push(" WHERE target_id = ");
        }
        logs_qb.push_bind(target_id);
    }

    logs_qb.push(" ORDER BY created_at DESC LIMIT ");
    logs_qb.push_bind(limit);
    logs_qb.push(" OFFSET ");
    logs_qb.push_bind(offset);

    let rows = logs_qb
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let logs = rows
        .iter()
        .map(|r| AuditLogEntry {
            id: r.get("id"),
            actor_user_id: r.get("actor_user_id"),
            action: r.get("action"),
            target_type: r.get("target_type"),
            target_id: r.get("target_id"),
            metadata: r.get("metadata"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListAuditLogsResponse { logs, total }))
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/admin/users", get(list_users))
        .route(
            "/v1/admin/users/{id}",
            axum::routing::patch(patch_user_admin),
        )
        .route("/v1/admin/audit", get(list_audit_logs))
        .with_state(state)
}
