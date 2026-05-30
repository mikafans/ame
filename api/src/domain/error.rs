use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("scope required: {0}")]
    ScopeRequired(&'static str),
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
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
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
                "anonymous_attempt_rate_limited",
                "rate limit exceeded for anonymous attempts".to_string(),
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
