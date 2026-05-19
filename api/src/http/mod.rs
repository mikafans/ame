use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use sqlx::PgPool;

/// Shared HTTP state. Cloned per request by Axum, so anything added here must
/// be cheap to clone (PgPool is internally an `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub mod idempotency;

/// Build the production HTTP router.
///
/// Plan 3 (`docs/plans/2026-05-19-plan-3-bank.md`) introduces the first POST
/// routes that accept `Idempotency-Key`; those will attach
/// [`idempotency::idempotency_middleware`] via `route_layer`. `/healthz` is
/// intentionally outside any middleware so a misconfigured DB cannot mask a
/// healthy process.
pub fn router(pool: PgPool) -> Router {
    let state = AppState { pool };
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(state)
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
