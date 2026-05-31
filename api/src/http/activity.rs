//! Activity log middleware: records authenticated agent API calls to `activity_log`.
//!
//! The middleware fires a background task after every request that carries a
//! parseable Bearer token. The background task does a lightweight token→user lookup
//! (no secret verify — just the token id) and inserts a row into `activity_log`.
//! Failures are swallowed silently so a DB hiccup never breaks the request path.

use axum::{
    extract::{MatchedPath, Request, State},
    middleware::Next,
    response::Response,
};
use sqlx::Row;

use crate::{auth::token::parse_bearer_token, http::AppState};

/// Mapping from (METHOD, matched-path-template) to MCP tool name.
fn derive_tool_name(method: &str, path: &str) -> String {
    match (method, path) {
        ("GET", "/v1/assessments") => "assessment.list",
        ("GET", "/v1/assessments/:id") | ("GET", "/v1/assessments/{id}") => "assessment.get",
        ("POST", "/v1/assessments") => "assessment.import",
        ("PATCH", "/v1/assessments/:id") | ("PATCH", "/v1/assessments/{id}") => "assessment.update",
        ("DELETE", "/v1/assessments/:id") | ("DELETE", "/v1/assessments/{id}") => {
            "assessment.delete"
        }
        ("GET", "/v1/assessments/:id/stats") | ("GET", "/v1/assessments/{id}/stats") => {
            "stats.cohort"
        }
        ("GET", "/v1/exams") => "exam.list",
        ("GET", "/v1/exams/:id") | ("GET", "/v1/exams/{id}") => "exam.get",
        ("POST", "/v1/exams") => "exam.compose",
        ("PATCH", "/v1/exams/:id") | ("PATCH", "/v1/exams/{id}") => "exam.update",
        ("GET", "/v1/exams/:id/stats") | ("GET", "/v1/exams/{id}/stats") => "exam.stats",
        ("POST", "/v1/sessions") => "session.create",
        ("GET", "/v1/sessions/:id") | ("GET", "/v1/sessions/{id}") => "session.get",
        ("POST", "/v1/sessions/:id/answer") | ("POST", "/v1/sessions/{id}/answer") => {
            "session.answer"
        }
        ("POST", "/v1/sessions/:id/finish") | ("POST", "/v1/sessions/{id}/finish") => {
            "session.finish"
        }
        ("GET", "/v1/attempts/:id") | ("GET", "/v1/attempts/{id}") => "attempt.get",
        ("POST", "/v1/attempts/:id/grade") | ("POST", "/v1/attempts/{id}/grade") => "attempt.grade",
        ("POST", "/v1/messages") => "feedback.send",
        ("POST", "/v1/plans") => "plan.create",
        ("GET", "/v1/plans/:id") | ("GET", "/v1/plans/{id}") => "plan.get",
        ("GET", "/v1/agents/activity") => "agent.activity",
        _ => return format!("{method} {path}"),
    }
    .to_string()
}

pub async fn activity_log_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(req).await;
    let status = response.status().as_u16() as i32;

    if let Some(auth_str) = auth_header
        && let Some(parsed) = parse_bearer_token(&auth_str)
    {
        let pool = state.pool.clone();
        let tool_name = derive_tool_name(&method, &path);
        let path_clone = path.clone();
        let token_id = parsed.id;

        tokio::spawn(async move {
            let result = sqlx::query(
                "SELECT user_id FROM tb_api_tokens WHERE id = $1 AND revoked_at IS NULL",
            )
            .bind(token_id)
            .fetch_optional(&pool)
            .await;

            if let Ok(Some(row)) = result {
                let agent_id: uuid::Uuid = row.get("user_id");
                let _ = sqlx::query(
                    "INSERT INTO tb_activity_log (agent_id, tool_name, method, path, status)
                         VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(agent_id)
                .bind(&tool_name)
                .bind(&method)
                .bind(&path_clone)
                .bind(status)
                .execute(&pool)
                .await;
            }
        });
    }

    response
}
