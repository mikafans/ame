//! Unified learning-journey routes.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    routing::post,
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
            ActivityKind, ActivityStatus, CreateLearningSession,
            FinishLearningSession as FinishLearningSessionInput, GoalStatus, JourneyStatus,
            LearningActivity, LearningObjective, LearningRepository, LearningSession,
            LearningSessionStatus, ObjectiveStatus,
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
pub struct LearningSessionResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub subject_user_id: Uuid,
    pub actor_identity_id: Uuid,
    pub status: LearningSessionStatus,
    #[schema(value_type = Object)]
    pub question_plan: serde_json::Value,
    #[schema(value_type = Option<Object>)]
    pub result: Option<serde_json::Value>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub finished_at: Option<OffsetDateTime>,
}

#[derive(Debug, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FinishLearningSessionBody {
    #[schema(value_type = Object)]
    pub result: serde_json::Value,
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
        .route("/v1/learning/sessions/{id}", get(get_learning_session))
        .route(
            "/v1/learning/sessions/{id}/finish",
            axum::routing::post(finish_learning_session),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
            post(start_activity),
        )
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

/// POST /v1/learning/journeys/{journey_id}/activities/{activity_id}/start — start or resume an activity session.
#[utoipa::path(
    post,
    path = "/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
    params(
        ("journey_id" = Uuid, Path, description = "Learning journey ID"),
        ("activity_id" = Uuid, Path, description = "Learning activity ID")
    ),
    responses(
        (status = 201, description = "Learning session started or resumed", body = LearningSessionResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 404, description = "Activity does not exist for this learner"),
        (status = 422, description = "Activity is not ready")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn start_activity(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((journey_id, activity_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<LearningSessionResponse>), ApiError> {
    if !auth
        .token_scopes
        .contains(&crate::domain::user::Scope::AttemptWrite)
        && !auth
            .token_scopes
            .contains(&crate::domain::user::Scope::Admin)
    {
        return Err(ApiError::ScopeRequired(std::borrow::Cow::Borrowed(
            "attempt.write",
        )));
    }
    let repository = PgLearningRepository::new(state.pool);
    let session = repository
        .start_learning_session(CreateLearningSession {
            journey_id,
            activity_id,
            subject_user_id: auth.owner_id(),
            actor_identity_id: auth.user.id,
            question_plan: serde_json::json!([]),
        })
        .await
        .map_err(map_learning_error)?;
    Ok((StatusCode::CREATED, Json(session_response(session))))
}

/// GET /v1/learning/sessions/{id} — retrieve a learner's resumable session.
#[utoipa::path(
    get,
    path = "/v1/learning/sessions/{id}",
    params(("id" = Uuid, Path, description = "Learning session ID")),
    responses(
        (status = 200, description = "Learning session", body = LearningSessionResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Session does not exist for this learner")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_learning_session(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<LearningSessionResponse>, ApiError> {
    let repository = PgLearningRepository::new(state.pool);
    let session = repository
        .get_learning_session(auth.owner_id(), id)
        .await
        .map_err(map_learning_error)?;
    Ok(Json(session_response(session)))
}

/// POST /v1/learning/sessions/{id}/finish — finish the current learning activity.
#[utoipa::path(
    post,
    path = "/v1/learning/sessions/{id}/finish",
    params(("id" = Uuid, Path, description = "Learning session ID")),
    request_body = FinishLearningSessionBody,
    responses(
        (status = 200, description = "Finished learning session", body = LearningSessionResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 404, description = "Session does not exist for this learner"),
        (status = 409, description = "Session is already finished")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn finish_learning_session(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<FinishLearningSessionBody>,
) -> Result<Json<LearningSessionResponse>, ApiError> {
    if !auth
        .token_scopes
        .contains(&crate::domain::user::Scope::AttemptWrite)
        && !auth
            .token_scopes
            .contains(&crate::domain::user::Scope::Admin)
    {
        return Err(ApiError::ScopeRequired(std::borrow::Cow::Borrowed(
            "attempt.write",
        )));
    }
    let repository = PgLearningRepository::new(state.pool);
    let session = repository
        .finish_learning_session(FinishLearningSessionInput {
            subject_user_id: auth.owner_id(),
            session_id: id,
            result: body.result,
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(session_response(session)))
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

fn session_response(session: LearningSession) -> LearningSessionResponse {
    LearningSessionResponse {
        id: session.id,
        journey_id: session.journey_id,
        activity_id: session.activity_id,
        subject_user_id: session.subject_user_id,
        actor_identity_id: session.actor_identity_id,
        status: session.status,
        question_plan: session.question_plan,
        result: session.result,
        started_at: session.started_at,
        finished_at: session.finished_at,
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
        crate::domain::learning::LearningRepositoryError::ActivityNotReady => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "activityId".to_string(),
                message: "activity is not ready to start".to_string(),
            }])
        }
        crate::domain::learning::LearningRepositoryError::LearningSessionFinished => {
            ApiError::SessionFinished
        }
        other => ApiError::Internal(other.into()),
    }
}
