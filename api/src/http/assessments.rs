//! Assessment routes: list, get, patch, create, delete.
//! Unified entity for quizzes (mode: practice) and exams (mode: graded).

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        assessment::{
            Assessment, AssessmentMode, AssessmentStatus, CreateAssessmentRequest,
            UpdateAssessmentRequest,
        },
        error::{ApiError, FieldError},
        question::QuestionKind,
    },
    http::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{Arguments, Row, postgres::PgArguments};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/v1/assessments",
            get(list_assessments).post(create_assessment),
        )
        .route(
            "/v1/assessments/{id}",
            get(get_assessment)
                .patch(patch_assessment)
                .delete(delete_assessment),
        )
        .route(
            "/v1/assessments/{id}/questions",
            post(add_assessment_question),
        )
        .route(
            "/v1/assessments/{id}/questions/{question_id}",
            axum::routing::delete(remove_assessment_question),
        )
        .with_state(state)
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentSummary {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub mode: String,
    pub status: String,
    pub course: Option<String>,
    pub objectives: Vec<String>,
    pub duration_min: Option<i32>,
    pub total_points: i32,
    pub question_count: i64,
    pub completed: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentQuestion {
    pub id: Uuid,
    pub kind: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<serde_json::Value>,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    pub points: i32,
    pub status: String,
    pub order_index: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentSectionDetail {
    pub id: Uuid,
    pub title: String,
    pub order_index: i32,
    pub weight: f64,
    pub mix: Option<serde_json::Value>,
    pub items_count: i32,
    pub questions: Vec<AssessmentQuestion>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentDetail {
    #[serde(flatten)]
    pub assessment: Assessment,
    pub sections: Vec<AssessmentSectionDetail>,
    pub questions: Vec<AssessmentQuestion>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddAssessmentQuestionBody {
    /// Link an existing bank question by ID.
    #[serde(default)]
    pub question_id: Option<Uuid>,
    /// Create a new question inline (kind + prompt required if question_id absent).
    #[serde(default)]
    pub kind: Option<QuestionKind>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub points_override: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddAssessmentQuestionResponse {
    pub question_id: Uuid,
    pub order_index: i32,
}

#[derive(Deserialize)]
pub struct ListParams {
    pub mode: Option<String>,
    pub status: Option<String>,
}

// Helper to parse strings to Enums for manual mapping
fn parse_mode(s: &str) -> Result<AssessmentMode, String> {
    match s {
        "practice" => Ok(AssessmentMode::Practice),
        "graded" => Ok(AssessmentMode::Graded),
        _ => Err(format!("unknown mode: {}", s)),
    }
}

fn parse_status(s: &str) -> Result<AssessmentStatus, String> {
    match s {
        "draft" => Ok(AssessmentStatus::Draft),
        "active" => Ok(AssessmentStatus::Active),
        "archived" => Ok(AssessmentStatus::Archived),
        _ => Err(format!("unknown status: {}", s)),
    }
}

#[utoipa::path(
    get,
    path = "/v1/assessments",
    params(
        ("mode" = Option<String>, Query, description = "Filter by mode (practice or graded)"),
        ("status" = Option<String>, Query, description = "Filter by status (draft, active, archived)"),
    ),
    responses(
        (status = 200, description = "List of assessments", body = Vec<AssessmentSummary>),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn list_assessments(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<AssessmentSummary>>, ApiError> {
    let mut query = r#"
        SELECT a.id, a.title, a.description, a.mode, a.status, a.course, a.objectives, a.duration_min, a.total_points, a.created_at, a.updated_at,
               COALESCE((SELECT SUM(items_count) FROM tb_assessment_sections WHERE assessment_id = a.id), 0) AS question_count,
               EXISTS(
                   SELECT 1 FROM tb_sessions s
                   WHERE s.assessment_id = a.id AND s.user_id = $1 AND s.status = 'finished'
               ) AS completed
        FROM tb_assessments a
        WHERE 1=1
    "#.to_string();
    let mut args = PgArguments::default();
    args.add(user.owner_id)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    if let Some(ref mode) = params.mode {
        query.push_str(" AND a.mode = $2");
        args.add(mode)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    if let Some(status) = params.status {
        query.push_str(if params.mode.is_some() {
            " AND a.status = $3"
        } else {
            " AND a.status = $2"
        });
        args.add(status)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    let rows = sqlx::query_with(&query, args)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    Ok(Json(
        rows.iter()
            .map(|row| AssessmentSummary {
                id: row.get("id"),
                title: row.get("title"),
                description: row.get("description"),
                mode: row.get("mode"),
                status: row.get("status"),
                course: row.get("course"),
                objectives: row.get("objectives"),
                duration_min: row.get("duration_min"),
                total_points: row.get("total_points"),
                question_count: row.get("question_count"),
                completed: row.get("completed"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/assessments",
    request_body = CreateAssessmentRequest,
    responses(
        (status = 200, description = "Assessment created successfully", body = AssessmentSummary),
        (status = 401, description = "Missing or invalid token"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn create_assessment(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateAssessmentRequest>,
) -> Result<Json<AssessmentSummary>, ApiError> {
    let assessment_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO tb_assessments (id, title, description, mode, objectives, course, duration_min, time_limit_seconds, passing_points, show_results_during, affects_rating, method, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
    )
    .bind(assessment_id)
    .bind(payload.title.clone())
    .bind(payload.description.clone())
    .bind(payload.mode.to_string())
    .bind(payload.objectives.clone())
    .bind(payload.course.clone())
    .bind(payload.duration_min)
    .bind(payload.time_limit_seconds)
    .bind(payload.passing_points)
    .bind(payload.show_results_during)
    .bind(payload.affects_rating)
    .bind(payload.method)
    .bind(user.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    // Create a default section
    sqlx::query(
        "INSERT INTO tb_assessment_sections (id, assessment_id, title, order_index, weight, items_count)
         VALUES ($1, $2, $3, 0, 1.0, 0)"
    )
    .bind(Uuid::new_v4())
    .bind(assessment_id)
    .bind("Main Section")
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    Ok(Json(AssessmentSummary {
        id: assessment_id,
        title: payload.title,
        description: payload.description,
        mode: payload.mode.to_string(),
        status: "draft".to_string(),
        course: payload.course,
        objectives: payload.objectives,
        duration_min: payload.duration_min,
        total_points: 0,
        question_count: 0,
        completed: false,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }))
}

#[utoipa::path(
    get,
    path = "/v1/assessments/{id}",
    params(
        ("id" = Uuid, Path, description = "Assessment ID"),
    ),
    responses(
        (status = 200, description = "Assessment details", body = AssessmentDetail),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Assessment not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn get_assessment(
    _user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AssessmentDetail>, ApiError> {
    let row = sqlx::query("SELECT * FROM tb_assessments WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
        .ok_or(ApiError::NotFound {
            resource: "assessment",
        })?;

    let assessment = Assessment {
        id: row.get("id"),
        title: row.get("title"),
        description: row.get("description"),
        mode: parse_mode(&row.get::<String, _>("mode"))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        status: parse_status(&row.get::<String, _>("status"))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        objectives: row.get("objectives"),
        course: row.get("course"),
        duration_min: row.get("duration_min"),
        time_limit_seconds: row.get("time_limit_seconds"),
        total_points: row.get("total_points"),
        passing_points: row.get("passing_points"),
        show_results_during: row.get("show_results_during"),
        affects_rating: row.get("affects_rating"),
        method: row.get("method"),
        composition_trace: row.get("composition_trace"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    };

    // Fetch sections
    let section_rows = sqlx::query(
        "SELECT * FROM tb_assessment_sections WHERE assessment_id = $1 ORDER BY order_index ASC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let mut sections = Vec::new();
    for sec_row in section_rows {
        let sec_id: Uuid = sec_row.get("id");

        // Fetch items for this section
        let item_rows = sqlx::query(
            "SELECT q.id, q.kind, q.prompt, q.code_snippet, q.payload, q.explanation, 
                    COALESCE(ai.points_override, q.points) AS points, q.status, ai.order_index
             FROM tb_assessment_items ai
             JOIN tb_questions q ON q.id = ai.question_id
             WHERE ai.section_id = $1
             ORDER BY ai.order_index ASC",
        )
        .bind(sec_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

        let questions = item_rows
            .into_iter()
            .map(|r| AssessmentQuestion {
                id: r.get("id"),
                kind: r.get("kind"),
                prompt: r.get("prompt"),
                code_snippet: r.get("code_snippet"),
                payload: r.get("payload"),
                explanation: r.get("explanation"),
                points: r.get("points"),
                status: r.get("status"),
                order_index: r.get("order_index"),
            })
            .collect();

        sections.push(AssessmentSectionDetail {
            id: sec_id,
            title: sec_row.get("title"),
            order_index: sec_row.get("order_index"),
            weight: sec_row.get("weight"),
            mix: sec_row.get("mix"),
            items_count: sec_row.get("items_count"),
            questions,
        });
    }

    // Flat questions list for compatible single-section view (practice quizzes)
    let questions = sections
        .first()
        .map(|s| s.questions.clone())
        .unwrap_or_default();

    Ok(Json(AssessmentDetail {
        assessment,
        sections,
        questions,
    }))
}

#[utoipa::path(
    patch,
    path = "/v1/assessments/{id}",
    params(
        ("id" = Uuid, Path, description = "Assessment ID"),
    ),
    request_body = UpdateAssessmentRequest,
    responses(
        (status = 200, description = "Assessment updated successfully", body = Assessment),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Assessment not found"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn patch_assessment(
    _user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAssessmentRequest>,
) -> Result<Json<Assessment>, ApiError> {
    let row = sqlx::query(
        "UPDATE tb_assessments 
           SET title = COALESCE($1, title), 
               description = COALESCE($2, description), 
               status = COALESCE($3, status),
               objectives = COALESCE($4, objectives),
               updated_at = now()
           WHERE id = $5
           RETURNING *",
    )
    .bind(payload.title)
    .bind(payload.description.flatten())
    .bind(payload.status.map(|s| s.to_string()))
    .bind(payload.objectives)
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    .ok_or(ApiError::NotFound {
        resource: "assessment",
    })?;

    Ok(Json(Assessment {
        id: row.get("id"),
        title: row.get("title"),
        description: row.get("description"),
        mode: parse_mode(&row.get::<String, _>("mode"))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        status: parse_status(&row.get::<String, _>("status"))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        objectives: row.get("objectives"),
        course: row.get("course"),
        duration_min: row.get("duration_min"),
        time_limit_seconds: row.get("time_limit_seconds"),
        total_points: row.get("total_points"),
        passing_points: row.get("passing_points"),
        show_results_during: row.get("show_results_during"),
        affects_rating: row.get("affects_rating"),
        method: row.get("method"),
        composition_trace: row.get("composition_trace"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

#[utoipa::path(
    delete,
    path = "/v1/assessments/{id}",
    params(
        ("id" = Uuid, Path, description = "Assessment ID"),
    ),
    responses(
        (status = 204, description = "Assessment deleted successfully"),
        (status = 401, description = "Missing or invalid token"),
        (status = 404, description = "Assessment not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn delete_assessment(
    _user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM tb_assessments WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/v1/assessments/{id}/questions",
    params(("id" = Uuid, Path, description = "Assessment ID")),
    request_body = AddAssessmentQuestionBody,
    responses(
        (status = 201, description = "Question linked", body = AddAssessmentQuestionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such assessment or question"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn add_assessment_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(assessment_id): Path<Uuid>,
    Json(body): Json<AddAssessmentQuestionBody>,
) -> Result<(StatusCode, Json<AddAssessmentQuestionResponse>), ApiError> {
    // Find the first section
    let section_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM tb_assessment_sections WHERE assessment_id = $1 ORDER BY order_index ASC LIMIT 1"
    )
    .bind(assessment_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    .ok_or(ApiError::NotFound { resource: "assessment" })?;

    // Resolve the question ID
    let question_id = if let Some(qid) = body.question_id {
        let qexists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tb_questions WHERE id = $1)")
                .bind(qid)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
        if !qexists {
            return Err(ApiError::NotFound {
                resource: "question",
            });
        }
        qid
    } else {
        let kind = body.kind.ok_or_else(|| {
            ApiError::Validation(vec![FieldError {
                field: "kind".into(),
                message: "required when question_id is absent".into(),
            }])
        })?;
        let prompt = body.prompt.unwrap_or_default();
        let default_payload = crate::http::quizzes::default_payload_for_kind(kind);

        sqlx::query_scalar(
            "INSERT INTO tb_questions (kind, prompt, payload, status, points, created_by)
             VALUES ($1, $2, $3, 'draft', 1, $4)
             RETURNING id"
        )
        .bind(kind.as_str())
        .bind(&prompt)
        .bind(&default_payload)
        .bind(auth.user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    };

    // Compute next order_index
    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(order_index) + 1, 0) FROM tb_assessment_items WHERE section_id = $1",
    )
    .bind(section_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let inserted = sqlx::query(
        "INSERT INTO tb_assessment_items (section_id, question_id, order_index, points_override)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (section_id, question_id) DO NOTHING",
    )
    .bind(section_id)
    .bind(question_id)
    .bind(next_order)
    .bind(body.points_override)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    if inserted.rows_affected() > 0 {
        sqlx::query(
            "UPDATE tb_assessment_sections SET items_count = items_count + 1 WHERE id = $1",
        )
        .bind(section_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    Ok((
        StatusCode::CREATED,
        Json(AddAssessmentQuestionResponse {
            question_id,
            order_index: next_order,
        }),
    ))
}

#[utoipa::path(
    delete,
    path = "/v1/assessments/{id}/questions/{question_id}",
    params(
        ("id" = Uuid, Path, description = "Assessment ID"),
        ("question_id" = Uuid, Path, description = "Question ID"),
    ),
    responses(
        (status = 204, description = "Question removed from assessment"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such assessment or question"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn remove_assessment_question(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path((assessment_id, question_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    // Find the first section
    let section_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM tb_assessment_sections WHERE assessment_id = $1 ORDER BY order_index ASC LIMIT 1"
    )
    .bind(assessment_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    .ok_or(ApiError::NotFound { resource: "assessment" })?;

    let deleted =
        sqlx::query("DELETE FROM tb_assessment_items WHERE section_id = $1 AND question_id = $2")
            .bind(section_id)
            .bind(question_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "quiz_question",
        });
    }

    sqlx::query("UPDATE tb_assessment_sections SET items_count = items_count - 1 WHERE id = $1")
        .bind(section_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    // Delete the question itself if it's a draft not used by any other section
    sqlx::query(
        "DELETE FROM tb_questions
         WHERE id = $1
           AND status = 'draft'
           AND NOT EXISTS (SELECT 1 FROM tb_assessment_items WHERE question_id = $1)",
    )
    .bind(question_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    Ok(StatusCode::NO_CONTENT)
}
