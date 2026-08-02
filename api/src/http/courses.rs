//! Course-level authoring and publication commands.
//!
//! A learning journey is the learner-owned course aggregate. This module adds
//! the author-visible revision ledger and the server-side gate that prevents a
//! set of individually published rows from masquerading as a complete course.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::{ApiError, FieldError},
        learning::{CreateGoal, CreateJourney, JourneyOrigin, LearningRepository},
    },
    http::AppState,
    learning_postgres::PgLearningRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCourseBody {
    pub raw_intent: String,
    pub idempotency_key: String,
    #[schema(value_type = Object)]
    pub brief: Value,
    pub source_references: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCourseRevisionBody {
    /// Learner-facing contract: title, audience, estimatedMinutes,
    /// prerequisites, outcomes, and modules. The validator retains it with the
    /// release decision rather than trusting an agent's free-form assertion.
    #[schema(value_type = Object)]
    pub brief: Value,
    /// Citation certificate IDs used to ground the course-level source set.
    pub source_references: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCourseRevisionBody {
    #[schema(value_type = Object)]
    pub review: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CourseValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
    pub blocking: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CourseRevisionResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub version: u32,
    pub status: String,
    #[schema(value_type = Object)]
    pub brief: Value,
    pub source_references: Vec<String>,
    pub validation: Vec<CourseValidationIssue>,
    #[schema(value_type = Option<Object>)]
    pub review: Option<Value>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub published_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCourseResponse {
    pub journey_id: Uuid,
    pub revision: CourseRevisionResponse,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/learning/courses", post(create_course))
        .route(
            "/v1/learning/journeys/{journey_id}/course-revisions",
            post(create_course_revision),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}",
            get(get_course_revision),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/validate",
            post(validate_course_revision),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/review",
            post(review_course_revision),
        )
        .route(
            "/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/publish",
            post(publish_course_revision),
        )
        .with_state(state)
}

/// Create an empty, learner-owned course aggregate. This intentionally does
/// not seed generic material: agents must add outcomes, modules, instruction,
/// checks, and assessment/task work before the revision can publish.
#[utoipa::path(
    post,
    path = "/api/v1/learning/courses",
    request_body = CreateCourseBody,
    responses((status = 201, body = CreateCourseResponse), (status = 422, description = "Brief or source set is invalid")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn create_course(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateCourseBody>,
) -> Result<(axum::http::StatusCode, Json<CreateCourseResponse>), ApiError> {
    if body.raw_intent.trim().is_empty() {
        return Err(field_error("rawIntent", "must not be empty"));
    }
    if body.idempotency_key.trim().is_empty() {
        return Err(field_error("idempotencyKey", "must not be empty"));
    }
    validate_brief_shape(&body.brief)?;
    certify_references(&state.pool, auth.owner_id(), &body.source_references).await?;

    let repository = PgLearningRepository::new(state.pool.clone());
    let goal = repository
        .create_goal(CreateGoal {
            subject_user_id: auth.owner_id(),
            source_actor_id: auth.owner_id(),
            template_version_id: None,
            catalog_entry_id: None,
            catalog_entry_version: None,
            raw_intent: body.raw_intent.clone(),
            normalized_statement: body.raw_intent,
            idempotency_key: Some(body.idempotency_key),
        })
        .await
        .map_err(map_learning_error)?;
    let journey = repository
        .ensure_journey(CreateJourney {
            goal_id: goal.id,
            subject_user_id: auth.owner_id(),
            source_actor_id: auth.owner_id(),
            promise: body
                .brief
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            catalog_entry_id: None,
            catalog_entry_version: None,
            origin: JourneyOrigin::Learner,
        })
        .await
        .map_err(map_learning_error)?;

    let row = sqlx::query(
        r#"
        INSERT INTO tb_course_revisions (journey_id, subject_user_id, version, brief, source_references)
        SELECT $1, $2, COALESCE(MAX(version), 0) + 1, $3, $4
        FROM tb_course_revisions
        WHERE journey_id = $1
        RETURNING id, journey_id, version, status, brief, source_references, validation, review,
                  published_at, created_at, updated_at
        "#,
    )
    .bind(journey.id)
    .bind(auth.owner_id())
    .bind(body.brief)
    .bind(json!(body.source_references))
    .fetch_one(&state.pool)
    .await
    .map_err(map_insert_revision_error)?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreateCourseResponse {
            journey_id: journey.id,
            revision: revision_response(row)?,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/course-revisions",
    params(("journey_id" = Uuid, Path, description = "Owned journey to author as a course")),
    request_body = CreateCourseRevisionBody,
    responses((status = 201, body = CourseRevisionResponse), (status = 409, description = "An editable course revision already exists")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn create_course_revision(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
    Json(body): Json<CreateCourseRevisionBody>,
) -> Result<(axum::http::StatusCode, Json<CourseRevisionResponse>), ApiError> {
    ensure_owned_journey(&state.pool, auth.owner_id(), journey_id).await?;
    validate_brief_shape(&body.brief)?;
    certify_references(&state.pool, auth.owner_id(), &body.source_references).await?;

    let row = sqlx::query(
        r#"
        INSERT INTO tb_course_revisions (journey_id, subject_user_id, version, brief, source_references)
        SELECT $1, $2, COALESCE(MAX(version), 0) + 1, $3, $4
        FROM tb_course_revisions
        WHERE journey_id = $1
        RETURNING id, journey_id, version, status, brief, source_references, validation, review,
                  published_at, created_at, updated_at
        "#,
    )
    .bind(journey_id)
    .bind(auth.owner_id())
    .bind(body.brief)
    .bind(json!(body.source_references))
    .fetch_one(&state.pool)
    .await
    .map_err(map_insert_revision_error)?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(revision_response(row)?),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}",
    params(("journey_id" = Uuid, Path), ("revision_id" = Uuid, Path)),
    responses((status = 200, body = CourseRevisionResponse)),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn get_course_revision(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((journey_id, revision_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CourseRevisionResponse>, ApiError> {
    Ok(Json(
        load_owned_revision(&state.pool, auth.owner_id(), journey_id, revision_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/validate",
    params(("journey_id" = Uuid, Path), ("revision_id" = Uuid, Path)),
    responses((status = 200, body = CourseRevisionResponse)),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn validate_course_revision(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((journey_id, revision_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CourseRevisionResponse>, ApiError> {
    let revision =
        load_owned_revision(&state.pool, auth.owner_id(), journey_id, revision_id).await?;
    let issues = validate_course(&state.pool, auth.owner_id(), &revision).await?;
    store_validation(&state.pool, revision_id, &issues).await?;
    Ok(Json(
        load_owned_revision(&state.pool, auth.owner_id(), journey_id, revision_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/review",
    params(("journey_id" = Uuid, Path), ("revision_id" = Uuid, Path)),
    request_body = ReviewCourseRevisionBody,
    responses((status = 200, body = CourseRevisionResponse), (status = 422, description = "Course has blocking validation issues")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn review_course_revision(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((journey_id, revision_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ReviewCourseRevisionBody>,
) -> Result<Json<CourseRevisionResponse>, ApiError> {
    let revision =
        load_owned_revision(&state.pool, auth.owner_id(), journey_id, revision_id).await?;
    require_draft(&revision)?;
    let issues = validate_course(&state.pool, auth.owner_id(), &revision).await?;
    store_validation(&state.pool, revision_id, &issues).await?;
    if issues.iter().any(|issue| issue.blocking) {
        return Err(validation_error(&issues));
    }
    sqlx::query("UPDATE tb_course_revisions SET status = 'review', review = $1, updated_at = now() WHERE id = $2")
        .bind(body.review)
        .bind(revision_id)
        .execute(&state.pool)
        .await
        .map_err(db_error)?;
    Ok(Json(
        load_owned_revision(&state.pool, auth.owner_id(), journey_id, revision_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/publish",
    params(("journey_id" = Uuid, Path), ("revision_id" = Uuid, Path)),
    responses((status = 200, body = CourseRevisionResponse), (status = 422, description = "Course has blocking validation issues")),
    security(("bearer_auth" = [])),
    tag = "learning"
)]
pub async fn publish_course_revision(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((journey_id, revision_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CourseRevisionResponse>, ApiError> {
    let revision =
        load_owned_revision(&state.pool, auth.owner_id(), journey_id, revision_id).await?;
    if revision.status != "review" {
        return Err(ApiError::GenerationStateConflict);
    }
    let issues = validate_course(&state.pool, auth.owner_id(), &revision).await?;
    store_validation(&state.pool, revision_id, &issues).await?;
    if issues.iter().any(|issue| issue.blocking) {
        return Err(validation_error(&issues));
    }

    let mut transaction = state.pool.begin().await.map_err(db_error)?;
    sqlx::query(
        "UPDATE tb_activities SET publication_status = 'published', updated_at = now() WHERE journey_id = $1 AND subject_user_id = $2 AND publication_status = 'review'",
    )
    .bind(journey_id)
    .bind(auth.owner_id())
    .execute(&mut *transaction)
    .await
    .map_err(db_error)?;
    sqlx::query(
        "UPDATE tb_course_revisions SET status = 'published', published_at = now(), updated_at = now() WHERE id = $1 AND status = 'review'",
    )
    .bind(revision_id)
    .execute(&mut *transaction)
    .await
    .map_err(db_error)?;
    transaction.commit().await.map_err(db_error)?;
    Ok(Json(
        load_owned_revision(&state.pool, auth.owner_id(), journey_id, revision_id).await?,
    ))
}

async fn ensure_owned_journey(
    pool: &sqlx::PgPool,
    subject_user_id: Uuid,
    journey_id: Uuid,
) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tb_learning_journeys WHERE id = $1 AND subject_user_id = $2)",
    )
    .bind(journey_id)
    .bind(subject_user_id)
    .fetch_one(pool)
    .await
    .map_err(db_error)?;
    if exists {
        Ok(())
    } else {
        Err(ApiError::NotFound {
            resource: "journey",
        })
    }
}

async fn load_owned_revision(
    pool: &sqlx::PgPool,
    subject_user_id: Uuid,
    journey_id: Uuid,
    revision_id: Uuid,
) -> Result<CourseRevisionResponse, ApiError> {
    let row = sqlx::query(
        "SELECT id, journey_id, version, status, brief, source_references, validation, review, published_at, created_at, updated_at FROM tb_course_revisions WHERE id = $1 AND journey_id = $2 AND subject_user_id = $3",
    )
    .bind(revision_id)
    .bind(journey_id)
    .bind(subject_user_id)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    .ok_or(ApiError::NotFound { resource: "course revision" })?;
    revision_response(row)
}

fn revision_response(row: sqlx::postgres::PgRow) -> Result<CourseRevisionResponse, ApiError> {
    let validation: Value = row.get("validation");
    let source_references: Value = row.get("source_references");
    Ok(CourseRevisionResponse {
        id: row.get("id"),
        journey_id: row.get("journey_id"),
        version: u32::try_from(row.get::<i32, _>("version"))
            .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?,
        status: row.get("status"),
        brief: row.get("brief"),
        source_references: serde_json::from_value(source_references)
            .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?,
        validation: serde_json::from_value(validation)
            .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?,
        review: row.get("review"),
        published_at: row.get("published_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn validate_brief_shape(brief: &Value) -> Result<(), ApiError> {
    let Some(object) = brief.as_object() else {
        return Err(field_error("brief", "must be an object"));
    };
    for field in [
        "title",
        "audience",
        "estimatedMinutes",
        "prerequisites",
        "outcomes",
        "modules",
    ] {
        if !object.contains_key(field) {
            return Err(field_error("brief", &format!("must include {field}")));
        }
    }
    if object
        .get("title")
        .and_then(Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
        || object
            .get("audience")
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        || object
            .get("estimatedMinutes")
            .and_then(Value::as_u64)
            .is_none_or(|value| value == 0)
        || object
            .get("outcomes")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        || object
            .get("modules")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    {
        return Err(field_error(
            "brief",
            "must contain a title, audience, positive estimatedMinutes, and non-empty outcomes and modules",
        ));
    }
    Ok(())
}

async fn certify_references(
    pool: &sqlx::PgPool,
    subject_user_id: Uuid,
    references: &[String],
) -> Result<(), ApiError> {
    if references.is_empty() {
        return Err(field_error(
            "sourceReferences",
            "must include at least one citation certificate ID",
        ));
    }
    crate::http::citations::certify_references(pool.clone(), subject_user_id, references).await
}

fn require_draft(revision: &CourseRevisionResponse) -> Result<(), ApiError> {
    if revision.status == "draft" {
        Ok(())
    } else {
        Err(ApiError::GenerationStateConflict)
    }
}

async fn store_validation(
    pool: &sqlx::PgPool,
    revision_id: Uuid,
    issues: &[CourseValidationIssue],
) -> Result<(), ApiError> {
    sqlx::query("UPDATE tb_course_revisions SET validation = $1, updated_at = now() WHERE id = $2")
        .bind(
            serde_json::to_value(issues)
                .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?,
        )
        .bind(revision_id)
        .execute(pool)
        .await
        .map_err(db_error)?;
    Ok(())
}

async fn validate_course(
    pool: &sqlx::PgPool,
    subject_user_id: Uuid,
    revision: &CourseRevisionResponse,
) -> Result<Vec<CourseValidationIssue>, ApiError> {
    let mut issues = Vec::new();
    if let Err(error) = validate_brief_shape(&revision.brief) {
        issues.push(issue("invalid_course_brief", "brief", error.to_string()));
    }
    if let Err(error) = certify_references(pool, subject_user_id, &revision.source_references).await
    {
        issues.push(issue(
            "invalid_source_set",
            "sourceReferences",
            error.to_string(),
        ));
    }

    let objectives: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_journey_objectives WHERE journey_id = $1 AND subject_user_id = $2",
    )
    .bind(revision.journey_id)
    .bind(subject_user_id)
    .fetch_one(pool)
    .await
    .map_err(db_error)?;
    if objectives == 0 {
        issues.push(issue(
            "missing_outcomes",
            "objectives",
            "a course needs at least one measurable objective",
        ));
    }

    let chapters = sqlx::query("SELECT id, title FROM tb_journey_chapters WHERE journey_id = $1 AND subject_user_id = $2 ORDER BY order_index")
        .bind(revision.journey_id).bind(subject_user_id).fetch_all(pool).await.map_err(db_error)?;
    if chapters.is_empty() {
        issues.push(issue(
            "missing_modules",
            "chapters",
            "a course needs at least one ordered module",
        ));
    }
    for chapter in &chapters {
        let chapter_id: Uuid = chapter.get("id");
        let title: String = chapter.get("title");
        let counts = sqlx::query("SELECT COUNT(*) FILTER (WHERE kind = 'explanation') AS explanations, COUNT(*) FILTER (WHERE kind = 'example') AS examples FROM tb_activities WHERE chapter_id = $1 AND subject_user_id = $2")
            .bind(chapter_id).bind(subject_user_id).fetch_one(pool).await.map_err(db_error)?;
        if counts.get::<i64, _>("explanations") == 0 {
            issues.push(issue(
                "module_missing_instruction",
                &format!("chapters.{chapter_id}"),
                format!("{title} needs a source-backed instructional activity"),
            ));
        }
        if counts.get::<i64, _>("examples") == 0 {
            issues.push(issue(
                "module_missing_worked_example",
                &format!("chapters.{chapter_id}"),
                format!("{title} needs a worked example"),
            ));
        }
    }

    let activities = sqlx::query("SELECT id, kind, title, objective_ids, payload, rubric, publication_status FROM (SELECT a.id, a.kind, a.title, a.payload, a.rubric, a.publication_status, COALESCE(array_agg(ao.objective_id) FILTER (WHERE ao.objective_id IS NOT NULL), ARRAY[]::uuid[]) AS objective_ids FROM tb_activities a LEFT JOIN tb_activity_objectives ao ON ao.activity_id = a.id WHERE a.journey_id = $1 AND a.subject_user_id = $2 GROUP BY a.id) activities")
        .bind(revision.journey_id).bind(subject_user_id).fetch_all(pool).await.map_err(db_error)?;
    for activity in &activities {
        let activity_id: Uuid = activity.get("id");
        let kind: String = activity.get("kind");
        let title: String = activity.get("title");
        let objective_ids: Vec<Uuid> = activity.get("objective_ids");
        let payload: Value = activity.get("payload");
        let publication_status: String = activity.get("publication_status");
        if objective_ids.is_empty() {
            issues.push(issue(
                "activity_missing_outcome",
                &format!("activities.{activity_id}"),
                format!("{title} must link an objective"),
            ));
        }
        if publication_status != "review" {
            issues.push(issue(
                "activity_not_reviewed",
                &format!("activities.{activity_id}"),
                "all course activities must be in review before course publication",
            ));
        }
        if matches!(kind.as_str(), "explanation" | "example")
            && !has_approved_content_provenance(&payload)
        {
            issues.push(issue(
                "instruction_missing_provenance",
                &format!("activities.{activity_id}"),
                format!("{title} needs approved source-backed content"),
            ));
        }
        if kind == "application" && activity.get::<Option<Value>, _>("rubric").is_none() {
            issues.push(issue(
                "application_missing_rubric",
                &format!("activities.{activity_id}"),
                format!("{title} needs a reviewed rubric"),
            ));
        }
        if kind == "explanation" {
            let check_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_assessments assessment INNER JOIN tb_assessment_items item ON item.assessment_id = assessment.id INNER JOIN tb_question_versions question ON question.id = item.question_version_id WHERE assessment.activity_id = $1 AND assessment.subject_user_id = $2 AND assessment.mode = 'practice' AND assessment.status = 'published' AND question.review_status = 'approved' AND NULLIF(BTRIM(question.rationale), '') IS NOT NULL AND NULLIF(BTRIM(question.explanation), '') IS NOT NULL AND jsonb_array_length(question.source_references) > 0")
                .bind(activity_id).bind(subject_user_id).fetch_one(pool).await.map_err(db_error)?;
            if check_count == 0 {
                issues.push(issue("instruction_missing_formative_check", &format!("activities.{activity_id}"), format!("{title} needs an approved formative question with answer rationale and feedback")));
            }
        }
    }
    for objective in sqlx::query(
        "SELECT id FROM tb_journey_objectives WHERE journey_id = $1 AND subject_user_id = $2",
    )
    .bind(revision.journey_id)
    .bind(subject_user_id)
    .fetch_all(pool)
    .await
    .map_err(db_error)?
    {
        let objective_id: Uuid = objective.get("id");
        let mastery_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_assessment_items item INNER JOIN tb_assessments assessment ON assessment.id = item.assessment_id WHERE item.objective_id = $1 AND assessment.subject_user_id = $2 AND assessment.mode = 'graded' AND assessment.status = 'published'")
            .bind(objective_id).bind(subject_user_id).fetch_one(pool).await.map_err(db_error)?;
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tb_activities activity INNER JOIN tb_activity_objectives link ON link.activity_id = activity.id WHERE link.objective_id = $1 AND activity.subject_user_id = $2 AND activity.kind = 'application' AND activity.rubric IS NOT NULL")
            .bind(objective_id).bind(subject_user_id).fetch_one(pool).await.map_err(db_error)?;
        if mastery_count == 0 && task_count == 0 {
            issues.push(issue(
                "outcome_missing_mastery",
                &format!("objectives.{objective_id}"),
                "each outcome needs a graded assessment or rubric-backed application task",
            ));
        }
    }
    Ok(issues)
}

fn has_approved_content_provenance(payload: &Value) -> bool {
    let Some(provenance) = payload.get("contentProvenance") else {
        return false;
    };
    provenance.get("reviewStatus").and_then(Value::as_str) == Some("approved")
        && provenance
            .get("sourceReferences")
            .and_then(Value::as_array)
            .is_some_and(|references| !references.is_empty())
}

fn issue(code: &str, path: &str, message: impl AsRef<str>) -> CourseValidationIssue {
    CourseValidationIssue {
        code: code.into(),
        path: path.into(),
        message: message.as_ref().into(),
        blocking: true,
    }
}

fn field_error(field: &str, message: &str) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}

fn validation_error(issues: &[CourseValidationIssue]) -> ApiError {
    ApiError::Validation(
        issues
            .iter()
            .map(|issue| FieldError {
                field: issue.path.clone(),
                message: format!("{}: {}", issue.code, issue.message),
            })
            .collect(),
    )
}

fn db_error(error: sqlx::Error) -> ApiError {
    ApiError::Internal(anyhow::Error::new(error))
}

fn map_insert_revision_error(error: sqlx::Error) -> ApiError {
    if matches!(&error, sqlx::Error::Database(database) if database.constraint() == Some("tb_course_revisions_one_active"))
    {
        ApiError::GenerationStateConflict
    } else {
        db_error(error)
    }
}

fn map_learning_error(error: crate::domain::learning::LearningRepositoryError) -> ApiError {
    match error {
        crate::domain::learning::LearningRepositoryError::NotFound { resource } => {
            ApiError::NotFound { resource }
        }
        crate::domain::learning::LearningRepositoryError::SubjectMismatch => ApiError::NotFound {
            resource: "learning resource",
        },
        crate::domain::learning::LearningRepositoryError::IdempotencyConflict => {
            ApiError::IdempotencyConflict
        }
        crate::domain::learning::LearningRepositoryError::EmptyField { field } => {
            field_error(field, "must not be empty")
        }
        error => ApiError::Internal(anyhow::anyhow!(error)),
    }
}
