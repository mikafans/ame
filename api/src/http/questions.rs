//! Question HTTP routes.
//!
//! - `GET /questions`, `GET /questions/{id}` — open to any authenticated user
//!   (including `agent:read-only`).
//! - Writes (`POST`, `PATCH`, promote, archive) require the `human` or
//!   `agent:write-questions` scope. This is the "either" gate spec'd in
//!   §"Tokens & scopes" — agent:read-only must not be able to inflate the
//!   bank.
//! - `POST /questions` participates in the idempotency middleware (see the
//!   parent router) so retried batch ingests don't double-insert.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireAnyScope, ScopeOneOf},
    },
    bank::questions as repo,
    domain::{
        error::{ApiError, FieldError},
        question::{Question, QuestionStatus, QuestionVersion},
        user::Scope,
    },
    http::{AppState, idempotency::idempotency_middleware},
};

pub struct WriteQuestionScopes;
impl ScopeOneOf for WriteQuestionScopes {
    const SCOPES: &'static [Scope] = &[Scope::QuizWrite];
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListQuestionsQuery {
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub status: Option<QuestionStatus>,
    #[serde(default)]
    pub min_rating: Option<f64>,
    #[serde(default)]
    pub max_rating: Option<f64>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub page_size: Option<i64>,
    /// Keyset cursor from a prior response's `nextCursor`; when set, `page` is
    /// ignored and the next rows after the cursor are returned.
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateQuestionsBody {
    pub questions: Vec<repo::QuestionInsert>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct QuestionListResponse {
    pub questions: Vec<Question>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuestionBankListResponse {
    pub questions: Vec<repo::QuestionRow>,
    pub total: i64,
    /// Cursor to fetch the next page via `?cursor=`; null when there are no
    /// more rows or the query used an ordering that doesn't support keyset.
    pub next_cursor: Option<String>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CreateQuestionsResponse {
    pub questions: Vec<Question>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct QuestionVersionsResponse {
    pub versions: Vec<QuestionVersion>,
}

#[utoipa::path(
    get,
    path = "/v1/questions",
    params(ListQuestionsQuery),
    responses(
        (status = 200, description = "Filtered list of questions", body = QuestionListResponse),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_questions(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(q): Query<ListQuestionsQuery>,
) -> Result<Response, ApiError> {
    // If using new pagination params (page/pageSize/search/kind/cursor), use
    // list_questions_paged.
    if q.page.is_some()
        || q.page_size.is_some()
        || q.search.is_some()
        || q.kind.is_some()
        || q.cursor.is_some()
    {
        // A keyset cursor seeks past a position, so it supersedes page/offset.
        let after = match q.cursor.as_deref() {
            Some(c) => Some(repo::decode_cursor(c).ok_or_else(|| {
                ApiError::Validation(vec![FieldError {
                    field: "cursor".into(),
                    message: "malformed cursor".into(),
                }])
            })?),
            None => None,
        };
        let page = q.page.unwrap_or(1).max(1);
        let page_size = q.page_size.unwrap_or(25).clamp(1, 200);
        let offset = if after.is_some() {
            0
        } else {
            (page - 1) * page_size
        };

        let filter = repo::QuestionFilter {
            tag: q.tag,
            status: q.status,
            min_rating: q.min_rating,
            max_rating: q.max_rating,
            limit: page_size,
            offset,
            search: q.search,
            kind: q.kind,
        };

        let paged = repo::list_questions_paged(&state.pool, &filter, after).await?;
        return Ok(Json(QuestionBankListResponse {
            questions: paged.rows,
            total: paged.total,
            next_cursor: paged.next_cursor,
        })
        .into_response());
    }

    // Fall back to old API for backward compatibility
    let filter = repo::QuestionFilter {
        tag: q.tag,
        status: q.status,
        min_rating: q.min_rating,
        max_rating: q.max_rating,
        limit: q.limit.unwrap_or(50),
        offset: q.offset.unwrap_or(0),
        search: None,
        kind: None,
    };
    let questions = repo::list_questions(&state.pool, &filter).await?;
    Ok(Json(QuestionListResponse { questions }).into_response())
}

#[utoipa::path(
    get,
    path = "/v1/questions/{id}",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Question by id", body = Question),
        (status = 404, description = "No such question"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_question(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Question>, ApiError> {
    let question = repo::get_question(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound {
            resource: "question",
        })?;
    Ok(Json(question))
}

#[utoipa::path(
    get,
    path = "/v1/questions/{id}/versions",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Historical versions, newest first", body = QuestionVersionsResponse),
        (status = 404, description = "No such question"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_versions(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<QuestionVersionsResponse>, ApiError> {
    // Confirm the parent exists so callers get a 404 instead of an empty list
    // for a non-existent id.
    if repo::get_question(&state.pool, id).await?.is_none() {
        return Err(ApiError::NotFound {
            resource: "question",
        });
    }
    let versions = repo::list_question_versions(&state.pool, id).await?;
    Ok(Json(QuestionVersionsResponse { versions }))
}

#[utoipa::path(
    post,
    path = "/v1/questions",
    request_body = CreateQuestionsBody,
    responses(
        (status = 201, description = "Batch of created questions", body = CreateQuestionsResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 422, description = "Validation failed"),
        (status = 409, description = "Idempotency key conflict"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_questions(
    State(state): State<AppState>,
    user: RequireAnyScope<WriteQuestionScopes>,
    Json(body): Json<CreateQuestionsBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.questions.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "questions".into(),
            message: "must contain at least one question".into(),
        }]));
    }
    let questions = repo::create_questions(&state.pool, user.0.user.id, body.questions).await?;
    Ok((
        StatusCode::CREATED,
        Json(CreateQuestionsResponse { questions }),
    ))
}

#[utoipa::path(
    patch,
    path = "/v1/questions/{id}",
    params(("id" = Uuid, Path, description = "Question id")),
    request_body = repo::QuestionPatch,
    responses(
        (status = 200, description = "Updated question", body = Question),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 404, description = "No such question"),
        (status = 422, description = "Cannot edit archived question"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_question(
    State(state): State<AppState>,
    _user: RequireAnyScope<WriteQuestionScopes>,
    Path(id): Path<Uuid>,
    Json(patch): Json<repo::QuestionPatch>,
) -> Result<Json<Question>, ApiError> {
    let updated = repo::update_question(&state.pool, id, patch).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    post,
    path = "/v1/questions/{id}/promote",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Question promoted to live", body = Question),
        (status = 404, description = "No such question"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn promote_question(
    State(state): State<AppState>,
    _user: RequireAnyScope<WriteQuestionScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<Question>, ApiError> {
    Ok(Json(repo::promote_question(&state.pool, id).await?))
}

#[utoipa::path(
    post,
    path = "/v1/questions/{id}/archive",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Question archived", body = Question),
        (status = 404, description = "No such question"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn archive_question(
    State(state): State<AppState>,
    _user: RequireAnyScope<WriteQuestionScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<Question>, ApiError> {
    Ok(Json(repo::archive_question(&state.pool, id).await?))
}

pub fn router(state: AppState) -> Router<AppState> {
    // POST /questions runs under the idempotency middleware so retried batch
    // ingests don't double-insert; the other routes don't accept the header
    // and don't need it.
    let create_only = Router::new()
        .route("/v1/questions", post(create_questions))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency_middleware,
        ));

    let rest = Router::new()
        .route("/v1/questions", get(list_questions))
        .route(
            "/v1/questions/{id}",
            get(get_question).patch(update_question),
        )
        .route("/v1/questions/{id}/versions", get(list_versions))
        .route("/v1/questions/{id}/promote", post(promote_question))
        .route("/v1/questions/{id}/archive", post(archive_question));

    create_only.merge(rest).with_state(state)
}
