//! Owner-scoped provider-generation lifecycle resources.

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
    auth::extractor::AuthenticatedUser,
    domain::{
        error::ApiError,
        generation::{GenerationError, GenerationRepository, GenerationStatus, StartGenerationRun},
    },
    generation_postgres::PgGenerationRepository,
    http::AppState,
};

#[derive(Debug, Deserialize, ToSchema)]
pub enum GenerationOperation {
    #[serde(rename = "learning.activity.content.compose")]
    LearningActivityContentCompose,
    #[serde(rename = "learning.activity.rubric.compose")]
    LearningActivityRubricCompose,
    #[serde(rename = "question.compose")]
    QuestionCompose,
    #[serde(rename = "deep_dive.create")]
    DeepDiveCreate,
}

impl GenerationOperation {
    fn into_operation(self) -> String {
        match self {
            Self::LearningActivityContentCompose => "learning.activity.content.compose",
            Self::LearningActivityRubricCompose => "learning.activity.rubric.compose",
            Self::QuestionCompose => "question.compose",
            Self::DeepDiveCreate => "deep_dive.create",
        }
        .into()
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartGenerationRunBody {
    /// Literal operation required by the later content-authoring endpoint.
    pub operation: GenerationOperation,
    pub provider: Option<String>,
    pub retry_key: Option<String>,
    #[serde(default = "default_content_version")]
    pub content_version: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransitionGenerationRunBody {
    pub status: GenerationStatus,
    pub error: Option<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRunResponse {
    pub id: Uuid,
    pub operation: String,
    pub provider: Option<String>,
    pub retry_key: Option<String>,
    pub content_version: u32,
    pub status: GenerationStatus,
    pub error: Option<Value>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/generation-runs", post(start))
        .route("/v1/generation-runs/{id}", get(get_one).patch(transition))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/generation-runs",
    request_body = StartGenerationRunBody,
    responses((status = 201, description = "Started or resumed a provider generation run", body = GenerationRunResponse)),
    security(("bearer" = [])),
    tag = "generation"
)]
pub async fn start(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<StartGenerationRunBody>,
) -> Result<(axum::http::StatusCode, Json<GenerationRunResponse>), ApiError> {
    let value = PgGenerationRepository::new(state.pool)
        .start(StartGenerationRun {
            subject_user_id: auth.owner_id(),
            source_actor_id: auth.actor_identity_id(),
            operation: body.operation.into_operation(),
            provider: body.provider,
            retry_key: body.retry_key,
            content_version: body.content_version,
        })
        .await
        .map_err(map_error)?;
    Ok((axum::http::StatusCode::CREATED, Json(response(value))))
}

#[utoipa::path(
    get,
    path = "/api/v1/generation-runs/{id}",
    params(("id" = Uuid, Path, description = "Generation run ID")),
    responses((status = 200, description = "Owned provider generation run", body = GenerationRunResponse)),
    security(("bearer" = [])),
    tag = "generation"
)]
pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<GenerationRunResponse>, ApiError> {
    let value = PgGenerationRepository::new(state.pool)
        .get(auth.owner_id(), id)
        .await
        .map_err(map_error)?;
    Ok(Json(response(value)))
}

#[utoipa::path(
    patch,
    path = "/api/v1/generation-runs/{id}",
    params(("id" = Uuid, Path, description = "Generation run ID")),
    request_body = TransitionGenerationRunBody,
    responses((status = 200, description = "Transitioned owned provider generation run", body = GenerationRunResponse)),
    security(("bearer" = [])),
    tag = "generation"
)]
pub async fn transition(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<TransitionGenerationRunBody>,
) -> Result<Json<GenerationRunResponse>, ApiError> {
    let value = PgGenerationRepository::new(state.pool)
        .transition(auth.owner_id(), id, body.status, body.error)
        .await
        .map_err(map_error)?;
    Ok(Json(response(value)))
}

fn response(value: crate::domain::generation::GenerationRun) -> GenerationRunResponse {
    GenerationRunResponse {
        id: value.id,
        operation: value.operation,
        provider: value.provider,
        retry_key: value.retry_key,
        content_version: value.content_version,
        status: value.status,
        error: value.error,
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

fn default_content_version() -> u32 {
    1
}

pub(crate) fn map_error(error: GenerationError) -> ApiError {
    match error {
        GenerationError::EmptyField { field } => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: field.into(),
                message: "must not be empty".into(),
            }])
        }
        GenerationError::InvalidContentVersion => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "contentVersion".into(),
                message: "must be greater than zero".into(),
            }])
        }
        GenerationError::InvalidTransition { .. } => ApiError::GenerationStateConflict,
        GenerationError::NotPublished => ApiError::GenerationStateConflict,
        GenerationError::OperationMismatch => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "operation".into(),
                message: "does not match the content resource".into(),
            }])
        }
        GenerationError::SubjectMismatch | GenerationError::NotFound => ApiError::NotFound {
            resource: "generation run",
        },
        GenerationError::Storage(message) => ApiError::Internal(anyhow::anyhow!(message)),
    }
}
