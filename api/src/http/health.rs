use crate::http::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

pub async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    // 1. Check Postgres
    let pg_res = sqlx::query("SELECT 1").execute(&state.pool).await;

    if pg_res.is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "error",
                "failed_dependency": "postgres"
            })),
        );
    }

    // 2. Check Valkey
    let valkey_status = match state.valkey.get().await {
        Ok(mut conn) => {
            let ping_res: Result<(), _> = redis::cmd("PING").query_async(&mut *conn).await;
            if ping_res.is_ok() { "ok" } else { "degraded" }
        }
        Err(_) => "degraded",
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
