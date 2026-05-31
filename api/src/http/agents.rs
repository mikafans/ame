//! Agent-surface routes: MCP manifest, OpenAPI export, activity log.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireAnyScope, ScopeOneOf},
    },
    domain::{
        error::{ApiError, FieldError},
        user::Scope,
    },
    http::{AppState, openapi::openapi_yaml},
};

// ── scope guards ──────────────────────────────────────────────────────────────

pub struct AgentReadScopes;
impl ScopeOneOf for AgentReadScopes {
    const SCOPES: &'static [Scope] = &[Scope::AssessmentRead, Scope::Admin];
}

// ── shapes ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub ts: OffsetDateTime,
    pub tool_name: String,
    pub method: String,
    pub path: String,
    pub status: i32,
    pub note: Option<String>,
    pub target_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
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

/// GET /v1/agents/skill.json — agent skill manifest (public).
pub async fn skill_manifest(
    State(_state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let strict = params.get("strict").map(|v| v == "1").unwrap_or(false);
    Json(build_skill_manifest(strict))
}

/// GET /llms.txt — agent entry doc (public, llmstxt.org convention).
pub async fn llms_txt() -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        include_str!("../../llms.txt"),
    )
}

/// GET /v1/agents/openapi.json — serves the OpenAPI snapshot.
pub async fn openapi_json(
    State(_state): State<AppState>,
    _user: RequireAnyScope<AgentReadScopes>,
) -> impl IntoResponse {
    let yaml = openapi_yaml();
    let doc: Value = serde_yaml::from_str(&yaml)
        .unwrap_or_else(|_| json!({"error": "failed to parse openapi snapshot"}));
    Json(doc)
}

/// GET /v1/agents/activity — paginated ActivityLog.
pub async fn activity(
    State(state): State<AppState>,
    user: RequireAnyScope<AgentReadScopes>,
    Query(q): Query<ActivityQuery>,
) -> Result<Json<ActivityResponse>, ApiError> {
    let limit = q.limit.unwrap_or(50).min(200);

    let rows = if let Some(cursor) = q.cursor {
        sqlx::query(
            "SELECT id, ts, tool_name, method, path, status, note, target_id
             FROM tb_activity_log
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
             FROM tb_activity_log
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

fn build_skill_manifest(strict: bool) -> Value {
    let tools: Vec<Value> = vec![
        // Assessment
        tool(
            "assessment.list",
            "List assessments accessible to the caller.",
            json!({"type":"object","properties":{"course":{"type":"string"},"status":{"type":"string"},"mode":{"type":"string","enum":["practice","graded"]}}}),
            "GET",
            "/v1/assessments",
            Some("assessment.read"),
            strict,
        ),
        tool(
            "assessment.get",
            "Get a single assessment with its questions and sections.",
            id_only(),
            "GET",
            "/v1/assessments/{id}",
            Some("assessment.read"),
            strict,
        ),
        tool(
            "assessment.create",
            "Create a new assessment.",
            json!({"type":"object","required":["title","mode"],"properties":{"title":{"type":"string"},"description":{"type":"string"},"mode":{"type":"string","enum":["practice","graded"]},"course":{"type":"string"},"objectives":{"type":"array","items":{"type":"string"}}}}),
            "POST",
            "/v1/assessments",
            Some("assessment.write"),
            strict,
        ),
        tool(
            "assessment.update",
            "Update assessment metadata.",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"},"title":{"type":"string"},"status":{"type":"string"},"visibility":{"type":"string","enum":["public","private"]}}}),
            "PATCH",
            "/v1/assessments/{id}",
            Some("assessment.write"),
            strict,
        ),
        tool(
            "assessment.delete",
            "Delete an assessment.",
            id_only(),
            "DELETE",
            "/v1/assessments/{id}",
            Some("assessment.write"),
            strict,
        ),
        // Questions
        tool(
            "question.list",
            "List questions in the bank.",
            json!({"type":"object","properties":{"tag":{"type":"string"},"status":{"type":"string","enum":["draft","live","archived"]},"limit":{"type":"integer"},"offset":{"type":"integer"}}}),
            "GET",
            "/v1/questions",
            Some("assessment.read"),
            strict,
        ),
        tool(
            "question.create",
            "Batch-create one or more questions in the bank.",
            json!({"type":"object","required":["questions"],"properties":{"questions":{"type":"array","items":{"type":"object","required":["kind","prompt","payload"],"properties":{"kind":{"type":"string","enum":["mc","tf","short","essay","code"]},"prompt":{"type":"string"},"payload":{"type":"object"},"explanation":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"points":{"type":"integer"}}}}}}),
            "POST",
            "/v1/questions",
            Some("assessment.write"),
            strict,
        ),
        tool(
            "question.update",
            "Update a question in the bank.",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"},"prompt":{"type":"string"},"payload":{"type":"object"},"explanation":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"status":{"type":"string","enum":["draft","live","archived"]}}}),
            "PATCH",
            "/v1/questions/{id}",
            Some("assessment.write"),
            strict,
        ),
        tool(
            "question.promote",
            "Promote one draft question to live.",
            id_only(),
            "POST",
            "/v1/questions/{id}/promote",
            Some("assessment.write"),
            strict,
        ),
        tool(
            "stats.user",
            "Read the caller's aggregate learning statistics.",
            json!({"type":"object","properties":{}}),
            "GET",
            "/v1/me/stats",
            Some("stats.read"),
            strict,
        ),
        // Session & attempt
        tool(
            "session.create",
            "Start a practice or graded assessment session.",
            json!({"type":"object","properties":{"assessmentId":{"type":"string"}}}),
            "POST",
            "/v1/sessions",
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
    ];

    json!({
        "schema_version": "v1",
        "name": "ame",
        "description": "Read and write assessments, attempts, and study plans on AME.",
        "auth": { "type": "bearer", "format": "hk_<env>_<id>" },
        "entrypoint": "/llms.txt",
        "run": {
            "endpoint": "/v1/agents/run",
            "method": "POST",
            "body": { "tool": "<tool name>", "params": { "...": "..." } },
            "runnable_tools": [
                "assessment.create",
                "assessment.update",
                "question.create",
                "question.promote"
            ],
            "description": "Execute a composite or write tool.",
        },
        "tools": tools,
    })
}

// ── invoke ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RunBody {
    pub tool: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunResponse {
    pub ok: bool,
    pub tool: String,
    pub result: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn run(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<RunBody>,
) -> Result<Json<RunResponse>, ApiError> {
    let user_id = auth.user.id;
    match body.tool.as_str() {
        "assessment.create" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_create(&state, &auth, body.params).await
        }
        "assessment.update" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_update(&state, &auth, body.params).await
        }
        "question.create" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_question_create(&state, &user_id, body.params).await
        }
        other => Ok(Json(RunResponse {
            ok: false,
            tool: other.to_string(),
            result: Value::Null,
            error: Some(format!("unknown or non-runnable tool: {other}")),
        })),
    }
}

async fn run_assessment_create(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let req: crate::domain::assessment::CreateAssessmentRequest = serde_json::from_value(params)
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid create request: {e}"),
            }])
        })?;
    let res = super::assessments::create_assessment(auth.clone(), State(state.clone()), Json(req))
        .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.create".into(),
        result: serde_json::to_value(res.1.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_assessment_update(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let req: crate::domain::assessment::UpdateAssessmentRequest = serde_json::from_value(params)
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid update request: {e}"),
            }])
        })?;
    let res = super::assessments::patch_assessment(
        auth.clone(),
        State(state.clone()),
        Path(id),
        Json(req),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.update".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_question_create(
    state: &AppState,
    user_id: &Uuid,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    use crate::bank::questions as repo;
    let questions: Vec<repo::QuestionInsert> = params
        .get("questions")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "questions".into(),
                message: format!("invalid questions array: {e}"),
            }])
        })?
        .unwrap_or_default();
    let created = repo::create_questions(&state.pool, *user_id, questions).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "question.create".into(),
        result: json!({ "created": created.len(), "questions": created }),
        error: None,
    }))
}

fn require_scope(auth: &AuthenticatedUser, needed: Scope) -> Result<(), ApiError> {
    if auth.token_scopes.contains(&needed) || auth.token_scopes.contains(&Scope::Admin) {
        Ok(())
    } else {
        Err(ApiError::ScopeRequired(std::borrow::Cow::Borrowed(
            needed.as_str(),
        )))
    }
}

fn parse_id(params: &Value) -> Result<Uuid, ApiError> {
    params
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            ApiError::Validation(vec![FieldError {
                field: "id".into(),
                message: "missing or invalid id".into(),
            }])
        })
}

pub fn public_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/agents/skill.json", get(skill_manifest))
        .route("/v1/agents/openapi.json", get(openapi_json))
        .with_state(state)
}

pub fn logged_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/agents/activity", get(activity))
        .route("/v1/agents/run", post(run))
        .with_state(state)
}
