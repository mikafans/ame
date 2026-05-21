//! Tag HTTP routes.
//!
//! `GET /tags` is open to any authenticated user; `POST /tags` requires the
//! `human` scope so agents can't pollute the tag namespace. Tag creation is
//! idempotent on `name` — repeated POSTs with the same name return the
//! existing row.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireScope, ScopeConstraint},
    },
    bank::tags as repo,
    domain::{error::ApiError, question::Tag, user::Scope},
    http::AppState,
};

pub struct TagWriteScope;
impl ScopeConstraint for TagWriteScope {
    const SCOPE: Scope = Scope::QuizWrite;
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTagBody {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/tags",
    responses(
        (status = 200, description = "All tags, sorted by name", body = Vec<Tag>),
        (status = 401, description = "Missing or invalid token"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_tags(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<Vec<Tag>>, ApiError> {
    let tags = repo::list_tags(&state.pool).await?;
    Ok(Json(tags))
}

#[utoipa::path(
    post,
    path = "/v1/tags",
    request_body = CreateTagBody,
    responses(
        (status = 201, description = "Tag created (or already existed)", body = Tag),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_tag(
    State(state): State<AppState>,
    _user: RequireScope<TagWriteScope>,
    Json(body): Json<CreateTagBody>,
) -> Result<impl IntoResponse, ApiError> {
    let tag = repo::create_tag(&state.pool, &body.name, body.description.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

pub fn router(state: AppState) -> axum::Router<AppState> {
    axum::Router::new()
        .route("/v1/tags", get(list_tags).post(create_tag))
        .with_state(state)
}
