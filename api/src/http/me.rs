//! Current learner profile route.

use axum::{Json, Router, extract::State, routing::get};
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
    pub plan: String,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitStatusResponse {
    pub tier: String,
    pub limit: u32,
    pub remaining: u32,
    pub reset_after_seconds: u64,
    pub request_cost: u32,
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
        plan: auth.user.plan,
        created_at: auth.user.created_at,
    })
}

/// Read the current authenticated learner rate-limit bucket without consuming
/// an additional request from it.
#[utoipa::path(
    get,
    path = "/api/v1/me/rate-limit",
    responses(
        (status = 200, description = "Current learner rate-limit bucket", body = RateLimitStatusResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn get_rate_limit(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Json<RateLimitStatusResponse> {
    let settings = crate::settings::get_effective(&state.pool, &state.valkey, &state.config).await;
    let mut request_limit = crate::ratelimit::authenticated_request_limit(
        &state,
        &auth,
        &axum::http::Method::GET,
        "/api/v1/me/rate-limit",
    );
    let tier = if auth.user.plan == "premium" {
        &settings.ratelimit.premium
    } else {
        &settings.ratelimit.free
    };
    request_limit.burst = tier.burst;
    request_limit.refill_rate = tier.rate as f64;
    let snapshot = state
        .limiter
        .inspect(
            &request_limit.key,
            request_limit.burst,
            request_limit.refill_rate,
        )
        .await;
    Json(RateLimitStatusResponse {
        tier: auth.user.plan.clone(),
        limit: snapshot.limit,
        remaining: snapshot.remaining,
        reset_after_seconds: snapshot.reset_after_seconds,
        request_cost: request_limit.cost,
    })
}

pub fn current_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/me", get(get_me))
        .route("/v1/me/rate-limit", get(get_rate_limit))
        .with_state(state)
}
