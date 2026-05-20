//! Quiz listing routes: GET /v1/quizzes.

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::scope::{RequireAnyScope, ScopeOneOf},
    domain::{error::ApiError, user::Scope},
    http::AppState,
};

pub struct QuizReadScopes;
impl ScopeOneOf for QuizReadScopes {
    const SCOPES: &'static [Scope] = &[Scope::Human, Scope::QuizRead];
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuizSummary {
    pub id: Uuid,
    pub title: String,
    pub status: String,
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
        "SELECT id, title, status, created_by, created_at, updated_at, COUNT(*) OVER() AS total
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
            created_by: r.get("created_by"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok(Json(ListQuizzesResponse { quizzes, total }))
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/quizzes", get(list_quizzes))
        .with_state(state)
}
