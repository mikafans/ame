use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use sqlx::PgPool;

/// Shared HTTP state. Cloned per request by Axum, so anything added here must
/// be cheap to clone (PgPool is internally an `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub mod exams;
pub mod idempotency;
pub mod openapi;
pub mod plans;
pub mod questions;
pub mod sessions;
pub mod tags;

/// Build the production HTTP router.
///
/// `/healthz` is intentionally outside any middleware so a misconfigured DB
/// cannot mask a healthy process. Question/tag routes live under their own
/// modules and bring their own scope guards + idempotency wiring.
pub fn router(pool: PgPool) -> Router {
    let state = AppState { pool };
    Router::new()
        .route("/healthz", get(healthz))
        .merge(tags::router(state.clone()))
        .merge(questions::router(state.clone()))
        .merge(sessions::router(state.clone()))
        .merge(exams::router(state.clone()))
        .merge(plans::router(state.clone()))
        .merge(openapi::router(state.clone()))
        .with_state(state)
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
