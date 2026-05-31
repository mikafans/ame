use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{auth::extractor::AuthenticatedUser, domain::error::ApiError, http::AppState};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreQuery {
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub after: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResponse {
    pub items: Vec<serde_json::Value>,
    pub next_cursor: Option<String>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/explore", get(explore))
        .with_state(state)
}

pub async fn explore(
    State(_state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(_q): Query<ExploreQuery>,
) -> Result<Json<ExploreResponse>, ApiError> {
    // TODO: Implement paginated database query (keyset seek)
    Ok(Json(ExploreResponse {
        items: vec![],
        next_cursor: None,
    }))
}
