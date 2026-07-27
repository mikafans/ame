use crate::http::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    // 1. Check Postgres
    if !ame_platform_postgres::health::postgres_ready(&state.pool).await {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "error",
                "failed_dependency": "postgres"
            })),
        );
    }

    // 2. Check Valkey
    let valkey_status = if ame_platform_postgres::health::valkey_ready(&state.valkey).await {
        "ok"
    } else {
        "degraded"
    };

    let status = if valkey_status == "degraded" {
        "degraded"
    } else {
        "ok"
    };

    (
        StatusCode::OK,
        Json(json!({
            "status": status,
            "postgres": "ok",
            "valkey": valkey_status
        })),
    )
}
