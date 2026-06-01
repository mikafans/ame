//! Feedback messages: POST /v1/messages.

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::scope::{RequireScope, ScopeConstraint},
    domain::{
        error::{ApiError, FieldError},
        user::Scope,
    },
    http::AppState,
};

pub struct FeedbackWriteScope;
impl ScopeConstraint for FeedbackWriteScope {
    const SCOPE: Scope = Scope::FeedbackWrite;
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageBody {
    pub user_id: Uuid,
    pub channel: String,
    pub body: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    pub message_id: Uuid,
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/v1/messages",
    request_body = SendMessageBody,
    responses(
        (status = 201, description = "Message queued", body = SendMessageResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Requires feedback.write scope"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn send_message(
    State(state): State<AppState>,
    user: RequireScope<FeedbackWriteScope>,
    Json(body): Json<SendMessageBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.channel != "in_app" && body.channel != "email" {
        return Err(ApiError::Validation(vec![FieldError {
            field: "channel".into(),
            message: "must be 'in_app' or 'email'".into(),
        }]));
    }
    if body.body.trim().is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "body".into(),
            message: "must not be empty".into(),
        }]));
    }

    // Verify recipient exists
    let recipient_exists: bool =
        sqlx::query_scalar("SELECT exists(SELECT 1 FROM tb_users WHERE id = $1)")
            .bind(body.user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
    if !recipient_exists {
        return Err(ApiError::NotFound { resource: "user" });
    }

    let row = sqlx::query(
        "INSERT INTO tb_messages (from_user_id, to_user_id, channel, body, status)
         VALUES ($1, $2, $3, $4, 'queued')
         RETURNING id, status",
    )
    .bind(user.0.user.id)
    .bind(body.user_id)
    .bind(&body.channel)
    .bind(&body.body)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // email channel: stored as queued; no transport in P7
    Ok((
        StatusCode::CREATED,
        Json(SendMessageResponse {
            message_id: row.get("id"),
            status: row.get("status"),
        }),
    ))
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/messages", post(send_message))
        .with_state(state)
}
