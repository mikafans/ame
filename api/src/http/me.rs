//! Current learner profile route.

use axum::{Json, Router, routing::get};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::extractor::AuthenticatedUser, domain::user::Role, http::AppState};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub role: Role,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

/// Get the authenticated learner profile.
#[utoipa::path(
    get,
    path = "/api/v1/me",
    responses(
        (status = 200, description = "Current learner profile", body = MeResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn get_me(auth: AuthenticatedUser) -> Json<MeResponse> {
    Json(MeResponse {
        id: auth.user.id,
        display_name: auth.user.display_name,
        email: auth.user.email,
        role: auth.user.role,
        created_at: auth.user.created_at,
    })
}

pub fn current_router(state: AppState) -> Router<AppState> {
    Router::new().route("/v1/me", get(get_me)).with_state(state)
}
