use axum::{Json, Router, middleware, routing::get};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Shared HTTP state. Cloned per request by Axum, so anything added here must
/// be cheap to clone (PgPool is internally an `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub mod activity;
pub mod admin;
pub mod agents;
pub mod auth;
pub mod exams;
pub mod idempotency;
pub mod me;
pub mod messages;
pub mod openapi;
pub mod plans;
pub mod questions;
pub mod quizzes;
pub mod sessions;
pub mod shares;
pub mod stats;
pub mod tags;

/// Build the production HTTP router.
///
/// `/healthz` is intentionally outside any middleware so a misconfigured DB
/// cannot mask a healthy process. Question/tag routes live under their own
/// modules and bring their own scope guards + idempotency wiring.
///
/// Embed routes are declared inside `shares::router` before the plain resource
/// routes so the longer `/embed` path wins in Axum's router.
pub fn router(pool: PgPool) -> Router {
    let state = AppState { pool };

    // Routes wrapped with activity-log middleware (records all authenticated calls)
    let logged = Router::new()
        .merge(tags::router(state.clone()))
        .merge(questions::router(state.clone()))
        .merge(sessions::router(state.clone()))
        .merge(exams::router(state.clone()))
        .merge(plans::router(state.clone()))
        .merge(quizzes::router(state.clone()))
        .merge(stats::router(state.clone()))
        .merge(messages::router(state.clone()))
        .merge(me::router(state.clone()))
        .merge(admin::router(state.clone()))
        .merge(agents::router(state.clone()))
        .merge(shares::router(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            activity::activity_log_middleware,
        ));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let trace = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    Router::new()
        .route("/healthz", get(healthz))
        .merge(openapi::router(state.clone()))
        .merge(auth::router(state.clone()))
        .merge(logged)
        .layer(cors)
        .layer(trace)
        .with_state(state)
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
