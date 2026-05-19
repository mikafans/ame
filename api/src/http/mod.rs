use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub mod idempotency;

pub fn router(pool: PgPool) -> Router {
    let state = AppState { pool };
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(state)
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
