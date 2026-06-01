use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use axum_prometheus::PrometheusMetricLayer;
use serde_json::json;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub async fn auth_rate_limit_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::extract::FromRequestParts;
    let (mut parts, body) = req.into_parts();
    if let Ok(auth) =
        crate::auth::extractor::AuthenticatedUser::from_request_parts(&mut parts, &state).await
    {
        parts.extensions.insert(auth);
    }
    let req = axum::extract::Request::from_parts(parts, body);
    next.run(req).await
}

pub mod activity;
pub mod admin;
pub mod agents;
pub mod assessments;
pub mod auth;
pub mod explore;
pub mod export;
pub mod idempotency;
pub mod me;
pub mod messages;
pub mod openapi;
pub mod plans;
pub mod questions;
pub mod quota;
pub mod sessions;
pub mod stats;
pub mod tags;

pub fn metrics_layer() -> (PrometheusMetricLayer<'static>, Router) {
    let (layer, handle) = PrometheusMetricLayer::pair();
    let route = Router::new().route("/metrics", get(|| async move { handle.render() }));
    (layer, route)
}

pub fn router(pool: PgPool) -> Router {
    let state = AppState { pool };

    // Endpoints that should be logged (activity_log)
    let logged_router = Router::new()
        .merge(assessments::router(state.clone()))
        .merge(sessions::router(state.clone()))
        .merge(me::router(state.clone()))
        .merge(plans::router(state.clone()))
        .merge(agents::logged_router(state.clone()))
        .merge(messages::router(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            activity::activity_log_middleware,
        ));

    Router::new()
        .route("/v1/auth/register", post(auth::register))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/logout", post(auth::logout))
        .merge(admin::router(state.clone()))
        .merge(questions::router(state.clone()))
        .merge(explore::router(state.clone()))
        .merge(openapi::router(state.clone()))
        .merge(stats::router(state.clone()))
        .merge(tags::router(state.clone()))
        .merge(agents::public_router(state.clone()))
        .merge(logged_router)
        .route("/healthz", get(healthz))
        .with_state(state)
}

async fn healthz() -> impl axum::response::IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}
