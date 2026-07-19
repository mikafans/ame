//! Learner-owned explanatory deep-dive resources.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    deep_dive::DeepDiveRepository,
    deep_dive_postgres::PgDeepDiveRepository,
    domain::{
        deep_dive::{CreateDeepDive, DeepDiveError},
        error::{ApiError, FieldError},
        question::ContentReviewStatus,
    },
    http::AppState,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeepDiveBody {
    pub generation_run_id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub objective_id: Uuid,
    pub triggering_evidence_id: Uuid,
    pub title: String,
    pub body: String,
    pub example: String,
    pub caveats: Vec<String>,
    pub source_references: Vec<String>,
    pub application_task: String,
    #[serde(default)]
    pub review_status: ContentReviewStatus,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeepDiveResponse {
    pub id: Uuid,
    pub generation_run_id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub objective_id: Uuid,
    pub triggering_evidence_id: Uuid,
    pub title: String,
    pub body: String,
    pub example: String,
    pub caveats: Vec<String>,
    pub source_references: Vec<String>,
    pub application_task: String,
    pub content_version: u32,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct DeepDiveActivityQuery {
    pub activity_id: Uuid,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/deep-dives", post(create))
        .route("/v1/deep-dives", get(get_for_activity))
        .route("/v1/deep-dives/{id}", get(get_one))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/api/v1/deep-dives",
    params(DeepDiveActivityQuery),
    responses(
        (status = 200, description = "Deep dive attached to the activity", body = DeepDiveResponse),
        (status = 404, description = "No deep dive exists for this learner's activity")
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn get_for_activity(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<DeepDiveActivityQuery>,
) -> Result<Json<DeepDiveResponse>, ApiError> {
    let value = PgDeepDiveRepository::new(state.pool)
        .get_for_activity(auth.owner_id(), query.activity_id)
        .await
        .map_err(map_error)?
        .ok_or(ApiError::NotFound {
            resource: "deep dive",
        })?;
    Ok(Json(response(value)))
}

#[utoipa::path(post, path = "/api/v1/deep-dives", request_body = CreateDeepDiveBody, responses((status = 200, body = DeepDiveResponse)), security(("bearer" = [])), tag = "deep-dives")]
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateDeepDiveBody>,
) -> Result<Json<DeepDiveResponse>, ApiError> {
    let value = PgDeepDiveRepository::new(state.pool)
        .create(CreateDeepDive {
            subject_user_id: auth.owner_id(),
            source_actor_id: auth.owner_id(),
            generation_run_id: body.generation_run_id,
            journey_id: body.journey_id,
            activity_id: body.activity_id,
            objective_id: body.objective_id,
            triggering_evidence_id: body.triggering_evidence_id,
            title: body.title,
            body: body.body,
            example: body.example,
            caveats: body.caveats,
            source_references: body.source_references,
            application_task: body.application_task,
            review_status: body.review_status,
        })
        .await
        .map_err(map_error)?;
    Ok(Json(response(value)))
}

#[utoipa::path(get, path = "/api/v1/deep-dives/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = DeepDiveResponse)), security(("bearer" = [])), tag = "deep-dives")]
pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DeepDiveResponse>, ApiError> {
    Ok(Json(response(
        PgDeepDiveRepository::new(state.pool)
            .get(auth.owner_id(), id)
            .await
            .map_err(map_error)?,
    )))
}

fn response(value: crate::domain::deep_dive::DeepDive) -> DeepDiveResponse {
    let input = value.input;
    DeepDiveResponse {
        id: value.id,
        generation_run_id: input.generation_run_id,
        journey_id: input.journey_id,
        activity_id: input.activity_id,
        objective_id: input.objective_id,
        triggering_evidence_id: input.triggering_evidence_id,
        title: input.title,
        body: input.body,
        example: input.example,
        caveats: input.caveats,
        source_references: input.source_references,
        application_task: input.application_task,
        content_version: value.content_version,
    }
}
fn map_error(error: DeepDiveError) -> ApiError {
    match error {
        DeepDiveError::NotFound | DeepDiveError::SubjectMismatch => ApiError::NotFound {
            resource: "deep dive",
        },
        DeepDiveError::EmptyField { field } => ApiError::Validation(vec![FieldError {
            field: field.into(),
            message: "must not be empty".into(),
        }]),
        DeepDiveError::MissingSource => ApiError::Validation(vec![FieldError {
            field: "sourceReferences".into(),
            message: "must contain at least one source".into(),
        }]),
        DeepDiveError::Generation(error) => match error {
            crate::domain::generation::GenerationError::NotPublished => {
                ApiError::GenerationStateConflict
            }
            crate::domain::generation::GenerationError::OperationMismatch => {
                ApiError::Validation(vec![FieldError {
                    field: "generationRunId".into(),
                    message: "must reference a deep_dive.create run".into(),
                }])
            }
            crate::domain::generation::GenerationError::SubjectMismatch
            | crate::domain::generation::GenerationError::NotFound => ApiError::NotFound {
                resource: "generation run",
            },
            other => ApiError::Internal(anyhow::anyhow!(other.to_string())),
        },
        DeepDiveError::Storage(message) => ApiError::Internal(anyhow::anyhow!(message)),
    }
}
