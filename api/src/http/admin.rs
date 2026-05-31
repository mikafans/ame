//! Admin-only user management and moderation routes.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
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
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogsResponse {
    pub logs: Vec<AuditLogEntry>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModerateBody {
    pub quiz_id: Uuid,
}

/// GET /v1/admin/users — list all users
#[utoipa::path(
    get,
    path = "/v1/admin/users",
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
) -> Result<Json<ListUsersResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, owner_user_id, email, display_name, role, plan, created_at
         FROM tb_users
         ORDER BY created_at DESC
         LIMIT 500",
    )
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
                created_at: r.get("created_at"),
            }
        })
        .collect();

    Ok(Json(ListUsersResponse { users }))
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

    Ok(StatusCode::NO_CONTENT)
}

/// GET /v1/admin/audit — retrieve append-only audit trail logs
#[utoipa::path(
    get,
    path = "/v1/admin/audit",
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
) -> Result<Json<ListAuditLogsResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, actor_user_id, action, target_type, target_id, metadata, created_at
         FROM tb_audit_log
         ORDER BY created_at DESC
         LIMIT 1000",
    )
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

    Ok(Json(ListAuditLogsResponse { logs }))
}

/// POST /v1/admin/moderate — moderate public content by unpublishing it (setting visibility private)
#[utoipa::path(
    post,
    path = "/v1/admin/moderate",
    request_body = ModerateBody,
    responses(
        (status = 204, description = "Public content moderated and unpublished successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "Content not found"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn moderate_quiz(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Json(body): Json<ModerateBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify quiz exists
    let quiz_row = sqlx::query("SELECT id, title FROM tb_quizzes WHERE id = $1")
        .bind(body.quiz_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::NotFound { resource: "quiz" })?;

    // Unpublish: set visibility to 'private'
    sqlx::query("UPDATE tb_quizzes SET visibility = 'private' WHERE id = $1")
        .bind(body.quiz_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Audit log moderation action
    crate::audit::audit(
        state.pool.clone(),
        Some(admin.0.user.id),
        "moderate.unpublish",
        Some("quiz"),
        Some(body.quiz_id),
        serde_json::json!({
            "title": quiz_row.get::<String, _>("title"),
            "visibility": "private",
        }),
    );

    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/admin/users", get(list_users))
        .route(
            "/v1/admin/users/{id}",
            axum::routing::patch(patch_user_admin),
        )
        .route("/v1/admin/audit", get(list_audit_logs))
        .route("/v1/admin/moderate", post(moderate_quiz))
        .with_state(state)
}
