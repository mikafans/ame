//! Agent-surface routes: register, MCP manifest, OpenAPI export, activity log.

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::scope::{RequireAnyScope, ScopeOneOf},
    domain::{
        error::{ApiError, FieldError},
        user::Scope,
    },
    http::{AppState, me::generate_secret, openapi::openapi_yaml},
};

// ── scope guards ──────────────────────────────────────────────────────────────

pub struct AgentReadScopes;
impl ScopeOneOf for AgentReadScopes {
    const SCOPES: &'static [Scope] = &[Scope::QuizRead, Scope::Admin];
}

// ── shapes ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBody {
    pub label: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    pub api_key: String,
    pub user_id: Uuid,
    pub openapi_url: String,
    pub mcp_manifest_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: Uuid,
    pub ts: OffsetDateTime,
    pub tool_name: String,
    pub method: String,
    pub path: String,
    pub status: i32,
    pub note: Option<String>,
    pub target_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResponse {
    pub items: Vec<ActivityEntry>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

// ── handlers ──────────────────────────────────────────────────────────────────

/// POST /v1/agents/register — unauthenticated bootstrap for agent users.
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.scopes.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "scopes".into(),
            message: "must include at least one scope".into(),
        }]));
    }
    if body.scopes.contains(&"admin".to_string()) {
        return Err(ApiError::Validation(vec![FieldError {
            field: "scopes".into(),
            message: "admin scope cannot be requested via /agents/register".into(),
        }]));
    }
    for s in &body.scopes {
        if s.parse::<Scope>().is_err() {
            return Err(ApiError::Validation(vec![FieldError {
                field: "scopes".into(),
                message: format!("unknown scope: {s}"),
            }]));
        }
    }

    let label = body
        .label
        .filter(|l| !l.trim().is_empty())
        .unwrap_or_else(|| "agent".to_string());
    let user_id = Uuid::now_v7();
    let display_name = format!("agent:{}", &user_id.to_string()[..8]);

    sqlx::query("INSERT INTO users (id, display_name, role) VALUES ($1, $2, 'agent')")
        .bind(user_id)
        .bind(&display_name)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let token_id = Uuid::now_v7();
    let secret = generate_secret();
    let hash = crate::http::me::hash_secret(&secret)?;

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind(&label)
    .bind(&hash)
    .bind(&body.scopes)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            api_key: format!("{token_id}_{secret}"),
            user_id,
            openapi_url: "/v1/agents/openapi.json".to_string(),
            mcp_manifest_url: "/v1/agents/mcp.json".to_string(),
        }),
    ))
}

/// GET /v1/agents/mcp.json — MCP manifest (public).
pub async fn mcp_manifest(
    State(_state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let strict = params.get("strict").map(|v| v == "1").unwrap_or(false);
    Json(build_mcp_manifest(strict))
}

/// GET /v1/agents/openapi.json — serves the OpenAPI snapshot (requires quiz.read).
pub async fn openapi_json(
    State(_state): State<AppState>,
    _user: RequireAnyScope<AgentReadScopes>,
) -> impl IntoResponse {
    let yaml = openapi_yaml();
    // Convert yaml snapshot → JSON for this endpoint
    let doc: Value = serde_yaml::from_str(&yaml)
        .unwrap_or_else(|_| json!({"error": "failed to parse openapi snapshot"}));
    Json(doc)
}

/// GET /v1/agents/activity — paginated ActivityLog (requires quiz.read).
pub async fn activity(
    State(state): State<AppState>,
    user: RequireAnyScope<AgentReadScopes>,
    Query(q): Query<ActivityQuery>,
) -> Result<Json<ActivityResponse>, ApiError> {
    let limit = q.limit.unwrap_or(50).min(200);

    let rows = if let Some(cursor) = q.cursor {
        sqlx::query(
            "SELECT id, ts, tool_name, method, path, status, note, target_id
             FROM activity_log
             WHERE agent_id = $1 AND id < $2
             ORDER BY id DESC
             LIMIT $3",
        )
        .bind(user.0.user.id)
        .bind(cursor)
        .bind(limit + 1)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query(
            "SELECT id, ts, tool_name, method, path, status, note, target_id
             FROM activity_log
             WHERE agent_id = $1
             ORDER BY id DESC
             LIMIT $2",
        )
        .bind(user.0.user.id)
        .bind(limit + 1)
        .fetch_all(&state.pool)
        .await
    }
    .map_err(|e| ApiError::Internal(e.into()))?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<ActivityEntry> = rows
        .into_iter()
        .take(limit as usize)
        .map(|row| ActivityEntry {
            id: row.get("id"),
            ts: row.get("ts"),
            tool_name: row.get("tool_name"),
            method: row.get("method"),
            path: row.get("path"),
            status: row.get("status"),
            note: row.get("note"),
            target_id: row.get("target_id"),
        })
        .collect();

    let next_cursor = if has_more {
        items.last().map(|e| e.id)
    } else {
        None
    };

    Ok(Json(ActivityResponse { items, next_cursor }))
}

// ── MCP manifest builder ──────────────────────────────────────────────────────

fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    method: &str,
    path: &str,
    scope: Option<&str>,
    strict: bool,
) -> Value {
    let mut t = json!({
        "name": name,
        "description": description,
        "input_schema": input_schema,
    });
    if !strict {
        t["method"] = json!(method);
        t["path"] = json!(path);
        t["scope"] = match scope {
            Some(s) => json!(s),
            None => Value::Null,
        };
    }
    t
}

fn id_only() -> Value {
    json!({
        "type": "object",
        "required": ["id"],
        "properties": { "id": { "type": "string" } }
    })
}

fn build_mcp_manifest(strict: bool) -> Value {
    let tools: Vec<Value> = vec![
        // Quiz
        tool(
            "quiz.list",
            "List quizzes accessible to the caller.",
            json!({"type":"object","properties":{"course":{"type":"string"},"tag":{"type":"string"},"status":{"type":"string"}}}),
            "GET",
            "/v1/quizzes",
            Some("quiz.read"),
            strict,
        ),
        tool(
            "quiz.get",
            "Get a single quiz with its questions and rubric.",
            id_only(),
            "GET",
            "/v1/quizzes/{id}",
            Some("quiz.read"),
            strict,
        ),
        tool(
            "quiz.import",
            "Create a new quiz from JSON or Markdown source.",
            json!({"type":"object","required":["source","format"],"properties":{"source":{"type":"string"},"format":{"type":"string","enum":["json","md"]},"courseId":{"type":"string"},"objectives":{"type":"array","items":{"type":"string"}}}}),
            "POST",
            "/v1/quizzes",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "quiz.generate",
            "Generate a quiz from source text using AI.",
            json!({"type":"object","required":["source","questionCount"],"properties":{"source":{"type":"string"},"questionCount":{"type":"integer"},"types":{"type":"array","items":{"type":"string"}},"difficulty":{"type":"string"}}}),
            "POST",
            "/v1/quizzes/generate",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "quiz.update",
            "Update quiz metadata (locked once active).",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"},"title":{"type":"string"},"status":{"type":"string"}}}),
            "PATCH",
            "/v1/quizzes/{id}",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "quiz.delete",
            "Soft-delete a quiz (sets status=archived).",
            id_only(),
            "DELETE",
            "/v1/quizzes/{id}",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "stats.cohort",
            "Get aggregate stats for a quiz.",
            id_only(),
            "GET",
            "/v1/quizzes/{id}/stats",
            Some("stats.read"),
            strict,
        ),
        // Exam
        tool(
            "exam.list",
            "List exams.",
            json!({"type":"object","properties":{"course":{"type":"string"},"status":{"type":"string"}}}),
            "GET",
            "/v1/exams",
            Some("quiz.read"),
            strict,
        ),
        tool(
            "exam.get",
            "Get exam detail including sections and composition trace.",
            id_only(),
            "GET",
            "/v1/exams/{id}",
            Some("quiz.read"),
            strict,
        ),
        tool(
            "exam.compose",
            "Compose a new exam from quiz sections.",
            json!({"type":"object","required":["title","sections"],"properties":{"title":{"type":"string"},"courseId":{"type":"string"},"sections":{"type":"array"},"duration":{"type":"integer"},"window":{"type":"object"}}}),
            "POST",
            "/v1/exams",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "exam.update",
            "Update exam window or status.",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"},"status":{"type":"string"}}}),
            "PATCH",
            "/v1/exams/{id}",
            Some("quiz.write"),
            strict,
        ),
        tool(
            "exam.stats",
            "Get exam pass rate, section averages, and timing percentiles.",
            id_only(),
            "GET",
            "/v1/exams/{id}/stats",
            Some("stats.read"),
            strict,
        ),
        // Session & attempt
        tool(
            "session.create",
            "Start a practice, quiz, or exam session.",
            json!({"type":"object","properties":{"quizId":{"type":"string"},"examId":{"type":"string"}}}),
            "POST",
            "/v1/sessions",
            Some("attempt.write"),
            strict,
        ),
        tool(
            "session.get",
            "Get session state for resume.",
            id_only(),
            "GET",
            "/v1/sessions/{id}",
            Some("attempt.read"),
            strict,
        ),
        tool(
            "session.answer",
            "Submit an answer for a question.",
            json!({"type":"object","required":["questionId","response"],"properties":{"questionId":{"type":"string"},"response":{},"timeToAnswerMs":{"type":"integer"}}}),
            "POST",
            "/v1/sessions/{id}/answer",
            Some("attempt.write"),
            strict,
        ),
        tool(
            "session.finish",
            "Finish a session and compute results.",
            id_only(),
            "POST",
            "/v1/sessions/{id}/finish",
            Some("attempt.write"),
            strict,
        ),
        tool(
            "attempt.get",
            "Get attempt detail with answers and rubric.",
            id_only(),
            "GET",
            "/v1/attempts/{id}",
            Some("attempt.read"),
            strict,
        ),
        tool(
            "attempt.grade",
            "Manual grade override or re-grade.",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"},"score":{"type":"number"}}}),
            "POST",
            "/v1/attempts/{id}/grade",
            Some("attempt.write"),
            strict,
        ),
        // Agent surface
        tool(
            "agents.register",
            "Register a new agent user and receive an API key.",
            json!({"type":"object","required":["scopes"],"properties":{"label":{"type":"string"},"scopes":{"type":"array","items":{"type":"string"}}}}),
            "POST",
            "/v1/agents/register",
            None,
            strict,
        ),
        tool(
            "agent.activity",
            "Get paginated activity log for the calling agent.",
            json!({"type":"object","properties":{"cursor":{"type":"string"},"limit":{"type":"integer"}}}),
            "GET",
            "/v1/agents/activity",
            Some("quiz.read"),
            strict,
        ),
        // Feedback & plans
        tool(
            "feedback.send",
            "Send an in-app or email message to a learner.",
            json!({"type":"object","required":["userId","body"],"properties":{"userId":{"type":"string"},"channel":{"type":"string","enum":["in_app","email"]},"body":{"type":"string"},"linkQuizId":{"type":"string"}}}),
            "POST",
            "/v1/messages",
            Some("feedback.write"),
            strict,
        ),
        tool(
            "plan.create",
            "Generate a study plan for a user.",
            json!({"type":"object","required":["goal"],"properties":{"goal":{"type":"string"},"lookbackDays":{"type":"integer"}}}),
            "POST",
            "/v1/plans",
            Some("plan.write"),
            strict,
        ),
        tool(
            "plan.get",
            "Get a previously generated study plan.",
            id_only(),
            "GET",
            "/v1/plans/{id}",
            Some("plan.read"),
            strict,
        ),
        // Share
        tool(
            "share.create",
            "Mint a public or cohort-scoped read-only share link.",
            json!({"type":"object","required":["kind","id"],"properties":{"kind":{"type":"string","enum":["quiz","exam","item"]},"id":{"type":"string"},"visibility":{"type":"string","enum":["public","cohort"],"default":"public"},"includeExplanation":{"type":"boolean","default":false},"includeScore":{"type":"boolean","default":false},"includeAttribution":{"type":"boolean","default":true}}}),
            "POST",
            "/v1/shares",
            Some("quiz.read"),
            strict,
        ),
        tool(
            "share.get",
            "Resolve a share link to its stripped read-only view. No auth required.",
            id_only(),
            "GET",
            "/v1/shares/{id}",
            None,
            strict,
        ),
        tool(
            "share.revoke",
            "Revoke a previously minted share link. Caller must be the creator or admin.",
            id_only(),
            "DELETE",
            "/v1/shares/{id}",
            Some("quiz.read"),
            strict,
        ),
    ];

    json!({
        "schema_version": "v1",
        "name": "harus",
        "description": "Read and write quizzes, exams, attempts, and study plans on Harus.",
        "auth": { "type": "bearer", "format": "hk_<env>_<id>" },
        "tools": tools,
    })
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/agents/register", post(register))
        .route("/v1/agents/mcp.json", get(mcp_manifest))
        .route("/v1/agents/openapi.json", get(openapi_json))
        .route("/v1/agents/activity", get(activity))
        .with_state(state)
}
