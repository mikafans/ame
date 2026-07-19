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

use crate::{
    auth::{extractor::AuthenticatedUser, token::parse_bearer_token},
    http::AppState,
};

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
        ("GET", "/v1/agents/activity") => "agent.activity",
        _ => return format!("{method} {path}"),
    }
    .to_string()
}

/// Insert one activity-log row, swallowing any error.
///
/// `agent_id` MUST reference `tb_agents(id)` — the FK rejects any other id, so
/// only agent callers should be passed here (human ids silently fail the
/// insert). Errors are swallowed: a DB hiccup must never break the request
/// path. Callers run this fire-and-forget (`tokio::spawn`). Shared by the path
/// middleware and the run-door (`http::agents::run`).
pub(crate) async fn insert_activity_log(
    pool: &sqlx::PgPool,
    agent_id: uuid::Uuid,
    tool_name: &str,
    method: &str,
    path: &str,
    status: i32,
) {
    let _ = sqlx::query(
        "INSERT INTO tb_activity_log (actor_id, tool_name, method, path, status)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(agent_id)
    .bind(tool_name)
    .bind(method)
    .bind(path)
    .bind(status)
    .execute(pool)
    .await;
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

    // The run-door (POST /v1/agents/run) records its own activity entry with
    // the resolved inner tool name (see http::agents::run). Skip it here to
    // avoid a duplicate, uninformative "POST /v1/agents/run" row.
    if path == "/v1/agents/run" {
        return next.run(req).await;
    }

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let auth_extension = req.extensions().get::<AuthenticatedUser>().cloned();

    let response = next.run(req).await;
    let status = response.status().as_u16() as i32;

    // Only agent callers are logged — `tb_activity_log.agent_id` references
    // `tb_agents(id)`, so a human id would fail the FK. When the auth extractor
    // already ran we know the role; otherwise we resolve the token's `agent_id`
    // (NULL for human tokens, which we then skip).
    if let Some(auth) = auth_extension {
        if auth.user.role == crate::domain::user::Role::Agent {
            let pool = state.pool.clone();
            let tool_name = derive_tool_name(&method, &path);
            let agent_id = auth.user.id;
            tokio::spawn(async move {
                insert_activity_log(&pool, agent_id, &tool_name, &method, &path, status).await;
            });
        }
    } else if let Some(auth_str) = auth_header
        && let Some(parsed) = parse_bearer_token(&auth_str)
    {
        let pool = state.pool.clone();
        let tool_name = derive_tool_name(&method, &path);
        let token_id = parsed.id;

        tokio::spawn(async move {
            let result = sqlx::query(
                "SELECT agent_id FROM tb_api_tokens WHERE id = $1 AND revoked_at IS NULL",
            )
            .bind(token_id)
            .fetch_optional(&pool)
            .await;

            if let Ok(Some(row)) = result
                && let Some(agent_id) = row.get::<Option<uuid::Uuid>, _>("agent_id")
            {
                insert_activity_log(&pool, agent_id, &tool_name, &method, &path, status).await;
            }
        });
    }

    response
}
