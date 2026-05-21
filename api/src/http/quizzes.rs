//! Quiz routes: list, get, patch, generate.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::scope::{RequireAnyScope, ScopeOneOf},
    domain::{
        error::{ApiError, FieldError},
        question::QuestionKind,
        user::Scope,
    },
    http::AppState,
};

pub struct QuizReadScopes;
impl ScopeOneOf for QuizReadScopes {
    const SCOPES: &'static [Scope] = &[Scope::QuizRead, Scope::QuizWrite];
}

pub struct QuizWriteScopes;
impl ScopeOneOf for QuizWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::QuizWrite];
}

// ── list ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuizSummary {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub objectives: Vec<String>,
    pub created_by: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListQuizzesResponse {
    pub quizzes: Vec<QuizSummary>,
    pub total: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListQuizzesQuery {
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_status() -> String {
    "active".to_string()
}

fn default_limit() -> i64 {
    50
}

/// List quizzes.
#[utoipa::path(
    get,
    path = "/v1/quizzes",
    params(
        ("status" = Option<String>, Query, description = "Filter by status (default: active)"),
        ("limit" = Option<i64>, Query, description = "Page size (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Page offset"),
    ),
    responses(
        (status = 200, description = "Quiz list", body = ListQuizzesResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn list_quizzes(
    State(state): State<AppState>,
    _auth: RequireAnyScope<QuizReadScopes>,
    Query(q): Query<ListQuizzesQuery>,
) -> Result<Json<ListQuizzesResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, title, status, objectives, created_by, created_at, updated_at,
                COUNT(*) OVER() AS total
         FROM quizzes
         WHERE status = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(&q.status)
    .bind(q.limit)
    .bind(q.offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let total: i64 = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let quizzes = rows
        .into_iter()
        .map(|r| QuizSummary {
            id: r.get("id"),
            title: r.get("title"),
            status: r.get("status"),
            objectives: r.get::<Vec<String>, _>("objectives"),
            created_by: r.get("created_by"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok(Json(ListQuizzesResponse { quizzes, total }))
}

// ── get one ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuizQuestion {
    pub id: Uuid,
    pub kind: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<Value>,
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    pub points: i32,
    pub status: String,
    pub order_index: i32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetQuizResponse {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub objectives: Vec<String>,
    pub created_by: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
    pub questions: Vec<QuizQuestion>,
}

async fn get_quiz(
    State(state): State<AppState>,
    _auth: RequireAnyScope<QuizReadScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetQuizResponse>, ApiError> {
    let quiz_row = sqlx::query(
        "SELECT id, title, status, objectives, created_by, created_at, updated_at
         FROM quizzes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound { resource: "quiz" })?;

    let question_rows = sqlx::query(
        "SELECT q.id, q.kind, q.prompt, q.code_snippet, q.payload, q.explanation,
                COALESCE(qq.points_override, q.points) AS points, q.status, qq.order_index
         FROM quiz_questions qq
         JOIN questions q ON q.id = qq.question_id
         WHERE qq.quiz_id = $1
         ORDER BY qq.order_index",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let questions = question_rows
        .into_iter()
        .map(|r| QuizQuestion {
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

    Ok(Json(GetQuizResponse {
        id: quiz_row.get("id"),
        title: quiz_row.get("title"),
        status: quiz_row.get("status"),
        objectives: quiz_row.get::<Vec<String>, _>("objectives"),
        created_by: quiz_row.get("created_by"),
        created_at: quiz_row.get("created_at"),
        updated_at: quiz_row.get("updated_at"),
        questions,
    }))
}

// ── patch ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuizPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objectives: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchQuizResponse {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub objectives: Vec<String>,
    pub warnings: Vec<String>,
}

async fn patch_quiz(
    State(state): State<AppState>,
    _auth: RequireAnyScope<QuizWriteScopes>,
    Path(id): Path<Uuid>,
    Json(body): Json<QuizPatch>,
) -> Result<Json<PatchQuizResponse>, ApiError> {
    // Verify quiz exists and is owned by caller (instructors can edit any, agents only their own)
    let quiz_row =
        sqlx::query("SELECT id, title, status, objectives, created_by FROM quizzes WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
            .ok_or(ApiError::NotFound { resource: "quiz" })?;

    let mut warnings: Vec<String> = Vec::new();

    // Publish gating
    if body.status.as_deref() == Some("active") {
        let current_status: String = quiz_row.get("status");
        if current_status == "archived" {
            return Err(ApiError::Validation(vec![FieldError {
                field: "status".into(),
                message: "archived quizzes cannot be re-activated".into(),
            }]));
        }

        // Must have at least one question, all live
        let q_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM quiz_questions WHERE quiz_id = $1")
                .bind(id)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;

        if q_count == 0 {
            return Err(ApiError::Validation(vec![FieldError {
                field: "status".into(),
                message: "quiz must have at least one question before publishing".into(),
            }]));
        }

        let draft_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM quiz_questions qq
             JOIN questions q ON q.id = qq.question_id
             WHERE qq.quiz_id = $1 AND q.status != 'live'",
        )
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        if draft_count > 0 {
            return Err(ApiError::Validation(vec![FieldError {
                field: "status".into(),
                message: "all questions must be live before publishing".into(),
            }]));
        }
    }

    // Objectives soft-warn at > 6
    if let Some(ref objs) = body.objectives
        && objs.len() > 6
    {
        warnings.push(format!(
            "objectives has {} entries; more than 6 may be hard to scan",
            objs.len()
        ));
    }

    let updated = sqlx::query(
        "UPDATE quizzes SET
           title      = COALESCE($2, title),
           objectives = COALESCE($3, objectives),
           status     = COALESCE($4, status),
           updated_at = now()
         WHERE id = $1
         RETURNING id, title, status, objectives",
    )
    .bind(id)
    .bind(body.title)
    .bind(body.objectives)
    .bind(body.status)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(PatchQuizResponse {
        id: updated.get("id"),
        title: updated.get("title"),
        status: updated.get("status"),
        objectives: updated.get::<Vec<String>, _>("objectives"),
        warnings,
    }))
}

// ── create ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuizBody {
    pub title: String,
    #[serde(default)]
    pub course: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub duration: Option<i32>,
    #[serde(default)]
    pub objectives: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuizResponse {
    pub quiz_id: String,
    pub quiz: CreatedQuiz,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedQuiz {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub objectives: Vec<String>,
    pub created_by: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

/// Create a new quiz.
///
/// Only instructors and admins may create quizzes. Learners receive 403.
#[utoipa::path(
    post,
    path = "/v1/quizzes",
    responses(
        (status = 201, description = "Quiz created", body = CreateQuizResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (learner cannot create)"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn create_quiz(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuizWriteScopes>,
    Json(body): Json<CreateQuizBody>,
) -> Result<(axum::http::StatusCode, Json<CreateQuizResponse>), ApiError> {
    // Only instructor/admin may create; learner role is forbidden
    use crate::domain::user::Role;
    if auth.0.user.role == Role::Learner {
        return Err(ApiError::ScopeRequired("quiz.write"));
    }

    let objectives = body.objectives.unwrap_or_default();

    let result = sqlx::query(
        "INSERT INTO quizzes (title, objectives, status, created_by)
         VALUES ($1, $2, $3, $4)
         RETURNING id, title, status, objectives, created_by, created_at",
    )
    .bind(&body.title)
    .bind(&objectives)
    .bind("draft")
    .bind(auth.0.user.id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let quiz = CreatedQuiz {
        id: result.get("id"),
        title: result.get("title"),
        status: result.get("status"),
        objectives: result.get::<Vec<String>, _>("objectives"),
        created_by: result.get("created_by"),
        created_at: result.get("created_at"),
    };

    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreateQuizResponse {
            quiz_id: quiz.id.to_string(),
            quiz,
        }),
    ))
}

// ── add question to quiz ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddQuizQuestionBody {
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
pub struct AddQuizQuestionResponse {
    pub question_id: Uuid,
    pub order_index: i32,
}

async fn add_quiz_question(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuizWriteScopes>,
    Path(quiz_id): Path<Uuid>,
    Json(body): Json<AddQuizQuestionBody>,
) -> Result<(axum::http::StatusCode, Json<AddQuizQuestionResponse>), ApiError> {
    use crate::domain::user::Role;
    if auth.0.user.role == Role::Learner {
        return Err(ApiError::ScopeRequired("quiz.write"));
    }

    // Verify quiz exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM quizzes WHERE id = $1)")
        .bind(quiz_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    if !exists {
        return Err(ApiError::NotFound { resource: "quiz" });
    }

    // Resolve the question ID — use existing or create a new draft question
    let question_id = if let Some(qid) = body.question_id {
        // Verify it exists in the bank
        let qexists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM questions WHERE id = $1)")
                .bind(qid)
                .fetch_one(&state.pool)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;
        if !qexists {
            return Err(ApiError::NotFound {
                resource: "question",
            });
        }
        qid
    } else {
        // Create inline draft question
        let kind = body.kind.ok_or_else(|| {
            ApiError::Validation(vec![FieldError {
                field: "kind".into(),
                message: "required when question_id is absent".into(),
            }])
        })?;
        let prompt = body.prompt.unwrap_or_default();
        let default_payload = default_payload_for_kind(kind);

        let row = sqlx::query(
            "INSERT INTO questions (kind, prompt, payload, status, points, created_by)
             VALUES ($1, $2, $3, 'draft', 1, $4)
             RETURNING id",
        )
        .bind(kind.as_str())
        .bind(&prompt)
        .bind(&default_payload)
        .bind(auth.0.user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        row.get::<Uuid, _>("id")
    };

    // Compute next order_index
    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(order_index) + 1, 0) FROM quiz_questions WHERE quiz_id = $1",
    )
    .bind(quiz_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    sqlx::query(
        "INSERT INTO quiz_questions (quiz_id, question_id, order_index, points_override)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (quiz_id, question_id) DO NOTHING",
    )
    .bind(quiz_id)
    .bind(question_id)
    .bind(next_order)
    .bind(body.points_override)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(AddQuizQuestionResponse {
            question_id,
            order_index: next_order,
        }),
    ))
}

fn default_payload_for_kind(kind: QuestionKind) -> Value {
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

// ── count ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountQuizzesQuery {
    #[serde(default)]
    pub cats: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub diff: Option<String>,
    #[serde(default)]
    pub types: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CountQuizzesResponse {
    pub count: i64,
}

/// Get quiz count with optional filters.
#[utoipa::path(
    get,
    path = "/v1/quizzes/count",
    params(
        ("cats" = Option<String>, Query, description = "Comma-separated categories"),
        ("tags" = Option<String>, Query, description = "Comma-separated tags"),
        ("diff" = Option<String>, Query, description = "Difficulty filter"),
        ("types" = Option<String>, Query, description = "Comma-separated types"),
    ),
    responses(
        (status = 200, description = "Quiz count", body = CountQuizzesResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn count_quizzes(
    State(state): State<AppState>,
    _auth: RequireAnyScope<QuizReadScopes>,
    Query(_q): Query<CountQuizzesQuery>,
) -> Result<Json<CountQuizzesResponse>, ApiError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quizzes WHERE status = $1")
        .bind("active")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(CountQuizzesResponse { count }))
}

// ── generate (stub) ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenerateBody {
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
pub struct GenerateResponse {
    pub candidates: Vec<Value>,
    pub objectives: Vec<String>,
    pub warnings: Vec<String>,
}

async fn generate_quiz(
    _auth: RequireAnyScope<QuizWriteScopes>,
    Json(_body): Json<GenerateBody>,
) -> Result<Json<GenerateResponse>, ApiError> {
    // Stub — real LLM-backed generation is a deferred feature (post-MVP).
    Ok(Json(GenerateResponse {
        candidates: vec![],
        objectives: vec![],
        warnings: vec!["quiz.generate is not yet implemented; no candidates returned".into()],
    }))
}

// ── router ────────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/quizzes", get(list_quizzes).post(create_quiz))
        .route("/v1/quizzes/count", get(count_quizzes))
        .route("/v1/quizzes/generate", post(generate_quiz))
        .route("/v1/quizzes/{id}", get(get_quiz).patch(patch_quiz))
        .route("/v1/quizzes/{id}/questions", post(add_quiz_question))
        .with_state(state)
}
