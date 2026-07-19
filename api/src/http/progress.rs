//! Learner-owned mastery, recommendation, and streak resources.

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
        progress::{MasteryEvidenceInput, ProgressError, StreakEventInput},
    },
    http::AppState,
    progress::ProgressRepository,
    progress_postgres::PgProgressRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceBody {
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub activity_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub value: f32,
    pub derivation_version: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationBody {
    pub objectives: Vec<ObjectiveActivityBody>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveActivityBody {
    pub objective_id: Uuid,
    pub activity_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreakBody {
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub qualifying_event_key: String,
    pub learner_timezone: String,
    pub qualifying_day: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub activity_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub value: f32,
    pub derivation_version: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotResponse {
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub mastery: f32,
    pub confidence: f32,
    pub evidence_count: u32,
    pub calculated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationResponse {
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub activity_id: Uuid,
    pub reason: String,
    pub evidence_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreakResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub qualifying_event_key: String,
    pub qualifying_day: String,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/progress/evidence", post(record_evidence))
        .route(
            "/v1/progress/{journey_id}/objectives/{objective_id}",
            get(snapshot),
        )
        .route("/v1/progress/{journey_id}/recommendation", post(recommend))
        .route("/v1/progress/streaks", post(record_streak))
        .with_state(state)
}

#[utoipa::path(post, path = "/v1/progress/evidence", request_body = EvidenceBody, responses((status = 200, body = EvidenceResponse)), security(("bearer" = [])), tag = "progress")]
pub async fn record_evidence(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<EvidenceBody>,
) -> Result<Json<EvidenceResponse>, ApiError> {
    let evidence = PgProgressRepository::new(state.pool)
        .record_evidence(MasteryEvidenceInput {
            subject_user_id: auth.owner_id(),
            journey_id: body.journey_id,
            objective_id: body.objective_id,
            activity_id: body.activity_id,
            attempt_id: body.attempt_id,
            value: body.value,
            derivation_version: body.derivation_version,
        })
        .await
        .map_err(map_progress_error)?;
    Ok(Json(EvidenceResponse {
        id: evidence.id,
        journey_id: evidence.input.journey_id,
        objective_id: evidence.input.objective_id,
        activity_id: evidence.input.activity_id,
        attempt_id: evidence.input.attempt_id,
        value: evidence.input.value,
        derivation_version: evidence.input.derivation_version,
    }))
}

#[utoipa::path(get, path = "/v1/progress/{journey_id}/objectives/{objective_id}", params(("journey_id" = Uuid, Path), ("objective_id" = Uuid, Path)), responses((status = 200, body = SnapshotResponse)), security(("bearer" = [])), tag = "progress")]
pub async fn snapshot(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((journey_id, objective_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SnapshotResponse>, ApiError> {
    let value = PgProgressRepository::new(state.pool)
        .snapshot(auth.owner_id(), journey_id, objective_id)
        .await
        .map_err(map_progress_error)?;
    Ok(Json(SnapshotResponse {
        journey_id,
        objective_id,
        mastery: value.mastery,
        confidence: value.confidence,
        evidence_count: value.evidence_count,
        calculated_at: value
            .calculated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    }))
}

#[utoipa::path(post, path = "/v1/progress/{journey_id}/recommendation", params(("journey_id" = Uuid, Path)), request_body = RecommendationBody, responses((status = 200, body = RecommendationResponse)), security(("bearer" = [])), tag = "progress")]
pub async fn recommend(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
    Json(body): Json<RecommendationBody>,
) -> Result<Json<RecommendationResponse>, ApiError> {
    let objectives = body
        .objectives
        .into_iter()
        .map(|value| (value.objective_id, value.activity_id))
        .collect::<Vec<_>>();
    let value = PgProgressRepository::new(state.pool)
        .recommend_weakest(auth.owner_id(), journey_id, &objectives)
        .await
        .map_err(map_progress_error)?;
    Ok(Json(RecommendationResponse {
        journey_id,
        objective_id: value.objective_id,
        activity_id: value.activity_id,
        reason: value.reason,
        evidence_ids: value.evidence_ids,
    }))
}

#[utoipa::path(post, path = "/v1/progress/streaks", request_body = StreakBody, responses((status = 200, body = StreakResponse)), security(("bearer" = [])), tag = "progress")]
pub async fn record_streak(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<StreakBody>,
) -> Result<Json<StreakResponse>, ApiError> {
    let day = time::Date::parse(
        &body.qualifying_day,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|_| {
        ApiError::Validation(vec![FieldError {
            field: "qualifyingDay".into(),
            message: "must be an ISO date".into(),
        }])
    })?;
    let value = PgProgressRepository::new(state.pool)
        .record_streak_event(StreakEventInput {
            subject_user_id: auth.owner_id(),
            journey_id: body.journey_id,
            activity_id: body.activity_id,
            qualifying_event_key: body.qualifying_event_key,
            learner_timezone: body.learner_timezone,
            qualifying_day: day,
        })
        .await
        .map_err(map_progress_error)?;
    Ok(Json(StreakResponse {
        id: value.id,
        journey_id: value.input.journey_id,
        activity_id: value.input.activity_id,
        qualifying_event_key: value.input.qualifying_event_key,
        qualifying_day: value.input.qualifying_day.to_string(),
    }))
}

fn map_progress_error(error: ProgressError) -> ApiError {
    match error {
        ProgressError::NotFound | ProgressError::SubjectMismatch => ApiError::NotFound {
            resource: "progress",
        },
        ProgressError::EmptyField { field } => ApiError::Validation(vec![FieldError {
            field: field.into(),
            message: "must not be empty".into(),
        }]),
        ProgressError::InvalidValue => ApiError::Validation(vec![FieldError {
            field: "value".into(),
            message: "must be between 0 and 1".into(),
        }]),
        ProgressError::DuplicateStreakEvent => ApiError::IdempotencyConflict,
        ProgressError::Storage(message) => ApiError::Internal(anyhow::anyhow!(message)),
    }
}
