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
    #[error("scope required: {0}")]
    ScopeRequired(Cow<'static, str>),
    #[error("not found: {resource}")]
    NotFound { resource: &'static str },
    #[error("validation failed")]
    Validation(Vec<FieldError>),
    #[error("invalid payload for kind {kind}: {reason}")]
    InvalidPayload { kind: String, reason: String },
    #[error("exam pool insufficient")]
    ExamPoolInsufficient {
        section: String,
        required: usize,
        available: usize,
    },
    #[error("session already finished")]
    SessionFinished,
    #[error("exam expired")]
    ExamExpired,
    #[error("idempotency key conflict")]
    IdempotencyConflict,
    #[error("scoring unavailable")]
    ScoringUnavailable,
    #[error("too many requests")]
    TooManyRequests,
    #[error("quota exceeded")]
    QuotaExceeded {
        kind: String,
        limit: i64,
        usage: i64,
    },
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
            ApiError::Forbidden(_) | ApiError::ScopeRequired(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::Validation(_)
            | ApiError::InvalidPayload { .. }
            | ApiError::ExamPoolInsufficient { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::SessionFinished | ApiError::IdempotencyConflict => StatusCode::CONFLICT,
            ApiError::ExamExpired => StatusCode::GONE,
            ApiError::ScoringUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::TooManyRequests | ApiError::QuotaExceeded { .. } => {
                StatusCode::TOO_MANY_REQUESTS
            }
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
            ApiError::ScopeRequired(scope) => (
                StatusCode::FORBIDDEN,
                "scope_required",
                format!("token lacks required scope: {}", scope),
                Some(json!({ "scope": scope })),
            ),
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
            ApiError::InvalidPayload { kind, reason } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_question_payload",
                format!("payload doesn't match declared kind {}: {}", kind, reason),
                None,
            ),
            ApiError::ExamPoolInsufficient {
                section,
                required,
                available,
            } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "exam_pool_insufficient",
                "dynamic blueprint can't be filled".to_string(),
                Some(json!({ "section": section, "required": required, "available": available })),
            ),
            ApiError::SessionFinished => (
                StatusCode::CONFLICT,
                "session_finished",
                "answering a finished/abandoned session".to_string(),
                None,
            ),
            ApiError::ExamExpired => (
                StatusCode::GONE,
                "exam_expired",
                "answering after deadline".to_string(),
                None,
            ),
            ApiError::IdempotencyConflict => (
                StatusCode::CONFLICT,
                "idempotency_conflict",
                "same Idempotency-Key reused with different body".to_string(),
                None,
            ),
            ApiError::ScoringUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "scoring_unavailable",
                "grade required an unavailable dependency".to_string(),
                None,
            ),
            ApiError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "rate limit exceeded".to_string(),
                None,
            ),
            ApiError::QuotaExceeded { kind, limit, usage } => (
                StatusCode::TOO_MANY_REQUESTS,
                "quota_exceeded",
                format!("plan quota exceeded for {}", kind),
                Some(json!({ "kind": kind, "limit": limit, "usage": usage })),
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

        (status, Json(json!({ "error": error_body }))).into_response()
    }
}
