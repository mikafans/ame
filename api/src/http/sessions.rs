//! Session HTTP routes.

use std::{collections::HashMap, str::FromStr};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireAnyScope, ScopeOneOf},
    },
    domain::{
        attempt::{Attempt, AttemptResponse},
        error::{ApiError, FieldError},
        question::{McPayload, QuestionKind},
        session::{QuestionPlan, Session, SessionKind, SessionStatus},
        user::Scope,
    },
    engine::{
        elo::{EloUpdate, QuestionRating, UserTagRating},
        sessions::{
            AnswerInput, AnswerOutcome, RuntimeQuestion, SessionResult, StartSessionInput,
            answer_session, finish_session, start_session,
        },
    },
    http::AppState,
};

pub struct SessionWriteScopes;
impl ScopeOneOf for SessionWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::AttemptWrite];
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionBody {
    #[serde(default)]
    pub quiz_id: Option<Uuid>,
    #[serde(default)]
    pub exam_id: Option<Uuid>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub types: Vec<QuestionKind>,
    #[serde(default)]
    pub count: Option<usize>,
    #[serde(default)]
    pub duration: Option<i64>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub cats: Vec<String>,
    #[serde(default)]
    pub diff: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionResponse {
    pub session_id: Uuid,
    pub questions: Vec<SessionQuestion>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetSessionQuestion {
    pub question_id: Uuid,
    pub kind: QuestionKind,
    pub prompt: String,
    pub points: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_order: Option<Vec<usize>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetSessionResponse {
    pub session: Session,
    pub questions: Vec<GetSessionQuestion>,
    pub attempts: Vec<Attempt>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnswerSessionBody {
    pub question_id: Uuid,
    pub response: AttemptResponse,
    #[serde(default)]
    pub time_to_answer_ms: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnswerSessionResponse {
    pub attempt: Attempt,
    pub grade: crate::engine::graders::GradeOutcome,
    pub replayed: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FinishSessionResponse {
    pub session: Session,
    pub result: SessionResult,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionQuestion {
    pub id: Uuid,
    pub kind: QuestionKind,
    pub prompt: String,
    pub version: i32,
    pub points: i32,
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_order: Option<Vec<usize>>,
}

#[derive(Debug, Clone)]
struct PlannedQuestion {
    question: SessionQuestion,
}

#[utoipa::path(
    post,
    path = "/v1/sessions",
    request_body = CreateSessionBody,
    responses(
        (status = 201, description = "Created session", body = CreateSessionResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_session(
    State(state): State<AppState>,
    user: RequireAnyScope<SessionWriteScopes>,
    Json(body): Json<CreateSessionBody>,
) -> Result<impl IntoResponse, ApiError> {
    let CreatePlan {
        kind,
        filter,
        questions,
        affects_rating: ar_override,
    } = build_plan(&state.pool, user.0.user.id, &body).await?;
    let question_plan = QuestionPlan {
        items: questions
            .iter()
            .map(|planned| crate::domain::session::PlanItem {
                question_id: planned.question.id,
                version: planned.question.version,
                section: None,
                option_order: planned.question.option_order.clone(),
            })
            .collect(),
    };
    let started_at = OffsetDateTime::now_utc();
    let session = start_session(StartSessionInput {
        user_id: user.0.user.id,
        kind,
        quiz_id: body.quiz_id,
        exam_id: body.exam_id,
        filter,
        question_plan,
        affects_rating: ar_override.unwrap_or(kind != SessionKind::Exam),
        rating_snapshot: Default::default(),
        duration_min: body.duration,
        now: started_at,
    })?;
    insert_session(&state.pool, &session).await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateSessionResponse {
            session_id: session.id,
            questions: questions
                .into_iter()
                .map(|planned| planned.question)
                .collect(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/sessions/{id}",
    params(("id" = Uuid, Path, description = "Session id")),
    responses(
        (status = 200, description = "Session resume state", body = GetSessionResponse),
        (status = 404, description = "No such session"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<GetSessionResponse>, ApiError> {
    let session = get_owned_session(&state.pool, id, user.user.id).await?;
    let attempts = list_session_attempts(&state.pool, session.id).await?;
    let questions = hydrate_session_questions(&state.pool, &session).await?;
    Ok(Json(GetSessionResponse {
        session,
        questions,
        attempts,
    }))
}

async fn hydrate_session_questions(
    pool: &PgPool,
    session: &Session,
) -> Result<Vec<GetSessionQuestion>, ApiError> {
    let plan_ids: Vec<Uuid> = session
        .question_plan
        .items
        .iter()
        .map(|i| i.question_id)
        .collect();

    if plan_ids.is_empty() {
        return Ok(vec![]);
    }

    let rows = sqlx::query(
        "SELECT id, kind, prompt, points, code_snippet, payload \
         FROM questions WHERE id = ANY($1)",
    )
    .bind(&plan_ids)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    // preserve plan order
    let mut by_id: std::collections::HashMap<Uuid, _> = rows
        .into_iter()
        .map(|r| (r.get::<Uuid, _>("id"), r))
        .collect();

    session
        .question_plan
        .items
        .iter()
        .filter_map(|item| {
            let row = by_id.remove(&item.question_id)?;
            let kind_str: String = row.get("kind");
            let kind = QuestionKind::from_str(&kind_str).ok()?;
            let payload: serde_json::Value = row.get("payload");
            let options = payload
                .get("options")
                .cloned()
                .and_then(|v| v.as_array().cloned())
                .map(|arr| {
                    arr.into_iter()
                        .map(|opt| serde_json::json!({"text": opt}))
                        .collect()
                });
            Some(Ok(GetSessionQuestion {
                question_id: item.question_id,
                kind,
                prompt: row.get("prompt"),
                points: row.get("points"),
                code_snippet: row.get("code_snippet"),
                options,
                option_order: item.option_order.clone(),
            }))
        })
        .collect()
}

#[utoipa::path(
    post,
    path = "/v1/sessions/{id}/answer",
    params(("id" = Uuid, Path, description = "Session id")),
    request_body = AnswerSessionBody,
    responses(
        (status = 200, description = "Stored answer", body = AnswerSessionResponse),
        (status = 410, description = "Session expired"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn answer(
    State(state): State<AppState>,
    user: RequireAnyScope<SessionWriteScopes>,
    Path(id): Path<Uuid>,
    Json(body): Json<AnswerSessionBody>,
) -> Result<Json<AnswerSessionResponse>, ApiError> {
    let session = get_owned_session(&state.pool, id, user.0.user.id).await?;
    let existing = existing_attempts_by_question(&state.pool, session.id).await?;
    let runtime_question =
        load_runtime_question(&state.pool, user.0.user.id, body.question_id).await?;
    let outcome = answer_session(
        AnswerInput {
            session,
            question: runtime_question,
            response: body.response,
            time_to_answer_ms: body.time_to_answer_ms,
            now: OffsetDateTime::now_utc(),
        },
        &existing,
    )?;

    if !outcome.replayed {
        insert_attempt(&state.pool, &outcome).await?;
        if let Some(ref elo) = outcome.elo {
            persist_elo(&state.pool, user.0.user.id, body.question_id, elo).await?;
        }
    }

    Ok(Json(AnswerSessionResponse {
        attempt: outcome.attempt,
        grade: outcome.grade,
        replayed: outcome.replayed,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/sessions/{id}/finish",
    params(("id" = Uuid, Path, description = "Session id")),
    responses(
        (status = 200, description = "Finished session", body = FinishSessionResponse),
        (status = 404, description = "No such session"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn finish(
    State(state): State<AppState>,
    user: RequireAnyScope<SessionWriteScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<FinishSessionResponse>, ApiError> {
    let session = get_owned_session(&state.pool, id, user.0.user.id).await?;
    let attempts = list_session_attempts(&state.pool, session.id).await?;
    let max_points = max_points_by_question(&state.pool, &attempts).await?;
    let finished = finish_session(session, &attempts, &max_points, OffsetDateTime::now_utc())?;
    update_finished_session(&state.pool, &finished).await?;
    let result = serde_json::from_value(
        finished
            .result
            .clone()
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("missing session result")))?,
    )
    .map_err(anyhow::Error::from)?;

    Ok(Json(FinishSessionResponse {
        session: finished,
        result,
    }))
}

struct CreatePlan {
    kind: SessionKind,
    filter: Option<serde_json::Value>,
    questions: Vec<PlannedQuestion>,
    /// None = derive from kind; Some = override (exam uses exam.affects_rating)
    affects_rating: Option<bool>,
}

async fn build_plan(
    pool: &PgPool,
    user_id: Uuid,
    body: &CreateSessionBody,
) -> Result<CreatePlan, ApiError> {
    if let Some(quiz_id) = body.quiz_id {
        return build_quiz_plan(pool, quiz_id).await;
    }
    if let Some(exam_id) = body.exam_id {
        return build_exam_plan(pool, exam_id).await;
    }

    let count = body.count.unwrap_or(10).clamp(1, 100);
    let types: Vec<String> = body
        .types
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect();
    let tags = normalize_strings(&body.tags);
    let rows = sqlx::query(
        "SELECT q.id, q.kind, q.prompt, q.payload, q.version, q.points \
         FROM questions q \
         WHERE q.status = 'live' \
           AND (cardinality($1::text[]) = 0 OR q.kind = ANY($1::text[])) \
           AND (cardinality($2::text[]) = 0 OR EXISTS ( \
                SELECT 1 FROM question_tags qt \
                JOIN tags t ON t.id = qt.tag_id \
                WHERE qt.question_id = q.id AND t.name = ANY($2::text[]) \
           )) \
           AND NOT EXISTS ( \
                SELECT 1 FROM attempts a \
                WHERE a.user_id = $3 AND a.question_id = q.id \
                  AND a.created_at >= now() - interval '24 hours' \
           ) \
         ORDER BY q.created_at DESC \
         LIMIT $4",
    )
    .bind(&types)
    .bind(&tags)
    .bind(user_id)
    .bind(count as i64)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    if rows.is_empty() {
        return Err(ApiError::ExamPoolInsufficient {
            section: "practice".to_string(),
            required: count,
            available: 0,
        });
    }

    let mut questions = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row.get("id");
        let kind_str: String = row.get("kind");
        let kind = QuestionKind::from_str(&kind_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid kind in DB: {e}")))?;
        let payload: serde_json::Value = row.get("payload");
        let option_order = option_order(kind, &payload)?;
        questions.push(PlannedQuestion {
            question: SessionQuestion {
                id,
                kind,
                prompt: row.get("prompt"),
                version: row.get("version"),
                points: row.get("points"),
                payload,
                option_order,
            },
        });
    }

    Ok(CreatePlan {
        kind: SessionKind::Practice,
        filter: Some(serde_json::json!({
            "tags": tags,
            "types": types,
            "count": count,
            "duration": body.duration,
            "mode": body.mode,
            "cats": body.cats,
            "diff": body.diff,
        })),
        questions,
        affects_rating: None,
    })
}

async fn build_quiz_plan(pool: &PgPool, quiz_id: Uuid) -> Result<CreatePlan, ApiError> {
    sqlx::query("SELECT id FROM quizzes WHERE id = $1 AND status = 'active'")
        .bind(quiz_id)
        .fetch_optional(pool)
        .await
        .map_err(internal)?
        .ok_or(ApiError::NotFound { resource: "quiz" })?;

    let rows = sqlx::query(
        "SELECT q.id, q.kind, q.prompt, q.payload, q.version, q.points \
         FROM quiz_questions qq \
         JOIN questions q ON q.id = qq.question_id \
         WHERE qq.quiz_id = $1 AND q.status = 'live' \
         ORDER BY qq.order_index ASC",
    )
    .bind(quiz_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    if rows.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "quizId".to_string(),
            message: "quiz has no live questions".to_string(),
        }]));
    }

    let mut questions = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row.get("id");
        let kind_str: String = row.get("kind");
        let kind = QuestionKind::from_str(&kind_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid kind in DB: {e}")))?;
        let payload: serde_json::Value = row.get("payload");
        let option_order = option_order(kind, &payload)?;
        questions.push(PlannedQuestion {
            question: SessionQuestion {
                id,
                kind,
                prompt: row.get("prompt"),
                version: row.get("version"),
                points: row.get("points"),
                payload,
                option_order,
            },
        });
    }

    Ok(CreatePlan {
        kind: SessionKind::Quiz,
        filter: None,
        questions,
        affects_rating: None,
    })
}

async fn build_exam_plan(pool: &PgPool, exam_id: Uuid) -> Result<CreatePlan, ApiError> {
    let exam_row =
        sqlx::query("SELECT id, affects_rating FROM exams WHERE id = $1 AND status = 'published'")
            .bind(exam_id)
            .fetch_optional(pool)
            .await
            .map_err(internal)?
            .ok_or(ApiError::NotFound { resource: "exam" })?;

    let affects_rating: bool = exam_row.get("affects_rating");

    let section_rows = sqlx::query(
        "SELECT question_ids, order_index FROM exam_sections \
         WHERE exam_id = $1 ORDER BY order_index ASC",
    )
    .bind(exam_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    if section_rows.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "examId".to_string(),
            message: "exam has no sections".to_string(),
        }]));
    }

    // Collect all question_ids across sections in order
    let mut all_ids: Vec<Uuid> = Vec::new();
    for row in &section_rows {
        let ids: Option<Vec<Uuid>> = row.get("question_ids");
        if let Some(ids) = ids {
            all_ids.extend(ids);
        }
    }

    if all_ids.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "examId".to_string(),
            message: "exam sections contain no questions".to_string(),
        }]));
    }

    let q_rows = sqlx::query(
        "SELECT q.id, q.kind, q.prompt, q.payload, q.version, q.points \
         FROM questions q WHERE q.id = ANY($1::uuid[]) AND q.status = 'live'",
    )
    .bind(&all_ids)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    // Build ordered questions matching all_ids order
    let q_map: std::collections::HashMap<Uuid, _> = q_rows
        .into_iter()
        .map(|r| (r.get::<Uuid, _>("id"), r))
        .collect();

    let mut questions = Vec::new();
    for id in &all_ids {
        let row = q_map.get(id).ok_or(ApiError::NotFound {
            resource: "question",
        })?;
        let kind_str: String = row.get("kind");
        let kind = QuestionKind::from_str(&kind_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid kind in DB: {e}")))?;
        let payload: serde_json::Value = row.get("payload");
        let option_order = option_order(kind, &payload)?;
        questions.push(PlannedQuestion {
            question: SessionQuestion {
                id: *id,
                kind,
                prompt: row.get("prompt"),
                version: row.get("version"),
                points: row.get("points"),
                payload,
                option_order,
            },
        });
    }

    Ok(CreatePlan {
        kind: SessionKind::Exam,
        filter: None,
        questions,
        affects_rating: Some(affects_rating),
    })
}

fn option_order(
    kind: QuestionKind,
    payload: &serde_json::Value,
) -> Result<Option<Vec<usize>>, ApiError> {
    if kind != QuestionKind::Mc {
        return Ok(None);
    }
    let payload: McPayload =
        serde_json::from_value(payload.clone()).map_err(|e| ApiError::InvalidPayload {
            kind: kind.as_str().to_string(),
            reason: e.to_string(),
        })?;
    let mut order: Vec<usize> = (0..payload.options.len()).collect();
    order.shuffle(&mut rand::thread_rng());
    Ok(Some(order))
}

async fn insert_session(pool: &PgPool, session: &Session) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO sessions \
         (id, user_id, kind, quiz_id, exam_id, filter, question_plan, status, affects_rating, \
          rating_snapshot, result, deadline_at, started_at, finished_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(session.id)
    .bind(session.user_id)
    .bind(session.kind.as_str())
    .bind(session.quiz_id)
    .bind(session.exam_id)
    .bind(&session.filter)
    .bind(serde_json::to_value(&session.question_plan).map_err(anyhow::Error::from)?)
    .bind(session.status.as_str())
    .bind(session.affects_rating)
    .bind(serde_json::to_value(&session.rating_snapshot).map_err(anyhow::Error::from)?)
    .bind(&session.result)
    .bind(session.deadline_at)
    .bind(session.started_at)
    .bind(session.finished_at)
    .execute(pool)
    .await
    .map_err(internal)?;
    Ok(())
}

async fn get_owned_session(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Session, ApiError> {
    let row = sqlx::query(
        "SELECT id, user_id, kind, quiz_id, exam_id, filter, question_plan, status, affects_rating, \
                rating_snapshot, result, deadline_at, started_at, finished_at \
         FROM sessions WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound {
        resource: "session",
    })?;
    row_to_session(&row)
}

fn row_to_session(row: &sqlx::postgres::PgRow) -> Result<Session, ApiError> {
    let kind_str: String = row.get("kind");
    let status_str: String = row.get("status");
    let question_plan: serde_json::Value = row.get("question_plan");
    let rating_snapshot: serde_json::Value = row.get("rating_snapshot");

    Ok(Session {
        id: row.get("id"),
        user_id: row.get("user_id"),
        kind: SessionKind::from_str(&kind_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid session kind: {e}")))?,
        quiz_id: row.get("quiz_id"),
        exam_id: row.get("exam_id"),
        filter: row.get("filter"),
        question_plan: serde_json::from_value(question_plan).map_err(anyhow::Error::from)?,
        status: SessionStatus::from_str(&status_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid session status: {e}")))?,
        affects_rating: row.get("affects_rating"),
        rating_snapshot: serde_json::from_value(rating_snapshot).map_err(anyhow::Error::from)?,
        result: row.get("result"),
        deadline_at: row.get("deadline_at"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
    })
}

async fn load_runtime_question(
    pool: &PgPool,
    user_id: Uuid,
    question_id: Uuid,
) -> Result<RuntimeQuestion, ApiError> {
    let row = sqlx::query(
        "SELECT id, kind, payload, version, points, rating, attempts_count \
         FROM questions WHERE id = $1",
    )
    .bind(question_id)
    .fetch_optional(pool)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound {
        resource: "question",
    })?;
    let kind_str: String = row.get("kind");
    let kind = QuestionKind::from_str(&kind_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid kind in DB: {e}")))?;
    Ok(RuntimeQuestion {
        question_id: row.get("id"),
        version: row.get("version"),
        kind,
        payload: row.get("payload"),
        max_points: row.get("points"),
        rating: QuestionRating {
            rating: row.get("rating"),
            attempts_count: row.get("attempts_count"),
        },
        user_tag_ratings: load_user_tag_ratings(pool, user_id, question_id).await?,
    })
}

async fn load_user_tag_ratings(
    pool: &PgPool,
    user_id: Uuid,
    question_id: Uuid,
) -> Result<Vec<UserTagRating>, ApiError> {
    let rows = sqlx::query(
        "SELECT t.id, COALESCE(utr.rating, 1200::double precision) AS rating, \
                COALESCE(utr.attempts_count, 0) AS attempts_count \
         FROM question_tags qt \
         JOIN tags t ON t.id = qt.tag_id \
         LEFT JOIN user_tag_ratings utr ON utr.tag_id = t.id AND utr.user_id = $1 \
         WHERE qt.question_id = $2",
    )
    .bind(user_id)
    .bind(question_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    Ok(rows
        .into_iter()
        .map(|row| UserTagRating {
            tag_id: row.get("id"),
            rating: row.get("rating"),
            attempts_count: row.get("attempts_count"),
        })
        .collect())
}

async fn existing_attempts_by_question(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<HashMap<Uuid, Attempt>, ApiError> {
    Ok(list_session_attempts(pool, session_id)
        .await?
        .into_iter()
        .map(|attempt| (attempt.question_id, attempt))
        .collect())
}

async fn list_session_attempts(pool: &PgPool, session_id: Uuid) -> Result<Vec<Attempt>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, user_id, question_id, question_version, session_id, response, presentation, \
                is_correct, score, time_to_answer_ms, rating_before_user_avg, \
                rating_before_question, user_tag_deltas, question_delta, created_at \
         FROM attempts WHERE session_id = $1 ORDER BY created_at ASC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    rows.into_iter().map(row_to_attempt).collect()
}

fn row_to_attempt(row: sqlx::postgres::PgRow) -> Result<Attempt, ApiError> {
    let response: serde_json::Value = row.get("response");
    let presentation: serde_json::Value = row.get("presentation");
    let user_tag_deltas: serde_json::Value = row.get("user_tag_deltas");
    Ok(Attempt {
        id: row.get("id"),
        user_id: row.get("user_id"),
        question_id: row.get("question_id"),
        question_version: row.get("question_version"),
        session_id: row.get("session_id"),
        response: serde_json::from_value(response).map_err(anyhow::Error::from)?,
        presentation: serde_json::from_value(presentation).map_err(anyhow::Error::from)?,
        is_correct: row.get("is_correct"),
        score: row.get("score"),
        time_to_answer_ms: row.get("time_to_answer_ms"),
        rating_before_user_avg: row.get("rating_before_user_avg"),
        rating_before_question: row.get("rating_before_question"),
        user_tag_deltas: serde_json::from_value(user_tag_deltas).map_err(anyhow::Error::from)?,
        question_delta: row.get("question_delta"),
        created_at: row.get("created_at"),
    })
}

async fn insert_attempt(pool: &PgPool, outcome: &AnswerOutcome) -> Result<(), ApiError> {
    let attempt = &outcome.attempt;
    sqlx::query(
        "INSERT INTO attempts \
         (id, user_id, question_id, question_version, session_id, response, presentation, \
          is_correct, score, time_to_answer_ms, rating_before_user_avg, rating_before_question, \
          user_tag_deltas, question_delta, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(attempt.id)
    .bind(attempt.user_id)
    .bind(attempt.question_id)
    .bind(attempt.question_version)
    .bind(attempt.session_id)
    .bind(serde_json::to_value(&attempt.response).map_err(anyhow::Error::from)?)
    .bind(serde_json::to_value(&attempt.presentation).map_err(anyhow::Error::from)?)
    .bind(attempt.is_correct)
    .bind(attempt.score)
    .bind(attempt.time_to_answer_ms)
    .bind(attempt.rating_before_user_avg)
    .bind(attempt.rating_before_question)
    .bind(serde_json::to_value(&attempt.user_tag_deltas).map_err(anyhow::Error::from)?)
    .bind(attempt.question_delta)
    .bind(attempt.created_at)
    .execute(pool)
    .await
    .map_err(internal)?;
    Ok(())
}

async fn max_points_by_question(
    pool: &PgPool,
    attempts: &[Attempt],
) -> Result<HashMap<Uuid, i32>, ApiError> {
    if attempts.is_empty() {
        return Ok(HashMap::new());
    }
    let ids: Vec<Uuid> = attempts.iter().map(|attempt| attempt.question_id).collect();
    let rows = sqlx::query("SELECT id, points FROM questions WHERE id = ANY($1::uuid[])")
        .bind(&ids)
        .fetch_all(pool)
        .await
        .map_err(internal)?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("id"), row.get("points")))
        .collect())
}

async fn update_finished_session(pool: &PgPool, session: &Session) -> Result<(), ApiError> {
    sqlx::query("UPDATE sessions SET status = $2, result = $3, finished_at = $4 WHERE id = $1")
        .bind(session.id)
        .bind(session.status.as_str())
        .bind(&session.result)
        .bind(session.finished_at)
        .execute(pool)
        .await
        .map_err(internal)?;
    Ok(())
}

async fn persist_elo(
    pool: &PgPool,
    user_id: Uuid,
    question_id: Uuid,
    elo: &EloUpdate,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE questions SET rating = $2, attempts_count = $3 WHERE id = $1")
        .bind(question_id)
        .bind(elo.question.rating_after)
        .bind(elo.question.attempts_after)
        .execute(pool)
        .await
        .map_err(internal)?;

    for tag in &elo.user_tags {
        sqlx::query(
            "INSERT INTO user_tag_ratings (user_id, tag_id, rating, attempts_count, last_updated) \
             VALUES ($1, $2, $3, $4, now()) \
             ON CONFLICT (user_id, tag_id) DO UPDATE \
             SET rating = EXCLUDED.rating, \
                 attempts_count = EXCLUDED.attempts_count, \
                 last_updated = now()",
        )
        .bind(user_id)
        .bind(tag.tag_id)
        .bind(tag.rating_after)
        .bind(tag.attempts_after)
        .execute(pool)
        .await
        .map_err(internal)?;
    }

    Ok(())
}

fn normalize_strings(values: &[String]) -> Vec<String> {
    let mut out = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/sessions", post(create_session))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/answer", post(answer))
        .route("/v1/sessions/{id}/finish", post(finish))
        .with_state(state)
}
