//! Assessment routes: list, get, patch, create, delete.
//! Unified entity for assessments (mode: practice) and exams (mode: graded).

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        assessment::{
            Assessment, AssessmentMode, AssessmentStatus, AssessmentVisibility,
            CreateAssessmentRequest, UpdateAssessmentRequest,
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
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{Arguments, Row, postgres::PgArguments};
use time::{OffsetDateTime, UtcOffset};
use utoipa::ToSchema;
// End of router
// Helper to convert UTC to JST
fn to_jst(dt: OffsetDateTime) -> OffsetDateTime {
    dt.to_offset(UtcOffset::from_hms(9, 0, 0).expect("UTC+9 is a valid fixed offset"))
}
use uuid::Uuid;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/v1/assessments",
            get(list_assessments).post(create_assessment),
        )
        .route("/v1/assessments/explore", get(explore))
        .route("/v1/assessments/count", get(count_assessments))
        .route("/v1/assessments/generate", post(generate_assessment))
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
            delete(remove_assessment_question),
        )
        .route(
            "/v1/assessments/{id}/sections",
            post(create_assessment_section),
        )
        .route(
            "/v1/assessments/{id}/sections/{section_id}",
            patch(patch_assessment_section).delete(delete_assessment_section),
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
    pub visibility: String,
    pub course: Option<String>,
    pub objectives: Vec<String>,
    pub duration_min: Option<i32>,
    pub total_points: i32,
    pub question_count: i64,
    pub completed: bool,
    pub last_session_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
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
    /// Target section ID (defaults to first section if absent).
    #[serde(default)]
    pub section_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSectionBody {
    pub title: String,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub mix: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSectionBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub mix: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddAssessmentQuestionResponse {
    pub question_id: Uuid,
    pub order_index: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListParams {
    pub mode: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub tag: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAssessmentsResponse {
    pub assessments: Vec<AssessmentSummary>,
    pub total: i64,
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

fn parse_visibility(s: &str) -> AssessmentVisibility {
    match s {
        "public" => AssessmentVisibility::Public,
        "unlisted" => AssessmentVisibility::Unlisted,
        _ => AssessmentVisibility::Private,
    }
}

#[utoipa::path(
    get,
    path = "/v1/assessments",
    params(
        ("mode" = Option<String>, Query, description = "Filter by mode (practice or graded)"),
        ("status" = Option<String>, Query, description = "Filter by status (draft, active, archived). When absent, returns public-active + caller's own."),
        ("limit" = Option<i64>, Query, description = "Page size (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Page offset"),
    ),
    responses(
        (status = 200, description = "List of assessments", body = ListAssessmentsResponse),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn list_assessments(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListAssessmentsResponse>, ApiError> {
    let uid = user.owner_id;

    let mut sql = String::from(
        "SELECT a.id, a.title, a.description, a.mode, a.status, a.visibility,
                a.course, a.objectives, a.duration_min, a.total_points,
                a.created_at, a.updated_at,
                COUNT(*) OVER() AS total,
                COALESCE((SELECT SUM(items_count) FROM tb_assessment_sections
                           WHERE assessment_id = a.id), 0) AS question_count,
                EXISTS(
                    SELECT 1 FROM tb_sessions s
                    WHERE s.assessment_id = a.id AND s.user_id = $1
                      AND s.status = 'finished'
                ) AS completed,
                (
                    SELECT s.id FROM tb_sessions s
                    WHERE s.assessment_id = a.id AND s.user_id = $1
                      AND s.status = 'finished'
                    ORDER BY s.finished_at DESC LIMIT 1
                ) AS last_session_id
         FROM tb_assessments a
         WHERE (a.created_by = $1 OR EXISTS (
             SELECT 1 FROM tb_users u WHERE u.id = a.created_by AND u.owner_user_id = $1
         ))",
    );

    let mut args = PgArguments::default();
    args.add(uid)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
    let mut param_idx = 2;

    if let Some(status) = params.status {
        sql.push_str(&format!(" AND a.status = ${}", param_idx));
        args.add(status)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 1;
    }
    if let Some(mode) = params.mode {
        sql.push_str(&format!(" AND a.mode = ${}", param_idx));
        args.add(mode)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 1;
    }
    if let Some(tag) = params.tag {
        sql.push_str(&format!(" AND a.tags @> ARRAY[${}]", param_idx));
        args.add(tag)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 1;
    }
    if let Some(search) = params.search {
        sql.push_str(&format!(" AND a.title ILIKE ${}", param_idx));
        args.add(format!("%{}%", search))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 1;
    }

    sql.push_str(&format!(
        " ORDER BY a.updated_at DESC LIMIT ${} OFFSET ${}",
        param_idx,
        param_idx + 1
    ));
    args.add(params.limit)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
    args.add(params.offset)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    let rows = sqlx::query_with(&sql, args)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let total: i64 = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let assessments = rows
        .iter()
        .map(|row| AssessmentSummary {
            id: row.get("id"),
            title: row.get("title"),
            description: row.get("description"),
            mode: row.get("mode"),
            status: row.get("status"),
            visibility: row.get("visibility"),
            course: row.get("course"),
            objectives: row.get("objectives"),
            duration_min: row.get("duration_min"),
            total_points: row.get("total_points"),
            question_count: row.get("question_count"),
            completed: row.get("completed"),
            last_session_id: row.get("last_session_id"),
            created_at: to_jst(row.get("created_at")),
            updated_at: to_jst(row.get("updated_at")),
        })
        .collect();

    Ok(Json(ListAssessmentsResponse { assessments, total }))
}

#[utoipa::path(
    post,
    path = "/v1/assessments",
    request_body = CreateAssessmentRequest,
    responses(
        (status = 201, description = "Assessment created successfully", body = AssessmentSummary),
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
) -> Result<(StatusCode, Json<AssessmentSummary>), ApiError> {
    tracing::info!(
        "Creating assessment: title='{}', questions={}",
        payload.title,
        payload.questions.len()
    );
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let assessment_id = Uuid::now_v7();
    let visibility = payload.visibility.to_string();

    let question_count = payload.questions.len() as i64;
    let total_points: i32 = payload
        .questions
        .iter()
        .map(|q| q.points.unwrap_or(1))
        .sum();

    sqlx::query(
        "INSERT INTO tb_assessments \
         (id, title, description, mode, objectives, course, duration_min, \
          time_limit_seconds, passing_points, show_results_during, affects_rating, \
          method, visibility, created_by, total_points) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
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
    .bind(&payload.method)
    .bind(&visibility)
    .bind(user.user.id)
    .bind(total_points)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    // Create a default section
    let section_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_assessment_sections (id, assessment_id, title, order_index, weight, items_count) \
         VALUES ($1, $2, $3, 0, 1.0, $4)",
    )
    .bind(section_id)
    .bind(assessment_id)
    .bind("Main Section")
    .bind(question_count as i32)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    // Import questions if any
    for (i, q) in payload.questions.into_iter().enumerate() {
        let question_id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tb_questions (id, kind, prompt, payload, explanation, status, points, created_by) \
             VALUES ($1, $2, $3, $4, $5, 'live', $6, $7)"
        )
        .bind(question_id)
        .bind(q.kind.as_str())
        .bind(&q.prompt)
        .bind(&q.payload)
        .bind(&q.explanation)
        .bind(q.points.unwrap_or(1))
        .bind(user.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        // Link to assessment
        sqlx::query(
            "INSERT INTO tb_assessment_items (section_id, question_id, order_index) \
             VALUES ($1, $2, $3)",
        )
        .bind(section_id)
        .bind(question_id)
        .bind(i as i32)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        // Tags
        for tag_name in q.tags {
            let tag_id: Uuid = sqlx::query_scalar(
                "INSERT INTO tb_tags (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id"
            )
            .bind(&tag_name)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

            sqlx::query("INSERT INTO tb_question_tags (question_id, tag_id) VALUES ($1, $2)")
                .bind(question_id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(AssessmentSummary {
            id: assessment_id,
            title: payload.title,
            description: payload.description,
            mode: payload.mode.to_string(),
            status: "draft".to_string(),
            visibility,
            course: payload.course,
            objectives: payload.objectives,
            duration_min: payload.duration_min,
            total_points,
            question_count,
            completed: false,
            last_session_id: None,
            created_at: to_jst(OffsetDateTime::now_utc()),
            updated_at: to_jst(OffsetDateTime::now_utc()),
        }),
    ))
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
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AssessmentDetail>, ApiError> {
    // Visibility check: public/unlisted to anyone, otherwise scoped to the
    // owner tree (the owner and any of their agents).
    let row = sqlx::query(
        "SELECT * FROM tb_assessments WHERE id = $1 \
         AND (visibility IN ('public', 'unlisted') \
              OR created_by = $2 \
              OR EXISTS (SELECT 1 FROM tb_users u WHERE u.id = created_by AND u.owner_user_id = $2))",
    )
    .bind(id)
    .bind(user.owner_id)
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
        visibility: parse_visibility(&row.get::<String, _>("visibility")),
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
        created_at: to_jst(row.get("created_at")),
        updated_at: to_jst(row.get("updated_at")),
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

    // Flat questions list for compatible single-section view (practice assessments)
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
        (status = 403, description = "Not the owner"),
        (status = 404, description = "Assessment not found"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn patch_assessment(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAssessmentRequest>,
) -> Result<Json<Assessment>, ApiError> {
    // Only the owner can patch
    let status = payload.status.as_ref().map(|s| s.to_string());
    let row = sqlx::query(
        "UPDATE tb_assessments
           SET title       = COALESCE($1, title),
               description = COALESCE($2, description),
               status      = COALESCE($3, status),
               visibility  = COALESCE($4, visibility),
               objectives  = COALESCE($5, objectives),
               updated_at  = now()
           WHERE id = $6 AND created_by = $7
           RETURNING *",
    )
    .bind(&payload.title)
    .bind(payload.description.flatten())
    .bind(&status)
    .bind(payload.visibility.map(|v| v.to_string()))
    .bind(payload.objectives)
    .bind(id)
    .bind(user.user.id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    .ok_or(ApiError::NotFound {
        resource: "assessment",
    })?;

    if status == Some("active".to_string()) {
        // Promote all draft questions in this assessment to live
        sqlx::query(
            "UPDATE tb_questions
             SET status = 'live'
             WHERE status = 'draft'
               AND id IN (
                   SELECT question_id FROM tb_assessment_items ai
                   JOIN tb_assessment_sections s ON s.id = ai.section_id
                   WHERE s.assessment_id = $1
               )",
        )
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    Ok(Json(Assessment {
        id: row.get("id"),
        title: row.get("title"),
        description: row.get("description"),
        mode: parse_mode(&row.get::<String, _>("mode"))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        status: parse_status(&row.get::<String, _>("status"))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        visibility: parse_visibility(&row.get::<String, _>("visibility")),
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
        created_at: to_jst(row.get("created_at")),
        updated_at: to_jst(row.get("updated_at")),
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
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // Only the owner can delete
    sqlx::query("DELETE FROM tb_assessments WHERE id = $1 AND created_by = $2")
        .bind(id)
        .bind(user.user.id)
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
    // Resolve target section: use provided section_id or default to first section
    let section_id: Uuid = if let Some(sid) = body.section_id {
        // Validate that the section belongs to this assessment and is owned by the user
        sqlx::query_scalar(
            "SELECT s.id FROM tb_assessment_sections s
             JOIN tb_assessments a ON a.id = s.assessment_id
             WHERE s.id = $1 AND s.assessment_id = $2 AND a.created_by = $3",
        )
        .bind(sid)
        .bind(assessment_id)
        .bind(auth.user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
        .ok_or(ApiError::NotFound {
            resource: "assessment_section",
        })?
    } else {
        // Find the first section (owner check implicit: section exists only for owned assessments)
        sqlx::query_scalar(
            "SELECT s.id FROM tb_assessment_sections s
             JOIN tb_assessments a ON a.id = s.assessment_id
             WHERE s.assessment_id = $1 AND a.created_by = $2
             ORDER BY s.order_index ASC LIMIT 1",
        )
        .bind(assessment_id)
        .bind(auth.user.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
        .ok_or(ApiError::NotFound {
            resource: "assessment",
        })?
    };

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
        let default_payload = default_payload_for_kind(kind);

        sqlx::query_scalar(
            "INSERT INTO tb_questions (kind, prompt, payload, status, points, created_by)
             VALUES ($1, $2, $3, 'draft', 1, $4)
             RETURNING id",
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

pub fn default_payload_for_kind(kind: QuestionKind) -> serde_json::Value {
    match kind {
        QuestionKind::Mc => serde_json::json!({
            "options": ["Option A", "Option B", "Option C", "Option D"],
            "correct_index": 0
        }),
        QuestionKind::Tf => serde_json::json!({ "correct": true }),
        QuestionKind::Short => serde_json::json!({
            "accepted": [],
            "normalize": "exact",
            "judge": "exact"
        }),
        QuestionKind::Essay => serde_json::json!({
            "min_words": null,
            "rubric": null,
            "judge": "manual"
        }),
        QuestionKind::Code => serde_json::json!({
            "language": "python",
            "starter": "",
            "tests": []
        }),
    }
}

#[utoipa::path(
    post,
    path = "/v1/assessments/{id}/sections",
    params(("id" = Uuid, Path, description = "Assessment ID")),
    request_body = CreateSectionBody,
    responses(
        (status = 201, description = "Section created successfully", body = AssessmentSectionDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Assessment not found"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn create_assessment_section(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(assessment_id): Path<Uuid>,
    Json(body): Json<CreateSectionBody>,
) -> Result<(StatusCode, Json<AssessmentSectionDetail>), ApiError> {
    // Verify ownership
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tb_assessments WHERE id = $1 AND created_by = $2)",
    )
    .bind(assessment_id)
    .bind(auth.user.id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    if !exists {
        return Err(ApiError::NotFound {
            resource: "assessment",
        });
    }

    // Compute next order_index
    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(order_index) + 1, 0) FROM tb_assessment_sections WHERE assessment_id = $1",
    )
    .bind(assessment_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let section_id = Uuid::new_v4();
    let weight = body.weight.unwrap_or(1.0);

    sqlx::query(
        "INSERT INTO tb_assessment_sections (id, assessment_id, title, order_index, weight, mix, items_count) \
         VALUES ($1, $2, $3, $4, $5, $6, 0)",
    )
    .bind(section_id)
    .bind(assessment_id)
    .bind(&body.title)
    .bind(next_order)
    .bind(weight)
    .bind(&body.mix)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    Ok((
        StatusCode::CREATED,
        Json(AssessmentSectionDetail {
            id: section_id,
            title: body.title,
            order_index: next_order,
            weight,
            mix: body.mix,
            items_count: 0,
            questions: vec![],
        }),
    ))
}

#[utoipa::path(
    patch,
    path = "/v1/assessments/{id}/sections/{section_id}",
    params(
        ("id" = Uuid, Path, description = "Assessment ID"),
        ("section_id" = Uuid, Path, description = "Section ID"),
    ),
    request_body = UpdateSectionBody,
    responses(
        (status = 200, description = "Section updated successfully", body = AssessmentSectionDetail),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such assessment or section"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn patch_assessment_section(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((assessment_id, section_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateSectionBody>,
) -> Result<Json<AssessmentSectionDetail>, ApiError> {
    // Update scoped to owner via assessment join
    let sec_row = sqlx::query(
        "UPDATE tb_assessment_sections sec
            SET title  = COALESCE($1, sec.title),
                weight = COALESCE($2, sec.weight),
                mix    = COALESCE($3, sec.mix)
           FROM tb_assessments a
          WHERE sec.id = $4 AND sec.assessment_id = $5
            AND a.id = sec.assessment_id AND a.created_by = $6
          RETURNING sec.id, sec.title, sec.order_index, sec.weight, sec.mix, sec.items_count",
    )
    .bind(&body.title)
    .bind(body.weight)
    .bind(&body.mix)
    .bind(section_id)
    .bind(assessment_id)
    .bind(auth.user.id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    .ok_or(ApiError::NotFound {
        resource: "assessment_section",
    })?;

    let sec_id: Uuid = sec_row.get("id");

    // Fetch questions for this section
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

    Ok(Json(AssessmentSectionDetail {
        id: sec_id,
        title: sec_row.get("title"),
        order_index: sec_row.get("order_index"),
        weight: sec_row.get("weight"),
        mix: sec_row.get("mix"),
        items_count: sec_row.get("items_count"),
        questions,
    }))
}

#[utoipa::path(
    delete,
    path = "/v1/assessments/{id}/sections/{section_id}",
    params(
        ("id" = Uuid, Path, description = "Assessment ID"),
        ("section_id" = Uuid, Path, description = "Section ID"),
    ),
    responses(
        (status = 204, description = "Section deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such assessment or section"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn delete_assessment_section(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((assessment_id, section_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    // Delete scoped to owner via assessment join
    let deleted = sqlx::query(
        "DELETE FROM tb_assessment_sections sec
         USING tb_assessments a
         WHERE sec.id = $1 AND sec.assessment_id = $2
           AND a.id = sec.assessment_id AND a.created_by = $3",
    )
    .bind(section_id)
    .bind(assessment_id)
    .bind(auth.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "assessment_section",
        });
    }

    Ok(StatusCode::NO_CONTENT)
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
    auth: AuthenticatedUser,
    Path((assessment_id, question_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    // Find the section where this question actually lives, scoped to owner
    let section_id: Uuid = sqlx::query_scalar(
        "SELECT ai.section_id
          FROM tb_assessment_items ai
          JOIN tb_assessment_sections s ON s.id = ai.section_id
          JOIN tb_assessments a ON a.id = s.assessment_id
         WHERE s.assessment_id = $1 AND a.created_by = $2 AND ai.question_id = $3
         ORDER BY ai.order_index ASC LIMIT 1",
    )
    .bind(assessment_id)
    .bind(auth.user.id)
    .bind(question_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
    .ok_or(ApiError::NotFound {
        resource: "assessment_question",
    })?;

    let deleted =
        sqlx::query("DELETE FROM tb_assessment_items WHERE section_id = $1 AND question_id = $2")
            .bind(section_id)
            .bind(question_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    if deleted.rows_affected() > 0 {
        sqlx::query(
            "UPDATE tb_assessment_sections SET items_count = items_count - 1 WHERE id = $1",
        )
        .bind(section_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    // Delete the inline question if it's a draft not used by any other section
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

// ── count ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountAssessmentsQuery {
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountAssessmentsResponse {
    pub count: i64,
}

#[utoipa::path(
    get,
    path = "/v1/assessments/count",
    params(
        ("mode" = Option<String>, Query, description = "Filter by mode"),
    ),
    responses(
        (status = 200, description = "Assessment count", body = CountAssessmentsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn count_assessments(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(q): Query<CountAssessmentsQuery>,
) -> Result<Json<CountAssessmentsResponse>, ApiError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_assessments a
         WHERE (a.created_by = $1 OR EXISTS (
             SELECT 1 FROM tb_users u WHERE u.id = a.created_by AND u.owner_user_id = $1
         ))
           AND ($2::text IS NULL OR a.mode = $2)",
    )
    .bind(auth.owner_id)
    .bind(q.mode)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(CountAssessmentsResponse { count }))
}

// ── generate (stub) ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateAssessmentBody {
    pub source: String,
    #[serde(default = "default_generate_count")]
    pub question_count: u32,
    pub types: Option<Vec<QuestionKind>>,
    pub objectives: Option<Vec<String>>,
}

fn default_generate_count() -> u32 {
    5
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateAssessmentResponse {
    pub candidates: Vec<serde_json::Value>,
    pub objectives: Vec<String>,
    pub warnings: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/v1/assessments/generate",
    request_body = GenerateAssessmentBody,
    responses(
        (status = 200, description = "Generated assessment candidates", body = GenerateAssessmentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn generate_assessment(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<GenerateAssessmentBody>,
) -> Result<Json<GenerateAssessmentResponse>, ApiError> {
    use crate::engine::planner::{self, AssessmentPlanRequest};

    let objectives = body.objectives.clone().unwrap_or_default();

    // Select live bank questions whose tags match the objectives.
    let req = AssessmentPlanRequest {
        tags: objectives.clone(),
        tags_mode: Default::default(),
        difficulty_min: None,
        difficulty_max: None,
        count: body.question_count.max(1) as usize,
        exclude_recent_hours: 0,
    };
    let plan = planner::plan_assessment(&state.pool, auth.user.id, &req).await?;

    let mut candidates = Vec::new();
    for item in &plan.question_plan.items {
        if let Some(q) = crate::bank::questions::get_question(&state.pool, item.question_id).await?
        {
            if let Some(types) = &body.types
                && !types.is_empty()
                && !types.contains(&q.kind)
            {
                continue;
            }
            candidates.push(serde_json::to_value(q).unwrap_or(serde_json::Value::Null));
        }
    }

    let mut warnings = Vec::new();
    if let Some(w) = plan.warning {
        warnings.push(format!(
            "requested {} questions but only {} matched ({})",
            w.requested, w.planned, w.reason
        ));
    }

    Ok(Json(GenerateAssessmentResponse {
        candidates,
        objectives,
        warnings,
    }))
}

// ── explore ──────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/assessments/explore",
    params(
        ("limit" = Option<i64>, Query, description = "Page size (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Page offset"),
    ),
    responses(
        (status = 200, description = "Public assessment list", body = ListAssessmentsResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn explore(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<ListParams>,
) -> Result<Json<ListAssessmentsResponse>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT a.id, a.title, a.description, a.mode, a.status, a.visibility,
               a.course, a.objectives, a.duration_min, a.total_points,
               a.created_at, a.updated_at,
               COUNT(*) OVER() AS total,
               COALESCE((SELECT SUM(items_count) FROM tb_assessment_sections
                          WHERE assessment_id = a.id), 0) AS question_count,
               EXISTS(
                   SELECT 1 FROM tb_sessions s
                   WHERE s.assessment_id = a.id AND s.user_id = $1
                     AND s.status = 'finished'
               ) AS completed,
               (
                   SELECT s.id FROM tb_sessions s
                   WHERE s.assessment_id = a.id AND s.user_id = $1
                     AND s.status = 'finished'
                   ORDER BY s.finished_at DESC LIMIT 1
               ) AS last_session_id
        FROM tb_assessments a
        WHERE (a.created_by = $1 OR EXISTS (
            SELECT 1 FROM tb_users u WHERE u.id = a.created_by AND u.owner_user_id = $1
        ))
        ORDER BY a.updated_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.owner_id)
    .bind(params.limit)
    .bind(params.offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let total: i64 = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let assessments = rows
        .iter()
        .map(|row| AssessmentSummary {
            id: row.get("id"),
            title: row.get("title"),
            description: row.get("description"),
            mode: row.get("mode"),
            status: row.get("status"),
            visibility: row.get("visibility"),
            course: row.get("course"),
            objectives: row.get("objectives"),
            duration_min: row.get("duration_min"),
            total_points: row.get("total_points"),
            question_count: row.get("question_count"),
            completed: row.get("completed"),
            last_session_id: row.get("last_session_id"),
            created_at: to_jst(row.get("created_at")),
            updated_at: to_jst(row.get("updated_at")),
        })
        .collect();

    Ok(Json(ListAssessmentsResponse { assessments, total }))
}
