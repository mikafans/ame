//! Learner-owned assessment attempt resources.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    assessment::AssessmentRepository,
    assessment_postgres::PgAssessmentRepository,
    attempt::AttemptRepository,
    attempt_postgres::PgAttemptRepository,
    auth::extractor::AuthenticatedUser,
    domain::{
        assessment::AssessmentStatus,
        attempt::{Attempt, AttemptError, StartAttempt},
        error::{ApiError, FieldError},
    },
    http::AppState,
    question_postgres::PgQuestionRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartAttemptBody {
    pub learning_session_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveAnswerBody {
    pub assessment_item_id: Uuid,
    pub question_version_id: Uuid,
    pub response: Value,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttemptResponse {
    pub id: Uuid,
    pub assessment_id: Uuid,
    pub assessment_version: u32,
    pub status: String,
    pub responses: std::collections::HashMap<Uuid, Value>,
    pub score: Option<f32>,
    pub awarded_points: Option<f32>,
    pub max_points: Option<f32>,
    pub submitted_at: Option<String>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/v1/assessments/{assessment_id}/attempts",
            post(start_attempt),
        )
        .route("/v1/attempts/{attempt_id}", get(get_attempt))
        .route("/v1/attempts/{attempt_id}/answers", post(save_answer))
        .route("/v1/attempts/{attempt_id}/finish", post(finish_attempt))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/assessments/{assessment_id}/attempts",
    params(("assessment_id" = Uuid, Path, description = "Assessment ID")),
    request_body = StartAttemptBody,
    responses((status = 200, description = "Started or resumed attempt", body = AttemptResponse)),
    security(("bearer" = [])),
    tag = "attempts"
)]
pub async fn start_attempt(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(assessment_id): Path<Uuid>,
    Json(body): Json<StartAttemptBody>,
) -> Result<Json<AttemptResponse>, ApiError> {
    let assessment = PgAssessmentRepository::new(state.pool.clone())
        .get_assessment(auth.owner_id(), assessment_id)
        .await
        .map_err(|error| map_attempt_error(AttemptError::Storage(error.to_string())))?;
    if assessment.status != AssessmentStatus::Published {
        return Err(ApiError::NotFound {
            resource: "assessment",
        });
    }
    let attempt = PgAttemptRepository::new(state.pool)
        .start(
            StartAttempt {
                subject_user_id: auth.owner_id(),
                learning_session_id: body.learning_session_id,
                activity_id: assessment.activity_id,
                assessment_id,
                assessment_version: assessment.version,
            },
            &assessment,
        )
        .await
        .map_err(map_attempt_error)?;
    Ok(Json(attempt_response(attempt)))
}

#[utoipa::path(
    get,
    path = "/api/v1/attempts/{attempt_id}",
    params(("attempt_id" = Uuid, Path, description = "Attempt ID")),
    responses((status = 200, description = "Attempt", body = AttemptResponse)),
    security(("bearer" = [])),
    tag = "attempts"
)]
pub async fn get_attempt(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(attempt_id): Path<Uuid>,
) -> Result<Json<AttemptResponse>, ApiError> {
    let attempt = PgAttemptRepository::new(state.pool)
        .get(auth.owner_id(), attempt_id)
        .await
        .map_err(map_attempt_error)?;
    Ok(Json(attempt_response(attempt)))
}

#[utoipa::path(
    post,
    path = "/api/v1/attempts/{attempt_id}/answers",
    params(("attempt_id" = Uuid, Path, description = "Attempt ID")),
    request_body = SaveAnswerBody,
    responses((status = 200, description = "Saved answer", body = AttemptResponse)),
    security(("bearer" = [])),
    tag = "attempts"
)]
pub async fn save_answer(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(attempt_id): Path<Uuid>,
    Json(body): Json<SaveAnswerBody>,
) -> Result<Json<AttemptResponse>, ApiError> {
    let attempt = PgAttemptRepository::new(state.pool)
        .save_answer(
            auth.owner_id(),
            attempt_id,
            body.assessment_item_id,
            body.question_version_id,
            body.response,
        )
        .await
        .map_err(map_attempt_error)?;
    Ok(Json(attempt_response(attempt)))
}

#[utoipa::path(
    post,
    path = "/api/v1/attempts/{attempt_id}/finish",
    params(("attempt_id" = Uuid, Path, description = "Attempt ID")),
    responses((status = 200, description = "Finished attempt", body = AttemptResponse)),
    security(("bearer" = [])),
    tag = "attempts"
)]
pub async fn finish_attempt(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(attempt_id): Path<Uuid>,
) -> Result<Json<AttemptResponse>, ApiError> {
    let pool = state.pool.clone();
    let repository = PgAttemptRepository::new(pool.clone());
    let attempt = repository
        .get(auth.owner_id(), attempt_id)
        .await
        .map_err(map_attempt_error)?;
    let assessment = PgAssessmentRepository::new(pool.clone())
        .get_assessment(auth.owner_id(), attempt.input.assessment_id)
        .await
        .map_err(|error| map_attempt_error(AttemptError::Storage(error.to_string())))?;
    let question_repository = PgQuestionRepository::new(pool);
    let mut questions = Vec::with_capacity(assessment.items.len());
    for item in &assessment.items {
        questions.push(
            question_repository
                .get_version_by_id(auth.owner_id(), item.question_version_id)
                .await
                .map_err(|error| map_attempt_error(AttemptError::Storage(error.to_string())))?,
        );
    }
    let attempt = repository
        .finish(auth.owner_id(), attempt_id, &assessment, &questions)
        .await
        .map_err(map_attempt_error)?;
    Ok(Json(attempt_response(attempt)))
}

fn attempt_response(attempt: Attempt) -> AttemptResponse {
    let grade = attempt.grade;
    AttemptResponse {
        id: attempt.id,
        assessment_id: attempt.input.assessment_id,
        assessment_version: attempt.input.assessment_version,
        status: match attempt.status {
            crate::domain::attempt::AttemptStatus::InProgress => "in_progress",
            crate::domain::attempt::AttemptStatus::Submitted => "submitted",
            crate::domain::attempt::AttemptStatus::Graded => "graded",
            crate::domain::attempt::AttemptStatus::Abandoned => "abandoned",
        }
        .into(),
        responses: attempt.responses,
        score: grade.as_ref().and_then(|value| value.score),
        awarded_points: grade.as_ref().map(|value| value.awarded_points),
        max_points: grade.as_ref().map(|value| value.max_points),
        submitted_at: attempt.submitted_at.map(|value| {
            value
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        }),
    }
}

fn map_attempt_error(error: AttemptError) -> ApiError {
    match error {
        AttemptError::NotFound => ApiError::NotFound {
            resource: "attempt",
        },
        AttemptError::SubjectMismatch => ApiError::NotFound {
            resource: "attempt",
        },
        AttemptError::ItemNotInAssessment => ApiError::Validation(vec![FieldError {
            field: "assessmentItemId".into(),
            message: "does not belong to this assessment".into(),
        }]),
        AttemptError::StaleQuestionVersion => ApiError::Validation(vec![FieldError {
            field: "questionVersionId".into(),
            message: "does not match the immutable assessment version".into(),
        }]),
        AttemptError::AlreadyFinished
        | AttemptError::ResultConflict
        | AttemptError::InvalidTransition => ApiError::IdempotencyConflict,
        AttemptError::Storage(message) => ApiError::Internal(anyhow::anyhow!(message)),
    }
}
