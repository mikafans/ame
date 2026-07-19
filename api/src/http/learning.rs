//! Unified learning-journey routes.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::ApiError,
        learning::{
            ActivityKind, ActivityStatus, GoalStatus, JourneyStatus, LearningActivity,
            LearningObjective, LearningRepository, ObjectiveStatus,
        },
    },
    http::AppState,
    learning_postgres::PgLearningRepository,
};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningGoalResponse {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub template_version_id: Option<Uuid>,
    pub raw_intent: String,
    pub normalized_statement: String,
    pub status: GoalStatus,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningObjectiveResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub verb: String,
    pub statement: String,
    pub success_criteria: String,
    pub order_index: i32,
    pub status: ObjectiveStatus,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningActivityResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub source_run_id: Option<Uuid>,
    pub kind: ActivityKind,
    pub title: String,
    pub order_index: i32,
    pub payload_schema_version: i32,
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
    pub objective_ids: Vec<Uuid>,
    pub status: ActivityStatus,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningJourneyResponse {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub promise: String,
    pub status: JourneyStatus,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    pub goal: LearningGoalResponse,
    pub objectives: Vec<LearningObjectiveResponse>,
    pub activities: Vec<LearningActivityResponse>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/learning/journeys/{id}", get(get_journey))
        .with_state(state)
}

/// GET /v1/learning/journeys/{id} — retrieve the caller's resumable journey.
#[utoipa::path(
    get,
    path = "/v1/learning/journeys/{id}",
    params(("id" = Uuid, Path, description = "Learning journey ID")),
    responses(
        (status = 200, description = "Learning journey and its goal", body = LearningJourneyResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Journey does not exist for this learner")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_journey(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<LearningJourneyResponse>, ApiError> {
    let repository = PgLearningRepository::new(state.pool);
    let journey = repository
        .get_journey(auth.owner_id(), id)
        .await
        .map_err(map_learning_error)?;
    let goal = repository
        .get_goal(auth.owner_id(), journey.goal_id)
        .await
        .map_err(map_learning_error)?;
    let objectives = repository
        .list_objectives(auth.owner_id(), journey.id)
        .await
        .map_err(map_learning_error)?;
    let activities = repository
        .list_activities(auth.owner_id(), journey.id)
        .await
        .map_err(map_learning_error)?;

    Ok(Json(LearningJourneyResponse {
        id: journey.id,
        goal_id: journey.goal_id,
        subject_user_id: journey.subject_user_id,
        source_actor_id: journey.source_actor_id,
        promise: journey.promise,
        status: journey.status,
        created_at: journey.created_at,
        goal: LearningGoalResponse {
            id: goal.id,
            subject_user_id: goal.subject_user_id,
            source_actor_id: goal.source_actor_id,
            template_version_id: goal.template_version_id,
            raw_intent: goal.raw_intent,
            normalized_statement: goal.normalized_statement,
            status: goal.status,
            created_at: goal.created_at,
        },
        objectives: objectives.into_iter().map(objective_response).collect(),
        activities: activities.into_iter().map(activity_response).collect(),
    }))
}

fn objective_response(objective: LearningObjective) -> LearningObjectiveResponse {
    LearningObjectiveResponse {
        id: objective.id,
        journey_id: objective.journey_id,
        subject_user_id: objective.subject_user_id,
        verb: objective.verb,
        statement: objective.statement,
        success_criteria: objective.success_criteria,
        order_index: objective.order_index,
        status: objective.status,
        created_at: objective.created_at,
    }
}

fn activity_response(activity: LearningActivity) -> LearningActivityResponse {
    LearningActivityResponse {
        id: activity.id,
        journey_id: activity.journey_id,
        subject_user_id: activity.subject_user_id,
        source_actor_id: activity.source_actor_id,
        source_run_id: activity.source_run_id,
        kind: activity.kind,
        title: activity.title,
        order_index: activity.order_index,
        payload_schema_version: activity.payload_schema_version,
        payload: activity.payload,
        objective_ids: activity.objective_ids,
        status: activity.status,
        created_at: activity.created_at,
        updated_at: activity.updated_at,
    }
}

fn map_learning_error(error: crate::domain::learning::LearningRepositoryError) -> ApiError {
    match error {
        crate::domain::learning::LearningRepositoryError::NotFound { resource } => {
            ApiError::NotFound { resource }
        }
        crate::domain::learning::LearningRepositoryError::SubjectMismatch => ApiError::NotFound {
            resource: "journey",
        },
        other => ApiError::Internal(other.into()),
    }
}
