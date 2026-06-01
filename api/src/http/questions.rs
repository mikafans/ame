//! Question bank HTTP routes.

use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::scope::{RequireAnyScope, ScopeOneOf},
    bank::questions as repo,
    domain::{
        error::ApiError,
        question::{Question, QuestionKind, QuestionStatus, QuestionVersion},
        user::Scope,
    },
    http::AppState,
};

pub struct QuestionReadScopes;
impl ScopeOneOf for QuestionReadScopes {
    const SCOPES: &'static [Scope] = &[Scope::AssessmentRead, Scope::Admin];
}

pub struct QuestionWriteScopes;
impl ScopeOneOf for QuestionWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::AssessmentWrite, Scope::Admin];
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListQuestionsQuery {
    pub tag: Option<String>,
    pub status: Option<QuestionStatus>,
    pub search: Option<String>,
    pub kind: Option<String>,
    pub min_rating: Option<f64>,
    pub max_rating: Option<f64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub after: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListQuestionsResponse {
    pub questions: Vec<Question>,
    pub total: i64,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateQuestionsBody {
    pub questions: Vec<repo::QuestionInsert>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateQuestionsResponse {
    pub questions: Vec<Question>,
}

#[utoipa::path(
    get,
    path = "/v1/questions",
    params(
        ("tag" = Option<String>, Query, description = "Filter by tag name"),
        ("status" = Option<String>, Query, description = "Filter by status (draft, live, archived)"),
        ("search" = Option<String>, Query, description = "Partial match on prompt"),
        ("kind" = Option<String>, Query, description = "Filter by question kind"),
        ("after" = Option<String>, Query, description = "Pagination cursor"),
    ),
    responses(
        (status = 200, description = "Question list", body = ListQuestionsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn list_questions(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuestionReadScopes>,
    Query(q): Query<ListQuestionsQuery>,
) -> Result<Json<ListQuestionsResponse>, ApiError> {
    let kind = q.kind.and_then(|k| QuestionKind::from_str(&k).ok());

    // Filter by owner unless admin
    let created_by = if auth.0.user.role == crate::domain::user::Role::Admin {
        None
    } else {
        Some(auth.0.owner_id)
    };

    let filter = repo::QuestionFilter {
        tag: q.tag,
        status: q.status,
        search: q.search,
        kind,
        min_rating: q.min_rating,
        max_rating: q.max_rating,
        limit: q.limit,
        offset: q.offset,
        created_by,
    };

    let after = match q.after {
        Some(c) => Some(repo::decode_cursor(&c).ok_or_else(|| {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "after".into(),
                message: "invalid cursor format".into(),
            }])
        })?),
        None => None,
    };

    let paged = repo::list_questions_paged(&state.pool, &filter, after).await?;

    Ok(Json(ListQuestionsResponse {
        questions: paged.rows,
        total: paged.total,
        next_cursor: paged.next_cursor,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/questions/{id}",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Question details", body = Question),
        (status = 404, description = "Question not found"),
    ),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn get_question(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuestionReadScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<Question>, ApiError> {
    let question = repo::get_question(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound {
            resource: "question",
        })?;

    // Check ownership if not admin
    if auth.0.user.role != crate::domain::user::Role::Admin
        && !repo::question_in_owner_scope(&state.pool, id, auth.0.owner_id).await?
    {
        return Err(ApiError::NotFound {
            resource: "question",
        });
    }

    Ok(Json(question))
}

#[utoipa::path(
    get,
    path = "/v1/questions/{id}/versions",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Question history", body = Vec<QuestionVersion>),
        (status = 404, description = "Question not found"),
    ),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn list_versions(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuestionReadScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<QuestionVersion>>, ApiError> {
    let _question = repo::get_question(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound {
            resource: "question",
        })?;

    // Check ownership if not admin
    if auth.0.user.role != crate::domain::user::Role::Admin
        && !repo::question_in_owner_scope(&state.pool, id, auth.0.owner_id).await?
    {
        return Err(ApiError::NotFound {
            resource: "question",
        });
    }

    let versions = repo::get_question_versions(&state.pool, id).await?;
    Ok(Json(versions))
}

#[utoipa::path(
    post,
    path = "/v1/questions",
    request_body = CreateQuestionsBody,
    responses(
        (status = 201, description = "Questions created", body = CreateQuestionsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn create_questions(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuestionWriteScopes>,
    Json(body): Json<CreateQuestionsBody>,
) -> Result<(StatusCode, Json<CreateQuestionsResponse>), ApiError> {
    let questions = repo::create_questions(&state.pool, auth.0.user.id, body.questions).await?;
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
        (status = 200, description = "Question updated", body = Question),
        (status = 404, description = "Question not found"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn update_question(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuestionWriteScopes>,
    Path(id): Path<Uuid>,
    Json(patch): Json<repo::QuestionPatch>,
) -> Result<Json<Question>, ApiError> {
    let _question = repo::get_question(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound {
            resource: "question",
        })?;

    if auth.0.user.role != crate::domain::user::Role::Admin
        && !repo::question_in_owner_scope(&state.pool, id, auth.0.owner_id).await?
    {
        return Err(ApiError::NotFound {
            resource: "question",
        });
    }

    let updated = repo::update_question(&state.pool, id, patch).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    post,
    path = "/v1/questions/{id}/promote",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Question promoted to live", body = Question),
        (status = 404, description = "Question not found"),
    ),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn promote_question(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuestionWriteScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<Question>, ApiError> {
    let _question = repo::get_question(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound {
            resource: "question",
        })?;

    if auth.0.user.role != crate::domain::user::Role::Admin
        && !repo::question_in_owner_scope(&state.pool, id, auth.0.owner_id).await?
    {
        return Err(ApiError::NotFound {
            resource: "question",
        });
    }

    Ok(Json(repo::promote_question(&state.pool, id).await?))
}

#[utoipa::path(
    post,
    path = "/v1/questions/{id}/archive",
    params(("id" = Uuid, Path, description = "Question id")),
    responses(
        (status = 200, description = "Question archived", body = Question),
        (status = 404, description = "Question not found"),
    ),
    security(("bearer" = [])),
    tag = "questions"
)]
pub async fn archive_question(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuestionWriteScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<Question>, ApiError> {
    let _question = repo::get_question(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound {
            resource: "question",
        })?;

    if auth.0.user.role != crate::domain::user::Role::Admin
        && !repo::question_in_owner_scope(&state.pool, id, auth.0.owner_id).await?
    {
        return Err(ApiError::NotFound {
            resource: "question",
        });
    }

    Ok(Json(repo::archive_question(&state.pool, id).await?))
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/questions", get(list_questions).post(create_questions))
        .route(
            "/v1/questions/{id}",
            get(get_question).patch(update_question),
        )
        .route("/v1/questions/{id}/versions", get(list_versions))
        .route("/v1/questions/{id}/promote", post(promote_question))
        .route("/v1/questions/{id}/archive", post(archive_question))
        .with_state(state)
}
