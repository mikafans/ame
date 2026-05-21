//! Admin-only user management routes.

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
pub struct UpdateUserRoleBody {
    pub role: String,
}

pub async fn list_users(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
) -> Result<Json<ListUsersResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, email, display_name, role, created_at
         FROM users
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
                "instructor" => Role::Instructor,
                "agent" => Role::Agent,
                _ => Role::Learner,
            };
            User {
                id: r.get("id"),
                email: r.get("email"),
                display_name: r.get("display_name"),
                role,
                created_at: r.get("created_at"),
            }
        })
        .collect();

    Ok(Json(ListUsersResponse { users }))
}

pub async fn update_user_role(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUserRoleBody>,
) -> Result<impl IntoResponse, ApiError> {
    let valid_roles = ["learner", "instructor", "admin"];
    if !valid_roles.contains(&body.role.as_str()) {
        return Err(ApiError::Validation(vec![FieldError {
            field: "role".into(),
            message: "must be 'learner', 'instructor', or 'admin'".into(),
        }]));
    }

    let affected = sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
        .bind(&body.role)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound { resource: "user" });
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn deactivate_user(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // Deactivate by revoking all tokens
    sqlx::query(
        "UPDATE api_tokens SET revoked_at = now()
         WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/admin/users", get(list_users))
        .route("/v1/admin/users/{id}/role", post(update_user_role))
        .route("/v1/admin/users/{id}/deactivate", post(deactivate_user))
        .with_state(state)
}
