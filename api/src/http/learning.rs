//! Unified learning-journey routes.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::{ApiError, FieldError},
        generation::{GenerationRepository, validate_content_run},
        learning::{
            ActivityKind, ActivityPublicationStatus, ActivityStatus, AuthorActivityContent,
            AuthorActivityRubric, CreateActivity, CreateChapter, CreateLearningSession,
            CreateObjective, FinishLearningSession as FinishLearningSessionInput, GoalStatus,
            JourneyOrigin, JourneyStatus, LearningActivity, LearningChapter, LearningObjective,
            LearningRepository, LearningSession, LearningSessionStatus, ObjectiveStatus,
            TransitionActivityPublication,
        },
        task::{TaskRubric, validate_task_rubric},
    },
    http::AppState,
    learning::load_journey_manifest,
    learning_postgres::PgLearningRepository,
    progress::ProgressRepository,
};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningGoalResponse {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub template_version_id: Option<Uuid>,
    pub catalog_entry_id: Option<String>,
    pub catalog_entry_version: Option<u32>,
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
    pub course_revision_id: Option<Uuid>,
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

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningActivityResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub course_revision_id: Option<Uuid>,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub kind: ActivityKind,
    pub title: String,
    pub order_index: i32,
    pub payload_schema_version: i32,
    pub content_version: i32,
    pub publication_status: crate::domain::learning::ActivityPublicationStatus,
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
    pub objective_ids: Vec<Uuid>,
    pub status: ActivityStatus,
    /// Structured scoring rubric for task/application activities, if authored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rubric: Option<TaskRubric>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningChapterResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub course_revision_id: Option<Uuid>,
    pub subject_user_id: Uuid,
    pub title: String,
    pub summary: String,
    pub order_index: i32,
    pub activities: Vec<LearningActivityResponse>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
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

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningRecommendationResponse {
    pub activity_id: Uuid,
    pub objective_id: Uuid,
    pub title: String,
    pub objective_ids: Vec<Uuid>,
    pub evidence_ids: Vec<Uuid>,
    pub rationale: String,
    pub based_on_session_id: Option<Uuid>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FinishLearningSessionBody {
    pub completed: bool,
    pub responses: Vec<LearningResponseBody>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningResponseBody {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningJourneyResponse {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub promise: String,
    pub origin: JourneyOrigin,
    pub status: JourneyStatus,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    pub goal: LearningGoalResponse,
    pub objectives: Vec<LearningObjectiveResponse>,
    pub chapters: Vec<LearningChapterResponse>,
    pub ungrouped_activities: Vec<LearningActivityResponse>,
    /// Flat compatibility view; new clients should use chapters.activities.
    pub activities: Vec<LearningActivityResponse>,
    pub recommendation: Option<LearningRecommendationResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearningJourneySummaryResponse {
    pub id: Uuid,
    pub promise: String,
    pub origin: JourneyOrigin,
    pub status: JourneyStatus,
    pub raw_intent: String,
    pub goal: LearningGoalResponse,
    pub next_activity_id: Option<Uuid>,
    pub next_activity_title: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

/// A published course available to the signed-in learner. This deliberately
/// excludes draft and review revisions: authors can inspect those through the
/// authoring API, but they are never learner material.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerCourseSummaryResponse {
    pub journey_id: Uuid,
    pub title: String,
    pub goal: String,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_learning_at: Option<OffsetDateTime>,
    pub progress: LearnerCompletionResponse,
    pub current_module: Option<LearnerModuleReferenceResponse>,
    pub next: Option<LearnerNextActivityResponse>,
    pub review: LearnerReviewSummaryResponse,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerCompletionResponse {
    pub completed: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerModuleReferenceResponse {
    pub id: Uuid,
    pub title: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerNextActivityResponse {
    pub activity_id: Uuid,
    pub title: String,
    pub estimated_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerReviewSummaryResponse {
    pub due_count: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerViewResponse {
    pub journey_id: Uuid,
    pub title: String,
    pub status: String,
    pub published_revision_id: Uuid,
    pub tabs: Vec<String>,
    pub next: Option<LearnerNextActivityResponse>,
    pub course_url: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerCourseResponse {
    pub journey_id: Uuid,
    pub title: String,
    pub goal: String,
    pub promise: String,
    pub audience: Option<String>,
    pub estimated_minutes: Option<u32>,
    pub prerequisites: Vec<String>,
    pub outcomes: Vec<LearnerOutcomeResponse>,
    pub modules: Vec<LearnerModuleResponse>,
    pub publication: LearnerPublicationResponse,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerOutcomeResponse {
    pub id: Uuid,
    pub statement: String,
    pub success_criteria: String,
    pub position: i32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerModuleResponse {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub position: i32,
    pub activity_kinds: Vec<ActivityKind>,
    pub completion: LearnerCompletionResponse,
    pub state: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerPublicationResponse {
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub published_at: OffsetDateTime,
    pub reviewed: bool,
    pub source_reviewed: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerProgressViewResponse {
    pub journey_id: Uuid,
    pub outcomes: Vec<LearnerProgressOutcomeResponse>,
    pub attempts: Vec<LearnerAttemptSummaryResponse>,
    pub due_reviews: Vec<LearnerDueReviewResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerProgressOutcomeResponse {
    pub objective_id: Uuid,
    pub statement: String,
    pub evidence_count: u32,
    pub evidence_status: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerAttemptSummaryResponse {
    pub id: Uuid,
    pub activity_id: Uuid,
    pub activity_title: String,
    pub status: String,
    pub score: Option<f32>,
    pub max_score: Option<f32>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerDueReviewResponse {
    pub id: Uuid,
    pub activity_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub due_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerResourcesResponse {
    pub journey_id: Uuid,
    pub sources: Vec<LearnerResourceResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerResourceResponse {
    pub title: String,
    pub locator: String,
    pub quote: String,
    pub relevance: String,
    pub grounding_status: String,
    pub license_status: String,
    pub license_name: Option<String>,
    pub license_url: Option<String>,
    pub activity_ids: Vec<Uuid>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/learning/courses", get(list_learner_courses))
        .route("/v1/learning/journeys", get(list_journeys))
        .route(
            "/v1/learning/journeys/{id}/learner-view",
            get(get_learner_view),
        )
        .route("/v1/learning/journeys/{id}/course", get(get_learner_course))
        .route(
            "/v1/learning/journeys/{id}/progress-view",
            get(get_learner_progress_view),
        )
        .route(
            "/v1/learning/journeys/{id}/resources",
            get(get_learner_resources),
        )
        .route("/v1/learning/journeys/{id}", get(get_journey))
        .route(
            "/v1/learning/journeys/{journey_id}/objectives",
            post(create_objective),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/chapters",
            post(create_chapter),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/activities",
            post(create_activity),
        )
        .route("/v1/learning/sessions/{id}", get(get_learning_session))
        .route(
            "/v1/learning/sessions/{id}/finish",
            axum::routing::post(finish_learning_session),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
            post(start_activity),
        )
        .route(
            "/v1/learning/activities/{activity_id}/content",
            get(get_activity_content),
        )
        .route(
            "/v1/learning/activities/{activity_id}/content",
            axum::routing::patch(author_activity_content),
        )
        .route(
            "/v1/learning/activities/{activity_id}/rubric",
            axum::routing::patch(author_activity_rubric),
        )
        .route(
            "/v1/learning/activities/{activity_id}/review",
            post(submit_activity_for_review),
        )
        .route(
            "/v1/learning/activities/{activity_id}/publish",
            post(publish_activity),
        )
        .with_state(state)
}

#[derive(Debug)]
struct PublishedLearnerCourse {
    journey_id: Uuid,
    revision_id: Uuid,
    title: String,
    goal: String,
    promise: String,
    status: String,
    brief: Value,
    source_references: Vec<Uuid>,
    published_at: OffsetDateTime,
    last_learning_at: Option<OffsetDateTime>,
}

/// GET /api/v1/learning/courses — the learner's published course library.
#[utoipa::path(
    get,
    path = "/api/v1/learning/courses",
    responses((status = 200, body = [LearnerCourseSummaryResponse])),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn list_learner_courses(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<LearnerCourseSummaryResponse>>, ApiError> {
    let courses = load_published_learner_courses(&state.pool, auth.owner_id()).await?;
    let mut response = Vec::with_capacity(courses.len());
    for course in courses {
        response.push(course_summary(&state.pool, auth.owner_id(), &course).await?);
    }
    Ok(Json(response))
}

/// GET /api/v1/learning/journeys/{id}/learner-view — stable selected-course shell.
#[utoipa::path(
    get,
    path = "/api/v1/learning/journeys/{id}/learner-view",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = LearnerViewResponse), (status = 404, description = "No published owned course")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_learner_view(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
) -> Result<Json<LearnerViewResponse>, ApiError> {
    let course = load_published_learner_course(&state.pool, auth.owner_id(), journey_id).await?;
    let summary = course_summary(&state.pool, auth.owner_id(), &course).await?;
    Ok(Json(LearnerViewResponse {
        journey_id,
        title: course.title,
        status: course.status,
        published_revision_id: course.revision_id,
        tabs: vec![
            "learn".into(),
            "course".into(),
            "progress".into(),
            "resources".into(),
        ],
        next: summary.next,
        course_url: format!("/learning/journeys/{journey_id}"),
    }))
}

/// GET /api/v1/learning/journeys/{id}/course — published learner syllabus.
#[utoipa::path(
    get,
    path = "/api/v1/learning/journeys/{id}/course",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = LearnerCourseResponse), (status = 404, description = "No published owned course")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_learner_course(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
) -> Result<Json<LearnerCourseResponse>, ApiError> {
    let course = load_published_learner_course(&state.pool, auth.owner_id(), journey_id).await?;
    let objectives = sqlx::query(
        "SELECT id, statement, success_criteria, order_index FROM tb_journey_objectives WHERE course_revision_id = $1 AND subject_user_id = $2 ORDER BY order_index",
    )
    .bind(course.revision_id)
    .bind(auth.owner_id())
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?
    .into_iter()
    .map(|row| LearnerOutcomeResponse {
        id: row.get("id"),
        statement: row.get("statement"),
        success_criteria: row.get("success_criteria"),
        position: row.get("order_index"),
    })
    .collect();
    let chapters = sqlx::query(
        "SELECT id, title, summary, order_index FROM tb_journey_chapters WHERE course_revision_id = $1 AND subject_user_id = $2 ORDER BY order_index",
    )
    .bind(course.revision_id)
    .bind(auth.owner_id())
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    let mut modules = Vec::with_capacity(chapters.len());
    let mut current_set = false;
    for chapter in chapters {
        let chapter_id: Uuid = chapter.get("id");
        let activities = sqlx::query(
            "SELECT kind, status FROM tb_activities WHERE chapter_id = $1 AND course_revision_id = $2 AND subject_user_id = $3 AND publication_status = 'published' ORDER BY order_index",
        )
        .bind(chapter_id)
        .bind(course.revision_id)
        .bind(auth.owner_id())
        .fetch_all(&state.pool)
        .await
        .map_err(database_error)?;
        let total = activities.len() as u32;
        let completed = activities
            .iter()
            .filter(|activity| activity.get::<String, _>("status") == "completed")
            .count() as u32;
        let has_ready = activities.iter().any(|activity| {
            matches!(
                activity.get::<String, _>("status").as_str(),
                "ready" | "in_progress"
            )
        });
        let state = if total > 0 && completed == total {
            "completed"
        } else if !current_set && (has_ready || completed < total) {
            current_set = true;
            "current"
        } else if has_ready {
            "ready"
        } else {
            "planned"
        };
        modules.push(LearnerModuleResponse {
            id: chapter_id,
            title: chapter.get("title"),
            summary: chapter.get("summary"),
            position: chapter.get("order_index"),
            activity_kinds: activities
                .iter()
                .filter_map(|activity| {
                    serde_json::from_value(Value::String(activity.get("kind"))).ok()
                })
                .collect(),
            completion: LearnerCompletionResponse { completed, total },
            state: state.into(),
        });
    }
    Ok(Json(LearnerCourseResponse {
        journey_id,
        title: course.title,
        goal: course.goal,
        promise: course.promise,
        audience: course
            .brief
            .get("audience")
            .and_then(Value::as_str)
            .map(str::to_owned),
        estimated_minutes: course
            .brief
            .get("estimatedMinutes")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
        prerequisites: string_array(&course.brief, "prerequisites"),
        outcomes: objectives,
        modules,
        publication: LearnerPublicationResponse {
            published_at: course.published_at,
            reviewed: true,
            source_reviewed: !course.source_references.is_empty(),
        },
    }))
}

/// GET /api/v1/learning/journeys/{id}/progress-view — evidence first; no
/// synthetic mastery value appears before recorded evidence exists.
#[utoipa::path(
    get,
    path = "/api/v1/learning/journeys/{id}/progress-view",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = LearnerProgressViewResponse), (status = 404, description = "No published owned course")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_learner_progress_view(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
) -> Result<Json<LearnerProgressViewResponse>, ApiError> {
    let course = load_published_learner_course(&state.pool, auth.owner_id(), journey_id).await?;
    let outcomes = sqlx::query(
        "SELECT objective.id, objective.statement, COUNT(evidence.id)::bigint AS evidence_count FROM tb_journey_objectives objective LEFT JOIN tb_mastery_evidence evidence ON evidence.objective_id = objective.id AND evidence.subject_user_id = $2 AND evidence.journey_id = $3 WHERE objective.course_revision_id = $1 AND objective.subject_user_id = $2 GROUP BY objective.id, objective.statement, objective.order_index ORDER BY objective.order_index",
    )
    .bind(course.revision_id)
    .bind(auth.owner_id())
    .bind(journey_id)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?
    .into_iter()
    .map(|row| {
        let evidence_count = row.get::<i64, _>("evidence_count") as u32;
        LearnerProgressOutcomeResponse {
            objective_id: row.get("id"),
            statement: row.get("statement"),
            evidence_count,
            evidence_status: if evidence_count == 0 { "not_assessed_yet" } else { "evidence_recorded" }.into(),
        }
    })
    .collect();
    let attempts = sqlx::query(
        "SELECT attempt.id, attempt.activity_id, activity.title AS activity_title, attempt.status, attempt.score::float4 AS score, attempt.max_score::float4 AS max_score FROM tb_attempts attempt JOIN tb_activities activity ON activity.id = attempt.activity_id WHERE attempt.subject_user_id = $1 AND activity.journey_id = $2 AND activity.course_revision_id = $3 ORDER BY attempt.created_at DESC",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .bind(course.revision_id)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?
    .into_iter()
    .map(|row| LearnerAttemptSummaryResponse {
        id: row.get("id"), activity_id: row.get("activity_id"), activity_title: row.get("activity_title"), status: row.get("status"), score: row.get("score"), max_score: row.get("max_score"),
    })
    .collect();
    let due_reviews = sqlx::query(
        "SELECT id, activity_id, due_at FROM tb_review_items WHERE subject_user_id = $1 AND journey_id = $2 AND due_at <= now() ORDER BY due_at",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?
    .into_iter()
    .map(|row| LearnerDueReviewResponse { id: row.get("id"), activity_id: row.get("activity_id"), due_at: row.get("due_at") })
    .collect();
    Ok(Json(LearnerProgressViewResponse {
        journey_id,
        outcomes,
        attempts,
        due_reviews,
    }))
}

/// GET /api/v1/learning/journeys/{id}/resources — course citation metadata,
/// never imported document bytes or storage hashes.
#[utoipa::path(
    get,
    path = "/api/v1/learning/journeys/{id}/resources",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = LearnerResourcesResponse), (status = 404, description = "No published owned course")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_learner_resources(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
) -> Result<Json<LearnerResourcesResponse>, ApiError> {
    let course = load_published_learner_course(&state.pool, auth.owner_id(), journey_id).await?;
    let rows = sqlx::query(
        "SELECT citation.id, source.locator, citation.quote, citation.grounding_note, citation.grounding_status, citation.license_status, citation.license_name, citation.license_url FROM tb_citations citation JOIN tb_source_snapshots snapshot ON snapshot.id = citation.snapshot_id JOIN tb_sources source ON source.id = snapshot.source_id WHERE citation.subject_user_id = $1 AND citation.id = ANY($2)",
    )
    .bind(auth.owner_id())
    .bind(&course.source_references)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    let activity_rows = sqlx::query(
        "SELECT id, payload FROM tb_activities WHERE journey_id = $1 AND course_revision_id = $2 AND subject_user_id = $3 AND publication_status = 'published'",
    )
    .bind(journey_id)
    .bind(course.revision_id)
    .bind(auth.owner_id())
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    let sources = rows
        .into_iter()
        .map(|row| {
            let citation_id: Uuid = row.get("id");
            let citation_id = citation_id.to_string();
            let activity_ids = activity_rows
                .iter()
                .filter_map(|activity| {
                    let payload: Value = activity.get("payload");
                    let references = payload
                        .get("contentProvenance")?
                        .get("sourceReferences")?
                        .as_array()?;
                    references
                        .iter()
                        .any(|reference| reference.as_str() == Some(&citation_id))
                        .then(|| activity.get("id"))
                })
                .collect();
            let locator: String = row.get("locator");
            LearnerResourceResponse {
                title: resource_title(&locator),
                locator,
                quote: row.get("quote"),
                relevance: row.get("grounding_note"),
                grounding_status: row.get("grounding_status"),
                license_status: row.get("license_status"),
                license_name: row.get("license_name"),
                license_url: row.get("license_url"),
                activity_ids,
            }
        })
        .collect();
    Ok(Json(LearnerResourcesResponse {
        journey_id,
        sources,
    }))
}

async fn load_published_learner_courses(
    pool: &PgPool,
    subject_user_id: Uuid,
) -> Result<Vec<PublishedLearnerCourse>, ApiError> {
    let rows = sqlx::query(
        "SELECT journey.id AS journey_id, revision.id AS revision_id, revision.brief, revision.source_references, revision.published_at, journey.promise, journey.status, goal.normalized_statement AS goal, latest.last_learning_at FROM tb_course_revisions revision JOIN tb_learning_journeys journey ON journey.id = revision.journey_id JOIN tb_learning_goals goal ON goal.id = journey.goal_id LEFT JOIN LATERAL (SELECT MAX(COALESCE(session.finished_at, session.started_at)) AS last_learning_at FROM tb_learning_sessions session WHERE session.subject_user_id = journey.subject_user_id AND session.journey_id = journey.id) latest ON TRUE WHERE revision.subject_user_id = $1 AND revision.status = 'published' ORDER BY latest.last_learning_at DESC NULLS LAST, journey.created_at DESC",
    )
    .bind(subject_user_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    rows.into_iter().map(published_course_row).collect()
}

async fn load_published_learner_course(
    pool: &PgPool,
    subject_user_id: Uuid,
    journey_id: Uuid,
) -> Result<PublishedLearnerCourse, ApiError> {
    let row = sqlx::query(
        "SELECT journey.id AS journey_id, revision.id AS revision_id, revision.brief, revision.source_references, revision.published_at, journey.promise, journey.status, goal.normalized_statement AS goal, latest.last_learning_at FROM tb_course_revisions revision JOIN tb_learning_journeys journey ON journey.id = revision.journey_id JOIN tb_learning_goals goal ON goal.id = journey.goal_id LEFT JOIN LATERAL (SELECT MAX(COALESCE(session.finished_at, session.started_at)) AS last_learning_at FROM tb_learning_sessions session WHERE session.subject_user_id = journey.subject_user_id AND session.journey_id = journey.id) latest ON TRUE WHERE revision.subject_user_id = $1 AND revision.journey_id = $2 AND revision.status = 'published'",
    )
    .bind(subject_user_id)
    .bind(journey_id)
    .fetch_optional(pool)
    .await
    .map_err(database_error)?
    .ok_or(ApiError::NotFound {
        resource: "published learning course",
    })?;
    published_course_row(row)
}

fn published_course_row(row: sqlx::postgres::PgRow) -> Result<PublishedLearnerCourse, ApiError> {
    let brief: Value = row.get("brief");
    let title = brief
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("published course lacks a title")))?
        .to_owned();
    let source_references: Vec<String> = serde_json::from_value(row.get("source_references"))
        .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?;
    let source_references = source_references
        .into_iter()
        .map(|reference| Uuid::parse_str(&reference))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?;
    Ok(PublishedLearnerCourse {
        journey_id: row.get("journey_id"),
        revision_id: row.get("revision_id"),
        title,
        goal: row.get("goal"),
        promise: row.get("promise"),
        status: row.get("status"),
        brief,
        source_references,
        published_at: row.get("published_at"),
        last_learning_at: row.get("last_learning_at"),
    })
}

async fn course_summary(
    pool: &PgPool,
    subject_user_id: Uuid,
    course: &PublishedLearnerCourse,
) -> Result<LearnerCourseSummaryResponse, ApiError> {
    let counts = sqlx::query(
        "SELECT COUNT(*)::bigint AS total, COUNT(*) FILTER (WHERE status = 'completed')::bigint AS completed FROM tb_activities WHERE course_revision_id = $1 AND subject_user_id = $2 AND publication_status = 'published'",
    )
    .bind(course.revision_id)
    .bind(subject_user_id)
    .fetch_one(pool)
    .await
    .map_err(database_error)?;
    let next_row = sqlx::query(
        "SELECT id, title, payload FROM tb_activities WHERE course_revision_id = $1 AND subject_user_id = $2 AND publication_status = 'published' AND status IN ('ready', 'in_progress') ORDER BY order_index LIMIT 1",
    )
    .bind(course.revision_id)
    .bind(subject_user_id)
    .fetch_optional(pool)
    .await
    .map_err(database_error)?;
    let next = next_row.map(|row| {
        let payload: Value = row.get("payload");
        LearnerNextActivityResponse {
            activity_id: row.get("id"),
            title: row.get("title"),
            estimated_minutes: payload
                .get("estimatedMinutes")
                .or_else(|| payload.get("estimated_minutes"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
        }
    });
    let current_module = sqlx::query(
        "SELECT chapter.id, chapter.title, chapter.order_index FROM tb_journey_chapters chapter WHERE chapter.course_revision_id = $1 AND chapter.subject_user_id = $2 AND EXISTS (SELECT 1 FROM tb_activities activity WHERE activity.chapter_id = chapter.id AND activity.course_revision_id = $1 AND activity.subject_user_id = $2 AND activity.publication_status = 'published' AND activity.status <> 'completed') ORDER BY chapter.order_index LIMIT 1",
    )
    .bind(course.revision_id)
    .bind(subject_user_id)
    .fetch_optional(pool)
    .await
    .map_err(database_error)?
    .map(|row| LearnerModuleReferenceResponse {
        id: row.get("id"), title: row.get("title"), position: row.get("order_index"),
    });
    let due_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tb_review_items WHERE subject_user_id = $1 AND journey_id = $2 AND due_at <= now()",
    )
    .bind(subject_user_id)
    .bind(course.journey_id)
    .fetch_one(pool)
    .await
    .map_err(database_error)? as u32;
    Ok(LearnerCourseSummaryResponse {
        journey_id: course.journey_id,
        title: course.title.clone(),
        goal: course.goal.clone(),
        status: course.status.clone(),
        last_learning_at: course.last_learning_at,
        progress: LearnerCompletionResponse {
            completed: counts.get::<i64, _>("completed") as u32,
            total: counts.get::<i64, _>("total") as u32,
        },
        current_module,
        next,
        review: LearnerReviewSummaryResponse { due_count },
    })
}

fn string_array(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn resource_title(locator: &str) -> String {
    locator
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(locator)
        .to_owned()
}

fn database_error(error: sqlx::Error) -> ApiError {
    ApiError::Internal(anyhow::Error::new(error))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorActivityContentBody {
    pub revision_id: Option<Uuid>,
    pub generation_run_id: Uuid,
    #[schema(value_type = Object)]
    pub content: Value,
    pub source_references: Vec<String>,
    pub review_status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorActivityRubricBody {
    pub revision_id: Option<Uuid>,
    pub generation_run_id: Uuid,
    pub rubric: TaskRubric,
    pub source_references: Vec<String>,
    pub review_status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevisionTargetBody {
    pub revision_id: Option<Uuid>,
}

/// A new chapter is always appended after the journey's existing chapters.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateChapterBody {
    pub revision_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
}

/// A course author declares observable outcomes before linking instruction,
/// formative checks, and mastery work to them.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateObjectiveBody {
    pub revision_id: Option<Uuid>,
    pub verb: String,
    pub statement: String,
    pub success_criteria: String,
}

/// A course author supplies the activity semantics and objective links; AME owns
/// ordering, versioning, publication, and the learner identity.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateActivityBody {
    pub revision_id: Option<Uuid>,
    pub chapter_id: Option<Uuid>,
    pub kind: ActivityKind,
    pub title: String,
    #[schema(value_type = Object)]
    pub payload: Value,
    pub objective_ids: Vec<Uuid>,
    pub status: ActivityStatus,
}

/// GET /api/v1/learning/journeys — list the caller's learner-owned journeys.
#[utoipa::path(
    get,
    path = "/api/v1/learning/journeys",
    responses(
        (status = 200, description = "Learner journeys", body = [LearningJourneySummaryResponse]),
        (status = 401, description = "Missing or invalid token")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn list_journeys(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<LearningJourneySummaryResponse>>, ApiError> {
    let repository = PgLearningRepository::new(state.pool);
    let journeys = repository
        .list_journeys(auth.owner_id())
        .await
        .map_err(map_learning_error)?;
    let mut response = Vec::with_capacity(journeys.len());
    for journey in journeys {
        let goal = repository
            .get_goal(auth.owner_id(), journey.goal_id)
            .await
            .map_err(map_learning_error)?;
        let next_activity = repository
            .list_activities(auth.owner_id(), journey.id)
            .await
            .map_err(map_learning_error)?
            .into_iter()
            .find(|activity| {
                matches!(
                    activity.status,
                    ActivityStatus::Ready | ActivityStatus::InProgress
                ) && activity.publication_status == ActivityPublicationStatus::Published
            });
        response.push(LearningJourneySummaryResponse {
            id: journey.id,
            promise: journey.promise,
            origin: journey.origin,
            status: journey.status,
            raw_intent: goal.raw_intent.clone(),
            goal: goal_response(goal),
            next_activity_id: next_activity.as_ref().map(|activity| activity.id),
            next_activity_title: next_activity.map(|activity| activity.title),
            created_at: journey.created_at,
        });
    }
    Ok(Json(response))
}

/// POST /api/v1/learning/journeys/{journey_id}/chapters — append a course chapter.
#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/chapters",
    params(("journey_id" = Uuid, Path, description = "Journey to extend")),
    request_body = CreateChapterBody,
    responses(
        (status = 200, description = "Appended chapter", body = LearningChapterResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Journey does not exist for this learner"),
        (status = 422, description = "Chapter is invalid")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn create_chapter(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
    Json(body): Json<CreateChapterBody>,
) -> Result<Json<LearningChapterResponse>, ApiError> {
    let revision_id = crate::http::courses::require_editable_revision(
        &state.pool,
        auth.owner_id(),
        journey_id,
        body.revision_id,
    )
    .await?;
    let repository = PgLearningRepository::new(state.pool);
    let chapters = match revision_id {
        Some(revision_id) => {
            repository
                .list_chapters_for_revision(auth.owner_id(), journey_id, revision_id)
                .await
        }
        None => repository.list_chapters(auth.owner_id(), journey_id).await,
    }
    .map_err(map_learning_error)?;
    let order_index = chapters.last().map_or(0, |chapter| chapter.order_index + 1);
    let chapter = repository
        .create_chapter(CreateChapter {
            journey_id,
            course_revision_id: revision_id,
            subject_user_id: auth.owner_id(),
            title: body.title,
            summary: body.summary,
            order_index,
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(chapter_response(chapter, &[])))
}

/// POST /api/v1/learning/journeys/{journey_id}/objectives — append a measurable course outcome.
#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/objectives",
    params(("journey_id" = Uuid, Path, description = "Journey to extend")),
    request_body = CreateObjectiveBody,
    responses(
        (status = 200, description = "Appended objective", body = LearningObjectiveResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Journey does not exist for this learner"),
        (status = 422, description = "Objective is invalid")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn create_objective(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
    Json(body): Json<CreateObjectiveBody>,
) -> Result<Json<LearningObjectiveResponse>, ApiError> {
    let revision_id = crate::http::courses::require_editable_revision(
        &state.pool,
        auth.owner_id(),
        journey_id,
        body.revision_id,
    )
    .await?;
    let repository = PgLearningRepository::new(state.pool);
    let objectives = match revision_id {
        Some(revision_id) => {
            repository
                .list_objectives_for_revision(auth.owner_id(), journey_id, revision_id)
                .await
        }
        None => {
            repository
                .list_objectives(auth.owner_id(), journey_id)
                .await
        }
    }
    .map_err(map_learning_error)?;
    let order_index = objectives
        .last()
        .map_or(0, |objective| objective.order_index + 1);
    let objective = repository
        .create_objective(CreateObjective {
            journey_id,
            course_revision_id: revision_id,
            subject_user_id: auth.owner_id(),
            verb: body.verb,
            statement: body.statement,
            success_criteria: body.success_criteria,
            order_index,
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(objective_response(objective)))
}

/// POST /api/v1/learning/journeys/{journey_id}/activities — append a course activity.
#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/activities",
    params(("journey_id" = Uuid, Path, description = "Journey to extend")),
    request_body = CreateActivityBody,
    responses(
        (status = 200, description = "Appended activity", body = LearningActivityResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Journey, chapter, or objective does not exist for this learner"),
        (status = 422, description = "Activity is invalid or cannot be learner-startable")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn create_activity(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
    Json(body): Json<CreateActivityBody>,
) -> Result<Json<LearningActivityResponse>, ApiError> {
    if body.objective_ids.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "objectiveIds".to_string(),
            message: "must link at least one journey objective".to_string(),
        }]));
    }
    if !matches!(
        body.status,
        ActivityStatus::Proposed | ActivityStatus::Ready
    ) {
        return Err(ApiError::Validation(vec![FieldError {
            field: "status".to_string(),
            message: "must be proposed or ready when authoring a course".to_string(),
        }]));
    }

    let revision_id = crate::http::courses::require_editable_revision(
        &state.pool,
        auth.owner_id(),
        journey_id,
        body.revision_id,
    )
    .await?;
    let repository = PgLearningRepository::new(state.pool);
    let activities = match revision_id {
        Some(revision_id) => {
            repository
                .list_activities_for_revision(auth.owner_id(), journey_id, revision_id)
                .await
        }
        None => {
            repository
                .list_activities(auth.owner_id(), journey_id)
                .await
        }
    }
    .map_err(map_learning_error)?;
    let order_index = activities
        .iter()
        .map(|activity| activity.order_index)
        .max()
        .map_or(0, |order_index| order_index + 1);
    let activity = repository
        .create_activity(CreateActivity {
            journey_id,
            course_revision_id: revision_id,
            subject_user_id: auth.owner_id(),
            source_actor_id: auth.actor_identity_id(),
            chapter_id: body.chapter_id,
            kind: body.kind,
            title: body.title,
            order_index,
            payload_schema_version: 1,
            content_version: 1,
            publication_status: ActivityPublicationStatus::Draft,
            payload: body.payload,
            objective_ids: body.objective_ids,
            status: body.status,
            rubric: None,
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(activity_response(activity)))
}

/// POST /api/v1/learning/activities/{activity_id}/review — submit a draft activity for review.
#[utoipa::path(
    post,
    path = "/api/v1/learning/activities/{activity_id}/review",
    params(("activity_id" = Uuid, Path, description = "Draft activity to submit")),
    responses(
        (status = 200, description = "Activity submitted for review", body = LearningActivityResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Activity does not exist for this learner"),
        (status = 409, description = "Activity is not an editable draft")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn submit_activity_for_review(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(activity_id): Path<Uuid>,
    body: Option<Json<RevisionTargetBody>>,
) -> Result<Json<LearningActivityResponse>, ApiError> {
    require_editable_activity_revision(
        &state.pool,
        auth.owner_id(),
        activity_id,
        body.and_then(|Json(body)| body.revision_id),
    )
    .await?;
    let activity = PgLearningRepository::new(state.pool)
        .transition_activity_publication(TransitionActivityPublication {
            subject_user_id: auth.owner_id(),
            activity_id,
            from: ActivityPublicationStatus::Draft,
            to: ActivityPublicationStatus::Review,
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(activity_response(activity)))
}

/// POST /api/v1/learning/activities/{activity_id}/publish — retained only to
/// return a migration-safe error; course revisions are the publication boundary.
#[utoipa::path(
    post,
    path = "/api/v1/learning/activities/{activity_id}/publish",
    params(("activity_id" = Uuid, Path, description = "Reviewed activity to publish")),
    responses(
        (status = 409, description = "Course-level publication validation is required"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Activity does not exist for this learner"),
        (status = 409, description = "Activity is not in review")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn publish_activity(
    State(_state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(_activity_id): Path<Uuid>,
) -> Result<Json<LearningActivityResponse>, ApiError> {
    Err(ApiError::CoursePublicationRequired)
}

/// GET /api/v1/learning/activities/{activity_id}/content — read immutable activity content without starting a session.
#[utoipa::path(
    get,
    path = "/api/v1/learning/activities/{activity_id}/content",
    params(("activity_id" = Uuid, Path, description = "Activity to review")),
    responses(
        (status = 200, description = "Owned activity content", body = LearningActivityResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Activity does not exist for this learner")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_activity_content(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(activity_id): Path<Uuid>,
) -> Result<Json<LearningActivityResponse>, ApiError> {
    let repository = PgLearningRepository::new(state.pool);
    for journey in repository
        .list_journeys(auth.owner_id())
        .await
        .map_err(map_learning_error)?
    {
        if let Some(activity) = repository
            .list_activities(auth.owner_id(), journey.id)
            .await
            .map_err(map_learning_error)?
            .into_iter()
            .find(|activity| activity.id == activity_id)
        {
            return Ok(Json(activity_response(activity)));
        }
    }
    Err(ApiError::NotFound {
        resource: "activity",
    })
}

/// PATCH /api/v1/learning/activities/{activity_id}/content — replace an uncompleted explanation or example with reviewed agent-authored content.
#[utoipa::path(
    patch,
    path = "/api/v1/learning/activities/{activity_id}/content",
    params(("activity_id" = Uuid, Path, description = "Activity to ground")),
    request_body = AuthorActivityContentBody,
    responses(
        (status = 200, description = "Grounded learner activity", body = LearningActivityResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Activity or generation run does not belong to this learner"),
        (status = 409, description = "Generation is not published or activity content is immutable"),
        (status = 422, description = "Content, provenance, or review status is invalid")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn author_activity_content(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(activity_id): Path<Uuid>,
    Json(body): Json<AuthorActivityContentBody>,
) -> Result<Json<LearningActivityResponse>, ApiError> {
    require_editable_activity_revision(&state.pool, auth.owner_id(), activity_id, body.revision_id)
        .await?;
    let generation = crate::generation_postgres::PgGenerationRepository::new(state.pool.clone())
        .get(auth.owner_id(), body.generation_run_id)
        .await
        .map_err(crate::http::generation::map_error)?;
    validate_content_run(
        &generation,
        auth.owner_id(),
        "learning.activity.content.compose",
    )
    .map_err(crate::http::generation::map_error)?;
    crate::http::citations::certify_references(
        state.pool.clone(),
        auth.owner_id(),
        &body.source_references,
    )
    .await?;
    let activity = PgLearningRepository::new(state.pool)
        .author_activity_content(AuthorActivityContent {
            subject_user_id: auth.owner_id(),
            activity_id,
            generation_run_id: body.generation_run_id,
            content: body.content,
            source_references: body.source_references,
            review_status: body.review_status,
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(activity_response(activity)))
}

/// PATCH /api/v1/learning/activities/{activity_id}/rubric — attach a reviewed, source-backed scoring rubric to a task activity.
#[utoipa::path(
    patch,
    path = "/api/v1/learning/activities/{activity_id}/rubric",
    params(("activity_id" = Uuid, Path, description = "Task activity to attach a rubric to")),
    request_body = AuthorActivityRubricBody,
    responses(
        (status = 200, description = "Activity with its attached rubric", body = LearningActivityResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Activity or generation run does not belong to this learner"),
        (status = 409, description = "Generation is not published or activity is completed"),
        (status = 422, description = "Rubric, provenance, or review status is invalid")
    ),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn author_activity_rubric(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(activity_id): Path<Uuid>,
    Json(body): Json<AuthorActivityRubricBody>,
) -> Result<Json<LearningActivityResponse>, ApiError> {
    require_editable_activity_revision(&state.pool, auth.owner_id(), activity_id, body.revision_id)
        .await?;
    validate_task_rubric(&body.rubric).map_err(|error| {
        ApiError::Validation(vec![crate::domain::error::FieldError {
            field: "rubric".to_string(),
            message: error.to_string(),
        }])
    })?;
    let generation = crate::generation_postgres::PgGenerationRepository::new(state.pool.clone())
        .get(auth.owner_id(), body.generation_run_id)
        .await
        .map_err(crate::http::generation::map_error)?;
    validate_content_run(
        &generation,
        auth.owner_id(),
        "learning.activity.rubric.compose",
    )
    .map_err(crate::http::generation::map_error)?;
    crate::http::citations::certify_references(
        state.pool.clone(),
        auth.owner_id(),
        &body.source_references,
    )
    .await?;
    let rubric = serde_json::to_value(&body.rubric)
        .map_err(|error| ApiError::Internal(anyhow::anyhow!(error)))?;
    let activity = PgLearningRepository::new(state.pool)
        .author_activity_rubric(AuthorActivityRubric {
            subject_user_id: auth.owner_id(),
            activity_id,
            generation_run_id: body.generation_run_id,
            rubric,
            source_references: body.source_references,
            review_status: body.review_status,
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(activity_response(activity)))
}

/// GET /api/v1/learning/journeys/{id} — retrieve the caller's resumable journey.
#[utoipa::path(
    get,
    path = "/api/v1/learning/journeys/{id}",
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
    let progress_pool = state.pool.clone();
    let repository = PgLearningRepository::new(state.pool);
    let manifest = load_journey_manifest(&repository, auth.owner_id(), id)
        .await
        .map_err(map_learning_error)?;
    let journey = manifest.journey;
    let goal = manifest.goal;
    let objectives = manifest.objectives;
    let ungrouped_activities = manifest.ungrouped_activities;
    let chapter_manifests = manifest.chapters;
    let activities = chapter_manifests
        .iter()
        .flat_map(|chapter| chapter.activities.iter().cloned())
        .chain(ungrouped_activities.iter().cloned())
        .collect::<Vec<_>>();
    let latest_session = repository
        .latest_finished_learning_session(auth.owner_id(), journey.id)
        .await
        .map_err(map_learning_error)?;
    let activity_responses: Vec<_> = activities.into_iter().map(activity_response).collect();
    let chapter_responses = chapter_manifests
        .into_iter()
        .map(|chapter| chapter_response(chapter.chapter, &activity_responses))
        .collect();
    let recommendation = next_recommendation(
        progress_pool,
        auth.owner_id(),
        journey.id,
        &activity_responses,
        latest_session.as_ref(),
    )
    .await?;

    Ok(Json(LearningJourneyResponse {
        id: journey.id,
        goal_id: journey.goal_id,
        subject_user_id: journey.subject_user_id,
        source_actor_id: journey.source_actor_id,
        promise: journey.promise,
        origin: journey.origin,
        status: journey.status,
        created_at: journey.created_at,
        goal: goal_response(goal),
        objectives: objectives.into_iter().map(objective_response).collect(),
        chapters: chapter_responses,
        ungrouped_activities: ungrouped_activities
            .into_iter()
            .map(activity_response)
            .collect(),
        activities: activity_responses,
        recommendation,
    }))
}

fn goal_response(goal: crate::domain::learning::LearningGoal) -> LearningGoalResponse {
    LearningGoalResponse {
        id: goal.id,
        subject_user_id: goal.subject_user_id,
        source_actor_id: goal.source_actor_id,
        template_version_id: goal.template_version_id,
        catalog_entry_id: goal.catalog_entry_id,
        catalog_entry_version: goal.catalog_entry_version,
        raw_intent: goal.raw_intent,
        normalized_statement: goal.normalized_statement,
        status: goal.status,
        created_at: goal.created_at,
    }
}

/// POST /api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start — start or resume an activity session.
#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
    params(
        ("journey_id" = Uuid, Path, description = "Learning journey ID"),
        ("activity_id" = Uuid, Path, description = "Learning activity ID")
    ),
    responses(
        (status = 201, description = "Learning session started or resumed", body = LearningSessionResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "The authenticated learner cannot access this resource"),
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

/// GET /api/v1/learning/sessions/{id} — retrieve a learner's resumable session.
#[utoipa::path(
    get,
    path = "/api/v1/learning/sessions/{id}",
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

/// POST /api/v1/learning/sessions/{id}/finish — finish the current learning activity.
#[utoipa::path(
    post,
    path = "/api/v1/learning/sessions/{id}/finish",
    params(("id" = Uuid, Path, description = "Learning session ID")),
    request_body = FinishLearningSessionBody,
    responses(
        (status = 200, description = "Finished learning session", body = LearningSessionResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "The authenticated learner cannot access this resource"),
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
    let repository = PgLearningRepository::new(state.pool);
    let current_session = repository
        .get_learning_session(auth.owner_id(), id)
        .await
        .map_err(map_learning_error)?;
    let activity = repository
        .list_activities(auth.owner_id(), current_session.journey_id)
        .await
        .map_err(map_learning_error)?
        .into_iter()
        .find(|activity| activity.id == current_session.activity_id)
        .ok_or(ApiError::NotFound {
            resource: "activity",
        })?;
    let evidence = body
        .responses
        .iter()
        .map(|response| {
            serde_json::json!({
                "responseId": response.id,
                "value": response.value,
                "objectiveIds": activity.objective_ids.clone(),
                "activityId": activity.id,
            })
        })
        .collect::<Vec<_>>();
    let session = repository
        .finish_learning_session(FinishLearningSessionInput {
            subject_user_id: auth.owner_id(),
            session_id: id,
            completed: body.completed,
            result: serde_json::json!({
                "completed": body.completed,
                "responses": body.responses,
                "evidence": evidence,
            }),
        })
        .await
        .map_err(map_learning_error)?;
    Ok(Json(session_response(session)))
}

async fn next_recommendation(
    pool: PgPool,
    subject_user_id: Uuid,
    journey_id: Uuid,
    activities: &[LearningActivityResponse],
    latest_session: Option<&LearningSession>,
) -> Result<Option<LearningRecommendationResponse>, ApiError> {
    let objective_activities = activities
        .iter()
        .filter(|activity| {
            matches!(
                activity.status,
                ActivityStatus::Ready | ActivityStatus::InProgress
            )
        })
        .flat_map(|activity| {
            activity
                .objective_ids
                .iter()
                .map(move |objective_id| (*objective_id, activity.id))
        })
        .collect::<Vec<_>>();
    let Some(value) = (!objective_activities.is_empty())
        .then(|| crate::progress_postgres::PgProgressRepository::new(pool))
    else {
        return Ok(None);
    };
    let recommendation = ProgressRepository::recommend_weakest(
        &value,
        subject_user_id,
        journey_id,
        &objective_activities,
    )
    .await
    .map_err(crate::http::progress::map_progress_error)?;
    let activity = activities
        .iter()
        .find(|activity| activity.id == recommendation.activity_id)
        .ok_or(ApiError::NotFound {
            resource: "recommended activity",
        })?;
    Ok(Some(LearningRecommendationResponse {
        activity_id: activity.id,
        objective_id: recommendation.objective_id,
        title: activity.title.clone(),
        objective_ids: activity.objective_ids.clone(),
        evidence_ids: recommendation.evidence_ids,
        rationale: recommendation.reason,
        based_on_session_id: latest_session.map(|session| session.id),
    }))
}

fn objective_response(objective: LearningObjective) -> LearningObjectiveResponse {
    LearningObjectiveResponse {
        id: objective.id,
        journey_id: objective.journey_id,
        course_revision_id: objective.course_revision_id,
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
        course_revision_id: activity.course_revision_id,
        subject_user_id: activity.subject_user_id,
        source_actor_id: activity.source_actor_id,
        chapter_id: activity.chapter_id,
        kind: activity.kind,
        title: activity.title,
        order_index: activity.order_index,
        payload_schema_version: activity.payload_schema_version,
        content_version: activity.content_version,
        publication_status: activity.publication_status,
        payload: activity.payload,
        objective_ids: activity.objective_ids,
        status: activity.status,
        rubric: activity
            .rubric
            .and_then(|value| serde_json::from_value(value).ok()),
        created_at: activity.created_at,
        updated_at: activity.updated_at,
    }
}

fn chapter_response(
    chapter: LearningChapter,
    activities: &[LearningActivityResponse],
) -> LearningChapterResponse {
    LearningChapterResponse {
        id: chapter.id,
        journey_id: chapter.journey_id,
        course_revision_id: chapter.course_revision_id,
        subject_user_id: chapter.subject_user_id,
        title: chapter.title,
        summary: chapter.summary,
        order_index: chapter.order_index,
        activities: activities
            .iter()
            .filter(|activity| activity.chapter_id == Some(chapter.id))
            .cloned()
            .collect(),
        created_at: chapter.created_at,
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

pub(crate) async fn require_editable_activity_revision(
    pool: &PgPool,
    subject_user_id: Uuid,
    activity_id: Uuid,
    requested_revision_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let activity = sqlx::query(
        "SELECT journey_id, course_revision_id FROM tb_activities WHERE id = $1 AND subject_user_id = $2",
    )
    .bind(activity_id)
    .bind(subject_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?
    .ok_or(ApiError::NotFound {
        resource: "activity",
    })?;
    let revision_id: Option<Uuid> = activity.get("course_revision_id");
    if revision_id != requested_revision_id {
        return Err(ApiError::Validation(vec![FieldError {
            field: "revisionId".into(),
            message: "must match the activity's course revision".into(),
        }]));
    }
    crate::http::courses::require_editable_revision(
        pool,
        subject_user_id,
        activity.get("journey_id"),
        requested_revision_id,
    )
    .await?;
    Ok(())
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
        crate::domain::learning::LearningRepositoryError::ActivityNotPublished => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "activityId".to_string(),
                message: "activity must be published before a learner can start it".to_string(),
            }])
        }
        crate::domain::learning::LearningRepositoryError::InvalidActivityPublicationTransition => {
            ApiError::ActivityContentConflict
        }
        crate::domain::learning::LearningRepositoryError::EmptyField { field } => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: field.into(),
                message: "must not be empty".to_string(),
            }])
        }
        crate::domain::learning::LearningRepositoryError::InvalidActivityContent => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "content".to_string(),
                message: "does not match the activity content contract".to_string(),
            }])
        }
        crate::domain::learning::LearningRepositoryError::InvalidRubric => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "rubric".to_string(),
                message: "does not match the rubric contract".to_string(),
            }])
        }
        crate::domain::learning::LearningRepositoryError::ActivityContentCompleted => {
            ApiError::ActivityContentConflict
        }
        crate::domain::learning::LearningRepositoryError::LearningSessionFinished => {
            ApiError::SessionFinished
        }
        crate::domain::learning::LearningRepositoryError::LearningSessionResultConflict => {
            ApiError::LearningSessionResultConflict
        }
        other => ApiError::Internal(other.into()),
    }
}
