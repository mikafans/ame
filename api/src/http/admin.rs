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

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenEntry {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    // tb_users.email is nullable (agent sub-accounts may have none), so this
    // must be optional — decoding a NULL into String would panic the handler.
    pub owner_email: Option<String>,
    pub owner_display_name: Option<String>,
    pub owner_role: String,
    pub status: String,
    pub scopes: Vec<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub revoked_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub expires_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListTokensResponse {
    pub tokens: Vec<TokenEntry>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ListTokensQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub q: Option<String>,
    pub status: Option<String>,
    pub role: Option<String>,
    pub owner_id: Option<Uuid>,
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
        "SELECT u.id, u.owner_user_id, u.email, u.display_name, u.role, COALESCE(o.plan, u.plan) as plan, u.created_at, u.deactivated_at FROM tb_users u LEFT JOIN tb_users o ON u.owner_user_id = o.id",
    );
    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        users_qb.push(" WHERE u.email ILIKE ");
        users_qb.push_bind(search_pat.clone());
        users_qb.push(" OR u.display_name ILIKE ");
        users_qb.push_bind(search_pat);
    }
    users_qb.push(" ORDER BY u.created_at DESC, u.id DESC LIMIT ");
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
                deactivated_at: r.get("deactivated_at"),
            }
        })
        .collect();

    Ok(Json(ListUsersResponse { users, total }))
}

/// GET /v1/admin/tokens — list all API tokens with filtering
#[utoipa::path(
    get,
    path = "/v1/admin/tokens",
    params(ListTokensQuery),
    responses(
        (status = 200, description = "Token list successfully retrieved", body = ListTokensResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn list_tokens(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
    Query(query): Query<ListTokensQuery>,
) -> Result<Json<ListTokensResponse>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * page_size;

    // 1. Get total count
    let mut count_qb = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM tb_api_tokens t \
         LEFT JOIN tb_users u ON u.id = t.user_id \
         LEFT JOIN tb_agents a ON a.id = t.agent_id \
         LEFT JOIN tb_users o ON a.owner_user_id = o.id",
    );
    let mut has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        count_qb.push(" WHERE (t.name ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR u.email ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR o.email ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR u.display_name ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR a.label ILIKE ");
        count_qb.push_bind(search_pat);
        count_qb.push(")");
        has_where = true;
    }

    if let Some(role) = query.role.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb
                .push(" AND (CASE WHEN t.agent_id IS NOT NULL THEN 'agent' ELSE u.role END) = ");
        } else {
            count_qb
                .push(" WHERE (CASE WHEN t.agent_id IS NOT NULL THEN 'agent' ELSE u.role END) = ");
            has_where = true;
        }
        count_qb.push_bind(role);
    }

    if let Some(owner_id) = query.owner_id {
        if has_where {
            count_qb.push(" AND COALESCE(t.user_id, t.agent_id) = ");
        } else {
            count_qb.push(" WHERE COALESCE(t.user_id, t.agent_id) = ");
            has_where = true;
        }
        count_qb.push_bind(owner_id);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb.push(" AND ");
        } else {
            count_qb.push(" WHERE ");
        }
        match status {
            "active" => {
                count_qb.push("(t.revoked_at IS NULL AND t.expires_at > now())");
            }
            "revoked" => {
                count_qb.push("(t.revoked_at IS NOT NULL)");
            }
            "expired" => {
                count_qb.push("(t.revoked_at IS NULL AND t.expires_at <= now())");
            }
            _ => {} // ignore invalid status
        }
    }

    let total: i64 = count_qb
        .build_query_as::<(i64,)>()
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .0;

    // 2. Get paginated tokens
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT t.id, t.name, COALESCE(t.user_id, t.agent_id) as user_id, \
         CASE WHEN t.agent_id IS NOT NULL THEN o.email ELSE u.email END as email, \
         CASE WHEN t.agent_id IS NOT NULL THEN a.label ELSE u.display_name END as display_name, \
         CASE WHEN t.agent_id IS NOT NULL THEN 'agent' ELSE u.role END as role, \
         t.scopes, t.last_used_at, t.revoked_at, t.expires_at, t.created_at \
         FROM tb_api_tokens t \
         LEFT JOIN tb_users u ON u.id = t.user_id \
         LEFT JOIN tb_agents a ON a.id = t.agent_id \
         LEFT JOIN tb_users o ON a.owner_user_id = o.id",
    );

    has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        qb.push(" WHERE (t.name ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR u.email ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR o.email ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR u.display_name ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR a.label ILIKE ");
        qb.push_bind(search_pat);
        qb.push(")");
        has_where = true;
    }

    if let Some(role) = query.role.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND (CASE WHEN t.agent_id IS NOT NULL THEN 'agent' ELSE u.role END) = ");
        } else {
            qb.push(" WHERE (CASE WHEN t.agent_id IS NOT NULL THEN 'agent' ELSE u.role END) = ");
            has_where = true;
        }
        qb.push_bind(role);
    }

    if let Some(owner_id) = query.owner_id {
        if has_where {
            qb.push(" AND COALESCE(t.user_id, t.agent_id) = ");
        } else {
            qb.push(" WHERE COALESCE(t.user_id, t.agent_id) = ");
            has_where = true;
        }
        qb.push_bind(owner_id);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND ");
        } else {
            qb.push(" WHERE ");
        }
        match status {
            "active" => {
                qb.push("(t.revoked_at IS NULL AND t.expires_at > now())");
            }
            "revoked" => {
                qb.push("(t.revoked_at IS NOT NULL)");
            }
            "expired" => {
                qb.push("(t.revoked_at IS NULL AND t.expires_at <= now())");
            }
            _ => {} // ignore invalid status
        }
    }

    qb.push(" ORDER BY t.created_at DESC, t.id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let now = time::OffsetDateTime::now_utc();
    let tokens = rows
        .iter()
        .map(|r| {
            let role_str: String = r.get("role");
            let scopes: Vec<String> = r.get("scopes");
            let revoked_at: Option<time::OffsetDateTime> = r.get("revoked_at");
            let expires_at: time::OffsetDateTime = r.get("expires_at");
            let status = if revoked_at.is_some() {
                "revoked".to_string()
            } else if expires_at <= now {
                "expired".to_string()
            } else {
                "active".to_string()
            };

            TokenEntry {
                id: r.get("id"),
                name: r.get("name"),
                owner_id: r.get("user_id"),
                owner_email: r.get("email"),
                owner_display_name: r.get("display_name"),
                owner_role: role_str,
                status,
                scopes,
                last_used_at: r.get("last_used_at"),
                revoked_at,
                expires_at,
                created_at: r.get("created_at"),
            }
        })
        .collect();

    Ok(Json(ListTokensResponse {
        tokens,
        total,
        page,
        page_size,
    }))
}

/// PATCH /v1/admin/users/{id} — toggle user plan or disable/deactivate user
#[utoipa::path(
    patch,
    path = "/v1/admin/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
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
    // 1. Verify user exists and fetch current role/status
    let current_user: Option<(String, bool)> = sqlx::query_as(
        "SELECT role, (deactivated_at IS NOT NULL) as disabled FROM tb_users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let (current_role, current_disabled) = match current_user {
        Some(u) => u,
        None => return Err(ApiError::NotFound { resource: "user" }),
    };

    let target_is_self = admin.0.user.id == user_id;

    // A. Check self-demotion
    if target_is_self && body.role.as_deref().filter(|r| *r != "admin").is_some() {
        crate::audit::audit(
            state.pool.clone(),
            Some(admin.0.user.id),
            "user.patch_rejected",
            Some("user"),
            Some(user_id),
            serde_json::json!({ "reason": "self_demotion_forbidden" }),
        );
        return Err(ApiError::Validation(vec![FieldError {
            field: "role".into(),
            message: "cannot demote your own admin role".into(),
        }]));
    }

    // B. Check self-disable
    if target_is_self && body.disabled.unwrap_or(false) {
        crate::audit::audit(
            state.pool.clone(),
            Some(admin.0.user.id),
            "user.patch_rejected",
            Some("user"),
            Some(user_id),
            serde_json::json!({ "reason": "self_disable_forbidden" }),
        );
        return Err(ApiError::Validation(vec![FieldError {
            field: "disabled".into(),
            message: "cannot disable your own account".into(),
        }]));
    }

    // C. Check last active admin protection
    let is_active_admin = current_role == "admin" && !current_disabled;
    let changing_to_non_admin = body.role.as_ref().map(|r| r != "admin").unwrap_or(false);
    let changing_to_disabled = body.disabled.unwrap_or(false);

    if is_active_admin && (changing_to_non_admin || changing_to_disabled) {
        let active_admin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tb_users WHERE role = 'admin' AND deactivated_at IS NULL",
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        if active_admin_count <= 1 {
            let field = if changing_to_disabled {
                "disabled"
            } else {
                "role"
            };
            crate::audit::audit(
                state.pool.clone(),
                Some(admin.0.user.id),
                "user.patch_rejected",
                Some("user"),
                Some(user_id),
                serde_json::json!({ "reason": "last_admin_protection" }),
            );
            return Err(ApiError::Validation(vec![FieldError {
                field: field.into(),
                message: "cannot demote or disable the last remaining admin".into(),
            }]));
        }
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

        if current_role == "admin" && role != "admin" {
            sqlx::query(
                "UPDATE tb_api_tokens SET revoked_at = now()
                 WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
        }

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

    // Invalidate caches: if role or disabled status changed, force re-login (delete sessions);
    // otherwise (plan-only change), just bust the cache to pick up the new plan on next load.
    if body.role.is_some() || body.disabled.is_some() {
        crate::auth::extractor::invalidate_user_tokens(&state.pool, &state.valkey, user_id).await;
    } else {
        crate::auth::extractor::invalidate_user_caches(&state.pool, &state.valkey, user_id).await;
    }

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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct AdminListAssessmentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
    pub mode: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminAssessmentEntry {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub mode: String,
    pub created_by: Uuid,
    pub created_by_email: Option<String>,
    pub objectives: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub deleted_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminListAssessmentsResponse {
    pub assessments: Vec<AdminAssessmentEntry>,
    pub total: i64,
}

/// GET /v1/admin/assessments — list all assessments on the platform
#[utoipa::path(
    get,
    path = "/v1/admin/assessments",
    params(AdminListAssessmentsQuery),
    responses(
        (status = 200, description = "Assessments retrieved successfully", body = AdminListAssessmentsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn list_assessments_admin(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
    Query(query): Query<AdminListAssessmentsQuery>,
) -> Result<Json<AdminListAssessmentsResponse>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut count_qb = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM tb_assessments a
         LEFT JOIN tb_users u ON a.created_by = u.id",
    );
    let mut has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        count_qb.push(" WHERE (a.title ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR a.description ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR u.email ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR u.display_name ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR EXISTS (SELECT 1 FROM tb_users ow WHERE ow.id = u.owner_user_id AND ow.email ILIKE ");
        count_qb.push_bind(search_pat);
        count_qb.push("))");
        has_where = true;
    }

    if let Some(mode) = query.mode.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb.push(" AND a.mode = ");
        } else {
            count_qb.push(" WHERE a.mode = ");
            has_where = true;
        }
        count_qb.push_bind(mode);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb.push(" AND a.status = ");
        } else {
            count_qb.push(" WHERE a.status = ");
        }
        count_qb.push_bind(status);
    }

    let total: i64 = count_qb
        .build_query_as::<(i64,)>()
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .0;

    let mut qb = sqlx::QueryBuilder::new(
        "SELECT a.id, a.title, a.description, a.status, a.mode, a.created_by,
                COALESCE(u.email, (SELECT email FROM tb_users WHERE id = u.owner_user_id), u.display_name) as created_by_email,
                a.objectives, a.created_at, a.deleted_at
         FROM tb_assessments a
         LEFT JOIN tb_users u ON a.created_by = u.id"
    );

    let mut has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        qb.push(" WHERE (a.title ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR a.description ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR u.email ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR u.display_name ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR EXISTS (SELECT 1 FROM tb_users ow WHERE ow.id = u.owner_user_id AND ow.email ILIKE ");
        qb.push_bind(search_pat);
        qb.push("))");
        has_where = true;
    }

    if let Some(mode) = query.mode.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND a.mode = ");
        } else {
            qb.push(" WHERE a.mode = ");
            has_where = true;
        }
        qb.push_bind(mode);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND a.status = ");
        } else {
            qb.push(" WHERE a.status = ");
        }
        qb.push_bind(status);
    }

    qb.push(" ORDER BY a.created_at DESC, a.id DESC LIMIT ");
    qb.push_bind(limit);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let assessments = rows
        .into_iter()
        .map(|r| {
            let status_str: String = r.get("status");
            let mode_str: String = r.get("mode");
            AdminAssessmentEntry {
                id: r.get("id"),
                title: r.get("title"),
                description: r.get("description"),
                status: status_str,
                mode: mode_str,
                created_by: r.get("created_by"),
                created_by_email: r.get("created_by_email"),
                objectives: r.get("objectives"),
                created_at: r.get("created_at"),
                deleted_at: r.get("deleted_at"),
            }
        })
        .collect();

    Ok(Json(AdminListAssessmentsResponse { assessments, total }))
}

/// DELETE /v1/admin/assessments/{id} — soft-delete any assessment (moderation)
#[utoipa::path(
    delete,
    path = "/v1/admin/assessments/{id}",
    params(("id" = Uuid, Path, description = "Assessment ID")),
    responses(
        (status = 204, description = "Assessment deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "Assessment not found"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn delete_assessment_admin(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let res = sqlx::query(
        "UPDATE tb_assessments SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "assessment",
        });
    }

    crate::audit::audit(
        state.pool.clone(),
        Some(admin.0.user.id),
        "assessment.delete_admin",
        Some("assessment"),
        Some(id),
        serde_json::json!({}),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// POST /v1/admin/assessments/{id}/restore — restore a soft-deleted assessment
#[utoipa::path(
    post,
    path = "/v1/admin/assessments/{id}/restore",
    params(("id" = Uuid, Path, description = "Assessment ID")),
    responses(
        (status = 204, description = "Assessment restored successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "Assessment not found or not deleted"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn restore_assessment_admin(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let res = sqlx::query(
        "UPDATE tb_assessments SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "assessment",
        });
    }

    crate::audit::audit(
        state.pool.clone(),
        Some(admin.0.user.id),
        "assessment.restore_admin",
        Some("assessment"),
        Some(id),
        serde_json::json!({}),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /v1/admin/tokens/{id} — revoke an API token (idempotent)
#[utoipa::path(
    delete,
    path = "/v1/admin/tokens/{id}",
    params(("id" = Uuid, Path, description = "Token ID")),
    responses(
        (status = 204, description = "Token revoked successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "Token not found"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn delete_token_admin(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. Fetch token details (owner, name, status) for auditing and existence check.
    let token_info: Option<(Uuid, String, Option<time::OffsetDateTime>)> = sqlx::query_as(
        "SELECT COALESCE(user_id, agent_id) as owner_id, name, revoked_at \
         FROM tb_api_tokens WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let (owner_id, name, revoked_at) = match token_info {
        Some(info) => info,
        None => {
            return Err(ApiError::NotFound {
                resource: "api_token",
            });
        }
    };

    // If it's already revoked, return 204 immediately (idempotent)
    if revoked_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }

    // Revoke the token
    sqlx::query("UPDATE tb_api_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Bust the Valkey token cache so the revoke takes effect immediately,
    // not after the 30s cache TTL — otherwise a leaked token keeps working.
    crate::auth::extractor::invalidate_token(&state.valkey, id).await;

    crate::audit::audit(
        state.pool.clone(),
        Some(admin.0.user.id),
        "token.revoke_admin",
        Some("api_token"),
        Some(id),
        serde_json::json!({
            "owner_id": owner_id,
            "name": name,
        }),
    );

    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_guard_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ApiError> {
    let auth = req
        .extensions()
        .get::<crate::auth::extractor::AuthenticatedUser>()
        .ok_or(ApiError::Unauthorized)?;

    if !auth.token_scopes.contains(&Scope::Admin) {
        return Err(ApiError::ScopeRequired(std::borrow::Cow::Borrowed(
            Scope::Admin.as_str(),
        )));
    }

    Ok(next.run(req).await)
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/admin/users", get(list_users))
        .route(
            "/v1/admin/users/{id}",
            axum::routing::patch(patch_user_admin),
        )
        .route("/v1/admin/tokens", get(list_tokens))
        .route(
            "/v1/admin/tokens/{id}",
            axum::routing::delete(delete_token_admin),
        )
        .route("/v1/admin/audit", get(list_audit_logs))
        .route("/v1/admin/health", get(get_admin_health))
        .route("/v1/admin/assessments", get(list_assessments_admin))
        .route(
            "/v1/admin/assessments/{id}",
            axum::routing::delete(delete_assessment_admin),
        )
        .route(
            "/v1/admin/assessments/{id}/restore",
            axum::routing::post(restore_assessment_admin),
        )
        .layer(axum::middleware::from_fn(admin_guard_middleware))
        .with_state(state)
}
