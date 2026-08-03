//! Learner-owned question authoring and version reads.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::{ApiError, FieldError},
        question::{ContentReviewStatus, CreateQuestion, QuestionKind, QuestionOption},
    },
    http::AppState,
    question::QuestionRepository,
    question_postgres::PgQuestionRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuestionBody {
    pub generation_run_id: Uuid,
    pub kind: QuestionKind,
    pub prompt: String,
    #[serde(default)]
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub accepted_answers: Vec<String>,
    pub explanation: Option<String>,
    pub rationale: Option<String>,
    pub difficulty: Option<String>,
    pub points: u32,
    #[serde(default)]
    pub review_status: ContentReviewStatus,
    #[serde(default)]
    pub source_references: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuestionResponse {
    pub id: Uuid,
    pub question_id: Uuid,
    pub version: u32,
    pub generation_run_id: Uuid,
    pub kind: QuestionKind,
    pub prompt: String,
    pub options: Vec<QuestionOption>,
    pub accepted_answers: Vec<String>,
    pub explanation: Option<String>,
    pub rationale: Option<String>,
    pub difficulty: Option<String>,
    pub points: u32,
    pub review_status: ContentReviewStatus,
    pub source_references: Vec<String>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/questions", post(create_question))
        .route(
            "/v1/questions/{question_id}/versions/{version}",
            get(get_question_version),
        )
        .route(
            "/v1/questions/{question_id}/versions",
            post(create_question_version),
        )
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/questions",
    request_body = CreateQuestionBody,
    responses((status = 200, description = "Created question version", body = QuestionResponse)),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn create_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateQuestionBody>,
) -> Result<Json<QuestionResponse>, ApiError> {
    if body.review_status == ContentReviewStatus::Approved {
        crate::http::citations::certify_references(
            state.pool.clone(),
            auth.owner_id(),
            &body.source_references,
        )
        .await?;
    }
    let repository = PgQuestionRepository::new(state.pool);
    let (_, version) = repository
        .create_question(CreateQuestion {
            subject_user_id: auth.owner_id(),
            source_actor_id: auth.actor_identity_id(),
            generation_run_id: body.generation_run_id,
            kind: body.kind,
            prompt: body.prompt,
            options: body.options,
            accepted_answers: body.accepted_answers,
            explanation: body.explanation,
            rationale: body.rationale,
            difficulty: body.difficulty,
            points: body.points,
            review_status: body.review_status,
            source_references: body.source_references,
        })
        .await
        .map_err(map_question_error)?;
    Ok(Json(question_response(version)))
}

#[utoipa::path(
    post,
    path = "/api/v1/questions/{question_id}/versions",
    params(("question_id" = Uuid, Path, description = "Question ID")),
    request_body = CreateQuestionBody,
    responses((status = 200, description = "Created question version", body = QuestionResponse)),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn create_question_version(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(question_id): Path<Uuid>,
    Json(body): Json<CreateQuestionBody>,
) -> Result<Json<QuestionResponse>, ApiError> {
    if body.review_status == ContentReviewStatus::Approved {
        crate::http::citations::certify_references(
            state.pool.clone(),
            auth.owner_id(),
            &body.source_references,
        )
        .await?;
    }
    let repository = PgQuestionRepository::new(state.pool);
    let version = repository
        .create_version(
            auth.owner_id(),
            question_id,
            CreateQuestion {
                subject_user_id: auth.owner_id(),
                source_actor_id: auth.actor_identity_id(),
                generation_run_id: body.generation_run_id,
                kind: body.kind,
                prompt: body.prompt,
                options: body.options,
                accepted_answers: body.accepted_answers,
                explanation: body.explanation,
                rationale: body.rationale,
                difficulty: body.difficulty,
                points: body.points,
                review_status: body.review_status,
                source_references: body.source_references,
            },
        )
        .await
        .map_err(map_question_error)?;
    Ok(Json(question_response(version)))
}

#[utoipa::path(
    get,
    path = "/api/v1/questions/{question_id}/versions/{version}",
    params(
        ("question_id" = Uuid, Path, description = "Question ID"),
        ("version" = u32, Path, description = "Question version")
    ),
    responses((status = 200, description = "Question version", body = QuestionResponse)),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn get_question_version(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((question_id, version)): Path<(Uuid, u32)>,
) -> Result<Json<QuestionResponse>, ApiError> {
    let repository = PgQuestionRepository::new(state.pool);
    let version = repository
        .get_version(auth.owner_id(), question_id, version)
        .await
        .map_err(map_question_error)?;
    Ok(Json(question_response(version)))
}

fn question_response(version: crate::domain::question::QuestionVersion) -> QuestionResponse {
    QuestionResponse {
        id: version.id,
        question_id: version.question_id,
        version: version.version,
        generation_run_id: version.generation_run_id,
        kind: version.kind,
        prompt: version.prompt,
        options: version.options,
        accepted_answers: version.accepted_answers,
        explanation: version.explanation,
        rationale: version.rationale,
        difficulty: version.difficulty,
        points: version.points,
        review_status: version.review_status,
        source_references: version.source_references,
    }
}

fn map_question_error(error: crate::domain::question::QuestionRepositoryError) -> ApiError {
    match error {
        crate::domain::question::QuestionRepositoryError::EmptyField { field } => {
            ApiError::Validation(vec![FieldError {
                field: field.to_string(),
                message: "must not be empty".into(),
            }])
        }
        crate::domain::question::QuestionRepositoryError::InvalidQuestion(message) => {
            ApiError::Validation(vec![FieldError {
                field: "question".into(),
                message,
            }])
        }
        crate::domain::question::QuestionRepositoryError::NotFound { resource } => {
            ApiError::NotFound { resource }
        }
        crate::domain::question::QuestionRepositoryError::SubjectMismatch => ApiError::NotFound {
            resource: "question",
        },
        crate::domain::question::QuestionRepositoryError::VersionConflict => {
            ApiError::IdempotencyConflict
        }
        crate::domain::question::QuestionRepositoryError::Generation(error) => match error {
            crate::domain::generation::GenerationError::NotPublished => {
                ApiError::GenerationStateConflict
            }
            crate::domain::generation::GenerationError::OperationMismatch => {
                ApiError::Validation(vec![FieldError {
                    field: "generationRunId".into(),
                    message: "must reference a question.compose run".into(),
                }])
            }
            crate::domain::generation::GenerationError::SubjectMismatch
            | crate::domain::generation::GenerationError::NotFound => ApiError::NotFound {
                resource: "generation run",
            },
            other => ApiError::Internal(anyhow::anyhow!(other.to_string())),
        },
        crate::domain::question::QuestionRepositoryError::Storage(error) => {
            ApiError::Internal(anyhow::anyhow!(error))
        }
    }
}
