use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::admin::RequireAdmin,
    domain::error::{ApiError, FieldError},
    domain::user::User,
    http::AppState,
};

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
    pub status: Option<String>,
    pub role: Option<String>,
}

/// GET /api/v1/admin/users — list all users
#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
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
    _admin: RequireAdmin,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let (users, total) =
        ame_platform_postgres::admin_users::list(&state.pool, limit, offset, query.q.as_deref())
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(ListUsersResponse { users, total }))
}

/// PATCH /v1/admin/users/{id} — update learner/admin role or account status
#[utoipa::path(
    patch,
    path = "/api/v1/admin/users/{id}",
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
    admin: RequireAdmin,
    Path(user_id): Path<Uuid>,
    Json(body): Json<PatchUserAdminBody>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. Verify user exists and fetch current clean role/status.
    let current_user = ame_platform_postgres::admin_users::role_status(&state.pool, user_id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let (current_role, current_status) = match current_user {
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
    if target_is_self && body.status.as_deref() == Some("deactivated") {
        crate::audit::audit(
            state.pool.clone(),
            Some(admin.0.user.id),
            "user.patch_rejected",
            Some("user"),
            Some(user_id),
            serde_json::json!({ "reason": "self_disable_forbidden" }),
        );
        return Err(ApiError::Validation(vec![FieldError {
            field: "status".into(),
            message: "cannot deactivate your own account".into(),
        }]));
    }

    // C. Check last active admin protection
    let is_active_admin = current_role == "admin" && current_status == "active";
    let changing_to_non_admin = body.role.as_ref().map(|r| r != "admin").unwrap_or(false);
    let changing_to_deactivated = body.status.as_deref() == Some("deactivated");

    if is_active_admin && (changing_to_non_admin || changing_to_deactivated) {
        let active_admin_count =
            ame_platform_postgres::admin_users::active_admin_count(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;

        if active_admin_count <= 1 {
            let field = if changing_to_deactivated {
                "status"
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

    // 2. Perform status update if requested.
    if let Some(status) = &body.status {
        if !["active", "deactivated"].contains(&status.as_str()) {
            return Err(ApiError::Validation(vec![FieldError {
                field: "status".into(),
                message: "must be 'active' or 'deactivated'".into(),
            }]));
        }
        ame_platform_postgres::admin_users::update_status(&state.pool, user_id, status)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
        crate::audit::audit(
            state.pool.clone(),
            Some(admin.0.user.id),
            "user.update_status",
            Some("user"),
            Some(user_id),
            serde_json::json!({ "status": status }),
        );
    }

    // 3. Perform role updates if requested.
    if let Some(role) = &body.role {
        let valid_roles = ["learner", "admin"];
        if !valid_roles.contains(&role.as_str()) {
            return Err(ApiError::Validation(vec![FieldError {
                field: "role".into(),
                message: "must be 'learner' or 'admin'".into(),
            }]));
        }

        ame_platform_postgres::admin_users::update_role(&state.pool, user_id, role)
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

    // Role/status changes revoke active sessions so the clean principal is
    // re-evaluated on the next request.
    if body.role.is_some() || body.status.is_some() {
        crate::auth::extractor::invalidate_user_tokens(&state.pool, &state.valkey, user_id).await;
    }

    Ok(StatusCode::NO_CONTENT)
}
