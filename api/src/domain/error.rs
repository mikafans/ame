use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::borrow::Cow;
use tracing::error;
use utoipa::ToSchema;

#[derive(thiserror::Error, Debug, ToSchema)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(Cow<'static, str>),
    #[error("not found: {resource}")]
    NotFound { resource: &'static str },
    #[error("validation failed")]
    Validation(Vec<FieldError>),
    #[error("session already finished")]
    SessionFinished,
    #[error("learning session result conflicts with the completed result")]
    LearningSessionResultConflict,
    #[error("learning activity content conflicts with the activity state")]
    ActivityContentConflict,
    #[error("idempotency key conflict")]
    IdempotencyConflict,
    #[error("generation state transition conflicts with the current state")]
    GenerationStateConflict,
    #[error("too many requests")]
    TooManyRequests,
    #[error("service in maintenance mode")]
    Maintenance,
    #[error(transparent)]
    #[schema(value_type = String)]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl ApiError {
    /// The HTTP status this error maps to. Kept in sync with `into_response`;
    /// used by callers (e.g. the agent run-door activity log) that need the
    /// status without consuming the error into a `Response`.
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::SessionFinished
            | ApiError::LearningSessionResultConflict
            | ApiError::ActivityContentConflict
            | ApiError::IdempotencyConflict
            | ApiError::GenerationStateConflict => StatusCode::CONFLICT,
            ApiError::Maintenance => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing or invalid token".to_string(),
                None,
            ),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.to_string(), None),
            ApiError::NotFound { resource } => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("resource not found: {}", resource),
                None,
            ),
            ApiError::Validation(fields) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "body fails schema validation".to_string(),
                Some(json!({ "fields": fields })),
            ),
            ApiError::SessionFinished => (
                StatusCode::CONFLICT,
                "session_finished",
                "answering a finished/abandoned session".to_string(),
                None,
            ),
            ApiError::LearningSessionResultConflict => (
                StatusCode::CONFLICT,
                "session_result_conflict",
                "the session was already finished with a different result".to_string(),
                None,
            ),
            ApiError::ActivityContentConflict => (
                StatusCode::CONFLICT,
                "activity_content_conflict",
                "the activity content cannot be changed in its current state".to_string(),
                None,
            ),
            ApiError::IdempotencyConflict => (
                StatusCode::CONFLICT,
                "idempotency_conflict",
                "same Idempotency-Key reused with different body".to_string(),
                None,
            ),
            ApiError::GenerationStateConflict => (
                StatusCode::CONFLICT,
                "generation_state_conflict",
                "the generation run cannot make that state transition".to_string(),
                None,
            ),
            ApiError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "rate limit exceeded".to_string(),
                None,
            ),
            ApiError::Maintenance => (
                StatusCode::SERVICE_UNAVAILABLE,
                "maintenance",
                "the service is temporarily in maintenance mode".to_string(),
                None,
            ),
            ApiError::Internal(err) => {
                let request_id = uuid::Uuid::now_v7().to_string();
                error!(%request_id, "internal error: {:#}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "internal server error".to_string(),
                    Some(json!({ "request_id": request_id })),
                )
            }
        };

        let mut error_body = json!({
            "code": code,
            "message": message,
        });

        if let Some(d) = details {
            error_body["details"] = d;
        }

        if status == StatusCode::TOO_MANY_REQUESTS {
            (
                status,
                [("retry-after", "1")],
                Json(json!({ "error": error_body })),
            )
                .into_response()
        } else {
            (status, Json(json!({ "error": error_body }))).into_response()
        }
    }
}
