//! Learner-owned practice and graded assessment resources.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    assessment::AssessmentRepository,
    assessment_postgres::PgAssessmentRepository,
    auth::extractor::AuthenticatedUser,
    domain::{
        assessment::{AssessmentItemInput, AssessmentMode, AssessmentStatus, CreateAssessment},
        error::{ApiError, FieldError},
    },
    http::AppState,
    question::QuestionRepository,
    question_postgres::PgQuestionRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssessmentBody {
    pub activity_id: Uuid,
    pub mode: AssessmentMode,
    pub items: Vec<AssessmentItemBody>,
    #[serde(default)]
    pub status: AssessmentStatus,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentItemBody {
    pub question_id: Uuid,
    pub question_version: u32,
    pub order_index: i32,
    pub points: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentResponse {
    pub id: Uuid,
    pub activity_id: Uuid,
    pub version: u32,
    pub mode: AssessmentMode,
    pub status: AssessmentStatus,
    pub items: Vec<AssessmentItemResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentItemResponse {
    pub question_version_id: Uuid,
    pub order_index: i32,
    pub points: u32,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/assessments", post(create_assessment))
        .route("/v1/assessments/{assessment_id}", get(get_assessment))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/v1/assessments",
    request_body = CreateAssessmentBody,
    responses((status = 200, description = "Created assessment", body = AssessmentResponse)),
    security(("bearer" = [])),
    tag = "assessments"
)]
pub async fn create_assessment(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateAssessmentBody>,
) -> Result<Json<AssessmentResponse>, ApiError> {
    let question_repository = PgQuestionRepository::new(state.pool.clone());
    let mut questions = Vec::with_capacity(body.items.len());
    let mut items = Vec::with_capacity(body.items.len());
    for item in body.items {
        let question = question_repository
            .get_version(auth.owner_id(), item.question_id, item.question_version)
            .await
            .map_err(map_assessment_error)?;
        items.push(AssessmentItemInput {
            question_version_id: question.id,
            order_index: item.order_index,
            points: item.points,
        });
        questions.push(question);
    }
    let assessment = PgAssessmentRepository::new(state.pool)
        .create_assessment(
            CreateAssessment {
                subject_user_id: auth.owner_id(),
                source_actor_id: auth.owner_id(),
                activity_id: body.activity_id,
                mode: body.mode,
                items,
                status: body.status,
            },
            &questions,
        )
        .await
        .map_err(map_assessment_error)?;
    Ok(Json(assessment_response(assessment)))
}

#[utoipa::path(
    get,
    path = "/v1/assessments/{assessment_id}",
    params(("assessment_id" = Uuid, Path, description = "Assessment ID")),
    responses((status = 200, description = "Assessment", body = AssessmentResponse)),
    security(("bearer" = [])),
    tag = "assessments"
)]
pub async fn get_assessment(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(assessment_id): Path<Uuid>,
) -> Result<Json<AssessmentResponse>, ApiError> {
    let assessment = PgAssessmentRepository::new(state.pool)
        .get_assessment(auth.owner_id(), assessment_id)
        .await
        .map_err(map_assessment_error)?;
    Ok(Json(assessment_response(assessment)))
}

fn assessment_response(assessment: crate::domain::assessment::Assessment) -> AssessmentResponse {
    AssessmentResponse {
        id: assessment.id,
        activity_id: assessment.activity_id,
        version: assessment.version,
        mode: assessment.mode,
        status: assessment.status,
        items: assessment
            .items
            .into_iter()
            .map(|item| AssessmentItemResponse {
                question_version_id: item.question_version_id,
                order_index: item.order_index,
                points: item.points,
            })
            .collect(),
    }
}

fn map_assessment_error(error: impl Into<AssessmentApiError>) -> ApiError {
    match error.into() {
        AssessmentApiError::Question(
            crate::domain::question::QuestionRepositoryError::NotFound { resource },
        ) => ApiError::NotFound { resource },
        AssessmentApiError::Question(
            crate::domain::question::QuestionRepositoryError::SubjectMismatch,
        ) => ApiError::NotFound {
            resource: "question",
        },
        AssessmentApiError::Question(
            crate::domain::question::QuestionRepositoryError::Storage(message),
        ) => ApiError::Internal(anyhow::anyhow!(message)),
        AssessmentApiError::Question(
            crate::domain::question::QuestionRepositoryError::EmptyField { field },
        ) => ApiError::Validation(vec![FieldError {
            field: field.to_string(),
            message: "must not be empty".into(),
        }]),
        AssessmentApiError::Question(
            crate::domain::question::QuestionRepositoryError::InvalidQuestion(message),
        ) => ApiError::Validation(vec![FieldError {
            field: "question".into(),
            message,
        }]),
        AssessmentApiError::Question(
            crate::domain::question::QuestionRepositoryError::VersionConflict,
        ) => ApiError::IdempotencyConflict,
        AssessmentApiError::Assessment(
            crate::domain::assessment::AssessmentRepositoryError::InvalidItem(message),
        ) => ApiError::Validation(vec![FieldError {
            field: "assessment".into(),
            message,
        }]),
        AssessmentApiError::Assessment(
            crate::domain::assessment::AssessmentRepositoryError::EmptyItems,
        ) => ApiError::Validation(vec![FieldError {
            field: "items".into(),
            message: "must not be empty".into(),
        }]),
        AssessmentApiError::Assessment(
            crate::domain::assessment::AssessmentRepositoryError::SubjectMismatch,
        ) => ApiError::NotFound {
            resource: "assessment",
        },
        AssessmentApiError::Assessment(
            crate::domain::assessment::AssessmentRepositoryError::NotFound,
        ) => ApiError::NotFound {
            resource: "assessment",
        },
        AssessmentApiError::Assessment(
            crate::domain::assessment::AssessmentRepositoryError::ActivityAlreadyHasAssessment,
        ) => ApiError::IdempotencyConflict,
        AssessmentApiError::Assessment(
            crate::domain::assessment::AssessmentRepositoryError::Question(error),
        ) => map_assessment_error(error),
        AssessmentApiError::Assessment(
            crate::domain::assessment::AssessmentRepositoryError::Storage(message),
        ) => ApiError::Internal(anyhow::anyhow!(message)),
    }
}

enum AssessmentApiError {
    Question(crate::domain::question::QuestionRepositoryError),
    Assessment(crate::domain::assessment::AssessmentRepositoryError),
}

impl From<crate::domain::question::QuestionRepositoryError> for AssessmentApiError {
    fn from(error: crate::domain::question::QuestionRepositoryError) -> Self {
        Self::Question(error)
    }
}

impl From<crate::domain::assessment::AssessmentRepositoryError> for AssessmentApiError {
    fn from(error: crate::domain::assessment::AssessmentRepositoryError) -> Self {
        Self::Assessment(error)
    }
}
