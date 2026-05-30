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
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireAnyScope, ScopeOneOf},
    },
    domain::{
        error::{ApiError, FieldError},
        question::QuestionKind,
        quiz::Visibility,
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
    pub visibility: Visibility,
    pub objectives: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub course: Option<String>,
    pub question_count: i64,
    pub created_by: Uuid,
    /// True when the requesting user has at least one finished session for this quiz.
    pub completed: bool,
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
    auth: AuthenticatedUser,
    Query(q): Query<ListQuizzesQuery>,
) -> Result<Json<ListQuizzesResponse>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT q.id, q.title, q.status, q.visibility, q.objectives, q.course, q.created_by,
               q.created_at, q.updated_at,
               COUNT(*) OVER() AS total,
               COUNT(qq.question_id) AS question_count,
               EXISTS(
                   SELECT 1 FROM tb_sessions s
                   WHERE s.quiz_id = q.id AND s.user_id = $4 AND s.status = 'finished'
               ) AS completed
        FROM tb_quizzes q
        LEFT JOIN tb_quiz_questions qq ON qq.quiz_id = q.id
        WHERE q.status = $1 AND (q.visibility = 'public' OR q.created_by = $4 OR EXISTS(SELECT 1 FROM tb_users u WHERE u.id = q.created_by AND (u.id = $4 OR u.owner_user_id = $4)))
        GROUP BY q.id
        ORDER BY q.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&q.status)
    .bind(q.limit)
    .bind(q.offset)
    .bind(auth.owner_id)

    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let total: i64 = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let quizzes: Vec<QuizSummary> = rows
        .into_iter()
        .map(|r| {
            Ok(QuizSummary {
                id: r.get("id"),
                title: r.get("title"),
                status: r.get("status"),
                visibility: r
                    .get::<String, _>("visibility")
                    .parse()
                    .map_err(|e: String| ApiError::Internal(anyhow::anyhow!(e)))?,
                objectives: r.get::<Vec<String>, _>("objectives"),
                course: r.get("course"),
                question_count: r.get("question_count"),
                created_by: r.get("created_by"),
                completed: r.get("completed"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

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
    pub visibility: Visibility,
    pub objectives: Vec<String>,
    pub created_by: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
    pub questions: Vec<QuizQuestion>,
}

#[utoipa::path(
    get,
    path = "/v1/quizzes/{id}",
    params(("id" = Uuid, Path, description = "Quiz id")),
    responses(
        (status = 200, description = "Quiz detail", body = GetQuizResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such quiz"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn get_quiz(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<GetQuizResponse>, ApiError> {
    let quiz_row = sqlx::query(
        "SELECT id, title, status, visibility, objectives, created_by, created_at, updated_at
         FROM tb_quizzes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound { resource: "quiz" })?;

    let visibility: Visibility = quiz_row
        .get::<String, _>("visibility")
        .parse()
        .map_err(|e: String| ApiError::Internal(anyhow::anyhow!(e)))?;
    let created_by: Uuid = quiz_row.get("created_by");

    if visibility == Visibility::Private {
        let is_owned = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tb_users WHERE id = $1 AND (id = $2 OR owner_user_id = $2))"
        )
        .bind(created_by)
        .bind(auth.owner_id)

        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        if !is_owned {
            return Err(ApiError::NotFound { resource: "quiz" });
        }
    }

    let question_rows = sqlx::query(
        "SELECT q.id, q.kind, q.prompt, q.code_snippet, q.payload, q.explanation,
                COALESCE(qq.points_override, q.points) AS points, q.status, qq.order_index
         FROM tb_quiz_questions qq
         JOIN tb_questions q ON q.id = qq.question_id
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
        visibility,
        objectives: quiz_row.get::<Vec<String>, _>("objectives"),
        created_by,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchQuizResponse {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub visibility: Visibility,
    pub objectives: Vec<String>,
    pub warnings: Vec<String>,
}

#[utoipa::path(
    patch,
    path = "/v1/quizzes/{id}",
    params(("id" = Uuid, Path, description = "Quiz id")),
    request_body = QuizPatch,
    responses(
        (status = 200, description = "Quiz updated", body = PatchQuizResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No such quiz"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn patch_quiz(
    State(state): State<AppState>,
    _auth: RequireAnyScope<QuizWriteScopes>,
    Path(id): Path<Uuid>,
    Json(body): Json<QuizPatch>,
) -> Result<Json<PatchQuizResponse>, ApiError> {
    Ok(Json(apply_quiz_patch(&state.pool, id, body).await?))
}

/// Apply a quiz patch: existence check, publish gating + draft-question
/// auto-promotion, then the metadata update. Shared by `PATCH /v1/quizzes/{id}`
/// and the agent `quiz.update` run-tool so publish semantics never drift.
pub(crate) async fn apply_quiz_patch(
    pool: &sqlx::PgPool,
    id: Uuid,
    body: QuizPatch,
) -> Result<PatchQuizResponse, ApiError> {
    // Verify quiz exists and is owned by caller (instructors can edit any, agents only their own)
    let quiz_row = sqlx::query(
        "SELECT id, title, status, objectives, created_by FROM tb_quizzes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound { resource: "quiz" })?;

    let mut warnings: Vec<String> = Vec::new();

    // Publish gating
    if body.status.as_deref() == Some("active") {
        // TODO: P4 — plan-gate public publishing
        // TODO: P6 — audit log entry for public publishing

        let current_status: String = quiz_row.get("status");
        if current_status == "archived" {
            return Err(ApiError::Validation(vec![FieldError {
                field: "status".into(),
                message: "archived quizzes cannot be re-activated".into(),
            }]));
        }

        // Must have at least one question, all live
        let q_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tb_quiz_questions WHERE quiz_id = $1")
                .bind(id)
                .fetch_one(pool)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;

        if q_count == 0 {
            return Err(ApiError::Validation(vec![FieldError {
                field: "status".into(),
                message: "quiz must have at least one question before publishing".into(),
            }]));
        }

        // Auto-promote any draft questions attached to this quiz
        sqlx::query(
            "UPDATE tb_questions SET status = 'live'
             WHERE id IN (
                 SELECT question_id FROM tb_quiz_questions WHERE quiz_id = $1
             ) AND status = 'draft'",
        )
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
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
        "UPDATE tb_quizzes SET
           title      = COALESCE($2, title),
           objectives = COALESCE($3, objectives),
           status     = COALESCE($4, status),
           visibility = COALESCE($5, visibility),
           updated_at = now()
         WHERE id = $1
         RETURNING id, title, status, visibility, objectives",
    )
    .bind(id)
    .bind(body.title)
    .bind(body.objectives)
    .bind(body.status)
    .bind(body.visibility.map(|v| v.as_str()))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(PatchQuizResponse {
        id: updated.get("id"),
        title: updated.get("title"),
        status: updated.get("status"),
        visibility: updated
            .get::<String, _>("visibility")
            .parse()
            .map_err(|e: String| ApiError::Internal(anyhow::anyhow!(e)))?,
        objectives: updated.get::<Vec<String>, _>("objectives"),
        warnings,
    })
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
    #[serde(default)]
    pub visibility: Option<Visibility>,
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
    pub visibility: Visibility,
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
    request_body = CreateQuizBody,
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
    auth: AuthenticatedUser,
    Json(body): Json<CreateQuizBody>,
) -> Result<(axum::http::StatusCode, Json<CreateQuizResponse>), ApiError> {
    if !auth.token_scopes.contains(&Scope::QuizWrite) && !auth.token_scopes.contains(&Scope::Admin)
    {
        return Err(ApiError::ScopeRequired("quiz.write"));
    }

    let objectives = body.objectives.unwrap_or_default();
    let visibility = body.visibility.unwrap_or(Visibility::Private);

    // TODO: P3 — plan-gate public publishing
    // TODO: P6 — audit log entry for public publishing

    let result = sqlx::query(
        "INSERT INTO tb_quizzes (title, objectives, course, visibility, status, created_by)
         VALUES ($1, $2, $3, $4, 'draft', $5)
         RETURNING id, title, status, visibility, objectives, created_by, created_at",
    )
    .bind(&body.title)
    .bind(&objectives)
    .bind(&body.course)
    .bind(visibility.as_str())
    .bind(auth.user.id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let quiz = CreatedQuiz {
        id: result.get("id"),
        title: result.get("title"),
        status: result.get("status"),
        visibility: result
            .get::<String, _>("visibility")
            .parse()
            .map_err(|e: String| ApiError::Internal(anyhow::anyhow!(e)))?,
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

#[utoipa::path(
    post,
    path = "/v1/quizzes/{id}/questions",
    params(("id" = Uuid, Path, description = "Quiz id")),
    request_body = AddQuizQuestionBody,
    responses(
        (status = 201, description = "Question linked", body = AddQuizQuestionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No such quiz or question"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn add_quiz_question(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuizWriteScopes>,
    Path(quiz_id): Path<Uuid>,
    Json(body): Json<AddQuizQuestionBody>,
) -> Result<(axum::http::StatusCode, Json<AddQuizQuestionResponse>), ApiError> {
    // Verify quiz exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tb_quizzes WHERE id = $1)")
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
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tb_questions WHERE id = $1)")
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
            "INSERT INTO tb_questions (kind, prompt, payload, status, points, created_by)
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
        "SELECT COALESCE(MAX(order_index) + 1, 0) FROM tb_quiz_questions WHERE quiz_id = $1",
    )
    .bind(quiz_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    sqlx::query(
        "INSERT INTO tb_quiz_questions (quiz_id, question_id, order_index, points_override)
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

pub fn default_payload_for_kind(kind: QuestionKind) -> Value {
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

// ── delete quiz ───────────────────────────────────────────────────────────────

#[utoipa::path(
    delete,
    path = "/v1/quizzes/{id}",
    params(("id" = Uuid, Path, description = "Quiz id")),
    responses(
        (status = 204, description = "Quiz deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — only draft quizzes can be deleted"),
        (status = 404, description = "No such quiz"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn delete_quiz(
    State(state): State<AppState>,
    _auth: RequireAnyScope<QuizWriteScopes>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, ApiError> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM tb_quizzes WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    match status.as_deref() {
        None => return Err(ApiError::NotFound { resource: "quiz" }),
        Some(s) if s != "draft" => {
            return Err(ApiError::Validation(vec![FieldError {
                field: "status".into(),
                message: "only draft quizzes can be deleted; archive an active quiz instead".into(),
            }]));
        }
        _ => {}
    }

    // Delete orphaned draft questions that were only used by this quiz
    sqlx::query(
        "DELETE FROM tb_questions
         WHERE status = 'draft'
           AND id IN (SELECT question_id FROM tb_quiz_questions WHERE quiz_id = $1)
           AND NOT EXISTS (
               SELECT 1 FROM tb_quiz_questions
               WHERE question_id = tb_questions.id AND quiz_id != $1
           )",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    sqlx::query("DELETE FROM tb_quizzes WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ── remove question from quiz ─────────────────────────────────────────────────

#[utoipa::path(
    delete,
    path = "/v1/quizzes/{id}/questions/{question_id}",
    params(
        ("id" = Uuid, Path, description = "Quiz id"),
        ("question_id" = Uuid, Path, description = "Question id"),
    ),
    responses(
        (status = 204, description = "Question removed from quiz"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "No such quiz or question"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn remove_quiz_question(
    State(state): State<AppState>,
    _auth: RequireAnyScope<QuizWriteScopes>,
    Path((quiz_id, question_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode, ApiError> {
    let deleted =
        sqlx::query("DELETE FROM tb_quiz_questions WHERE quiz_id = $1 AND question_id = $2")
            .bind(quiz_id)
            .bind(question_id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "quiz_question",
        });
    }

    // Delete the question itself if it's a draft not used by any other quiz
    sqlx::query(
        "DELETE FROM tb_questions
         WHERE id = $1
           AND status = 'draft'
           AND NOT EXISTS (SELECT 1 FROM tb_quiz_questions WHERE question_id = $1)",
    )
    .bind(question_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
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
    auth: RequireAnyScope<QuizReadScopes>,
    Query(_q): Query<CountQuizzesQuery>,
) -> Result<Json<CountQuizzesResponse>, ApiError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tb_quizzes q
         WHERE q.status = $1
           AND (q.visibility = 'public'
                OR EXISTS(SELECT 1 FROM tb_users u
                          WHERE u.id = q.created_by AND (u.id = $2 OR u.owner_user_id = $2)))",
    )
    .bind("active")
    .bind(auth.0.owner_id())
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

#[utoipa::path(
    post,
    path = "/v1/quizzes/generate",
    request_body = GenerateBody,
    responses(
        (status = 200, description = "Generated quiz candidates", body = GenerateResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
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

// ── explore ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResponse {
    pub quizzes: Vec<QuizSummary>,
    pub total: i64,
}

#[utoipa::path(
    get,
    path = "/v1/explore",
    params(
        ("limit" = Option<i64>, Query, description = "Page size (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Page offset"),
    ),
    responses(
        (status = 200, description = "Public quiz list", body = ExploreResponse),
    ),
    security(("bearer" = [])),
    tag = "quizzes"
)]
async fn explore(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(q): Query<ListQuizzesQuery>,
) -> Result<Json<ExploreResponse>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT q.id, q.title, q.status, q.visibility, q.objectives, q.course, q.created_by,
               q.created_at, q.updated_at,
               COUNT(*) OVER() AS total,
               COUNT(qq.question_id) AS question_count,
               EXISTS(
                   SELECT 1 FROM tb_sessions s
                   WHERE s.quiz_id = q.id AND s.user_id = $3 AND s.status = 'finished'
               ) AS completed
        FROM tb_quizzes q
        LEFT JOIN tb_quiz_questions qq ON qq.quiz_id = q.id
        WHERE q.visibility = 'public'
        GROUP BY q.id
        ORDER BY q.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(q.limit)
    .bind(q.offset)
    .bind(auth.owner_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let total: i64 = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let quizzes: Vec<QuizSummary> = rows
        .into_iter()
        .map(|r| {
            Ok(QuizSummary {
                id: r.get("id"),
                title: r.get("title"),
                status: r.get("status"),
                visibility: r
                    .get::<String, _>("visibility")
                    .parse()
                    .map_err(|e: String| ApiError::Internal(anyhow::anyhow!(e)))?,
                objectives: r.get::<Vec<String>, _>("objectives"),
                course: r.get("course"),
                question_count: r.get("question_count"),
                created_by: r.get("created_by"),
                completed: r.get("completed"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(Json(ExploreResponse { quizzes, total }))
}

// ── router ────────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/quizzes", get(list_quizzes).post(create_quiz))
        .route("/v1/explore", get(explore))
        .route("/v1/quizzes/count", get(count_quizzes))
        .route("/v1/quizzes/generate", post(generate_quiz))
        .route(
            "/v1/quizzes/{id}",
            get(get_quiz).patch(patch_quiz).delete(delete_quiz),
        )
        .route("/v1/quizzes/{id}/questions", post(add_quiz_question))
        .route(
            "/v1/quizzes/{id}/questions/{question_id}",
            axum::routing::delete(remove_quiz_question),
        )
        .with_state(state)
}
