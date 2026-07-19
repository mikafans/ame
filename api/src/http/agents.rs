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
        scope::{RequireAnyScope, RequireScope, ScopeOneOf},
    },
    domain::{
        error::{ApiError, FieldError},
        user::Scope,
    },
    http::AppState,
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
    pub agent_id: Uuid,
    pub agent_name: Option<String>,
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

/// GET /skill.json — agent skill manifest (public).
pub async fn skill_manifest(State(_state): State<AppState>) -> Json<Value> {
    Json(build_skill_manifest())
}

/// GET /llms.txt — agent entry doc (public, llmstxt.org convention).
pub async fn llms_txt() -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        include_str!("../../../docs/public/llms.txt"),
    )
}

async fn get_accessible_accounts(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
) -> Result<Vec<Uuid>, ApiError> {
    let rows = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM tb_agents WHERE owner_user_id = $1 OR id = $1",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// GET /v1/agents/activity — paginated ActivityLog.
pub async fn activity(
    State(state): State<AppState>,
    user: RequireAnyScope<AgentReadScopes>,
    Query(q): Query<ActivityQuery>,
) -> Result<Json<ActivityResponse>, ApiError> {
    let limit = q.limit.unwrap_or(50).min(50);
    let accounts = get_accessible_accounts(&state.pool, user.0.user.id).await?;

    let rows = if let Some(cursor) = q.cursor {
        sqlx::query(
            "SELECT a.id, a.ts, a.actor_id AS agent_id, i.label as agent_name, a.tool_name, a.method, a.path, a.status, a.note, a.target_id
             FROM tb_activity_log a
             LEFT JOIN tb_identities i ON a.actor_id = i.id
             WHERE a.actor_id = ANY($1) AND a.id < $2
             ORDER BY a.id DESC
             LIMIT $3",
        )
        .bind(&accounts)
        .bind(cursor)
        .bind(limit + 1)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query(
            "SELECT a.id, a.ts, a.actor_id AS agent_id, i.label as agent_name, a.tool_name, a.method, a.path, a.status, a.note, a.target_id
             FROM tb_activity_log a
             LEFT JOIN tb_identities i ON a.actor_id = i.id
             WHERE a.actor_id = ANY($1)
             ORDER BY a.id DESC
             LIMIT $2",
        )
        .bind(&accounts)
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
            agent_id: row.get("agent_id"),
            agent_name: row.get("agent_name"),
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
) -> Value {
    let mut t = json!({
        "name": name,
        "description": description,
        "input_schema": input_schema,
    });
    t["method"] = json!(method);
    t["path"] = json!(path);
    t["scope"] = match scope {
        Some(s) => json!(s),
        None => Value::Null,
    };
    t
}

fn id_only() -> Value {
    json!({
        "type": "object",
        "required": ["id"],
        "properties": { "id": { "type": "string" } }
    })
}

pub fn build_skill_manifest() -> Value {
    let tools: Vec<Value> = vec![
        // Assessment
        tool(
            "assessment.list",
            "List assessments accessible to the caller.",
            json!({"type":"object","properties":{"course":{"type":"string"},"status":{"type":"string"},"mode":{"type":"string","enum":["practice","graded"]}}}),
            "GET",
            "/v1/assessments",
            Some("assessment.read"),
        ),
        tool(
            "assessment.get",
            "Get assessment details.",
            id_only(),
            "GET",
            "/v1/assessments/{id}",
            Some("assessment.read"),
        ),
        tool(
            "assessment.archive",
            "Archive an assessment.",
            id_only(),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "assessment.publish",
            "Publish an assessment to make it active and promote draft questions to live.",
            id_only(),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "assessment.create",
            "Create a new assessment, optionally importing questions.",
            json!({
                "type":"object",
                "required":["title","mode","method"],
                "properties":{
                    "title":{"type":"string"},
                    "description":{"type":"string"},
                    "mode":{"type":"string","enum":["practice","graded"]},
                    "course":{"type":"string"},
                    "objectives":{"type":"array","items":{"type":"string"}},
                    "method":{"type":"string","enum":["manual","agent"]},
                    "status":{"type":"string","enum":["draft","active","archived"]},
                    "questions":{
                        "type":"array",
                        "items":{
                            "type":"object",
                            "required":["kind","prompt","payload"],
                            "properties":{
                                "kind":{"type":"string","enum":["mc","tf","short","essay","code"]},
                                "prompt":{"type":"string"},
                                "payload":{"type":"object"},
                                "explanation":{"type":"string"},
                                "tags":{"type":"array","items":{"type":"string"}},
                                "points":{"type":"integer"}
                            }
                        }
                    }
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "assessment.batchCreate",
            "Batch-create multiple assessments in one call.",
            json!({
                "type":"object",
                "required":["items"],
                "properties":{
                    "items":{
                        "type":"array",
                        "items":{
                            "type":"object",
                            "required":["title","mode","method"],
                            "properties":{
                                "title":{"type":"string"},
                                "description":{"type":"string"},
                                "mode":{"type":"string","enum":["practice","graded"]},
                                "course":{"type":"string"},
                                "objectives":{"type":"array","items":{"type":"string"}},
                                "method":{"type":"string","enum":["manual","agent"]},
                                "status":{"type":"string","enum":["draft","active","archived"]},
                                "questions":{
                                    "type":"array",
                                    "items":{
                                        "type":"object",
                                        "required":["kind","prompt","payload"],
                                        "properties":{
                                            "kind":{"type":"string","enum":["mc","tf","short","essay","code"]},
                                            "prompt":{"type":"string"},
                                            "payload":{"type":"object"},
                                            "explanation":{"type":"string"},
                                            "tags":{"type":"array","items":{"type":"string"}},
                                            "points":{"type":"integer"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "assessment.update",
            "Update assessment metadata.",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string"},"title":{"type":"string"},"status":{"type":"string"}}}),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "assessment.addQuestion",
            "Attach a bank question to an assessment or create a new question inline.",
            json!({
                "type":"object",
                "required":["id"],
                "properties":{
                    "id":{"type":"string","format":"uuid","description":"assessment id"},
                    "questionId":{"type":"string","format":"uuid","description":"existing bank question id (optional)"},
                    "kind":{"type":"string","enum":["mc","tf","short","essay","code"],"description":"required if questionId absent"},
                    "prompt":{"type":"string","description":"question text"},
                    "pointsOverride":{"type":"integer","description":"override default points"},
                    "sectionId":{"type":"string","format":"uuid","description":"target section (defaults to first)"}
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        // Questions
        tool(
            "question.list",
            "List questions in the bank.",
            json!({"type":"object","properties":{"tag":{"type":"string"},"status":{"type":"string","enum":["draft","live","archived"]},"limit":{"type":"integer"},"offset":{"type":"integer"},"after":{"type":"string"},"cursor":{"type":"string"},"assessmentId":{"type":"string","format":"uuid"}}}),
            "GET",
            "/v1/questions",
            Some("assessment.read"),
        ),
        tool(
            "question.create",
            "Batch-create one or more questions in the bank.",
            json!({"type":"object","required":["questions"],"properties":{"questions":{"type":"array","items":{"type":"object","required":["kind","prompt","payload"],"properties":{"kind":{"type":"string","enum":["mc","tf","short","essay","code"]},"prompt":{"type":"string"},"payload":{"type":"object"},"explanation":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"points":{"type":"integer"},"status":{"type":"string","enum":["draft","live","archived"]}}}}}}),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "question.promote",
            "Promote one draft question to live.",
            id_only(),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "question.update",
            "Update a question in the bank.",
            json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string", "format": "uuid" },
                    "prompt": { "type": "string" },
                    "explanation": { "type": "string" },
                    "points": { "type": "integer" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "payload": { "type": "object" }
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "question.deepen",
            "Retrieve deepen/review details for a question (tags, related questions, references, study notes). Returns 403 Forbidden if there is an active session for this question.",
            json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string", "format": "uuid", "description": "Question ID" },
                    "exclude": { "type": "string", "description": "Comma-separated list of question IDs to exclude" }
                }
            }),
            "GET",
            "/v1/questions/{id}/deepen",
            Some("assessment.read"),
        ),
        tool(
            "deepDive.request",
            "Request a durable Deep Dive explanation for an owned question.",
            json!({
                "type": "object",
                "required": ["questionId"],
                "properties": {
                    "questionId": { "type": "string", "format": "uuid" },
                    "sourceSessionId": { "type": "string", "format": "uuid" },
                    "sourceAttemptId": { "type": "string", "format": "uuid" },
                    "reason": { "type": "string" }
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("attempt.write"),
        ),
        tool(
            "deepDive.list",
            "List the owner's Deep Dive requests.",
            json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string", "enum": ["requested", "drafting", "published", "needs_revision", "archived"] }
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("assessment.read"),
        ),
        tool(
            "deepDive.publish",
            "Publish authored Markdown for an owned Deep Dive request.",
            json!({
                "type": "object",
                "required": ["id", "bodyMarkdown"],
                "properties": {
                    "id": { "type": "string", "format": "uuid" },
                    "bodyMarkdown": { "type": "string" },
                    "status": { "type": "string", "enum": ["published", "drafting", "needs_revision", "requested"] },
                    "category": { "type": "string" }
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        tool(
            "assessment.stats",
            "Get per-assessment performance stats (avg, median, distribution, per-question metrics).",
            id_only(),
            "GET",
            "/v1/assessments/{id}/stats",
            Some("stats.read"),
        ),
        tool(
            "activity.list",
            "List agent activity log.",
            json!({"type":"object","properties":{"cursor":{"type":"string","format":"uuid"},"limit":{"type":"integer"}}}),
            "GET",
            "/v1/agents/activity",
            Some("assessment.read"),
        ),
        tool(
            "learning.journey.get",
            "Read the owner's intent, objectives, activities, persisted evidence-backed recommendation, and source attribution for a learning journey.",
            id_only(),
            "POST",
            "/v1/agents/run",
            Some("plan.read"),
        ),
        tool(
            "stats.user",
            "Read the caller's aggregate learning statistics.",
            json!({"type":"object","properties":{}}),
            "GET",
            "/v1/me/stats",
            Some("stats.read"),
        ),
        tool(
            "attempt.list",
            "List the owner's per-attempt history, optionally filtered to one session.",
            json!({"type":"object","properties":{"limit":{"type":"integer"},"offset":{"type":"integer"},"sessionId":{"type":"string","format":"uuid"}}}),
            "GET",
            "/v1/me/attempts",
            Some("attempt.read"),
        ),
        // Attempt grading
        tool(
            "attempt.grade",
            "Grade a pending essay or code attempt.",
            json!({
                "type": "object",
                "required": ["id", "score"],
                "properties": {
                    "id": { "type": "string", "format": "uuid" },
                    "score": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                    "notes": { "type": "string" }
                }
            }),
            "POST",
            "/v1/agents/run",
            Some("attempt.write"),
        ),
        tool(
            "assessment.delete",
            "Delete an assessment.",
            id_only(),
            "POST",
            "/v1/agents/run",
            Some("assessment.write"),
        ),
        // Agent self — run-only tools (POST /v1/agents/run). These operate on the
        // calling agent's own profile/plan and are available to any agent token.
        tool(
            "profile.get",
            "Get the agent's own config (label, focus tags, goal, memory) plus the owner's per-tag ELO ratings.",
            json!({"type":"object","properties":{}}),
            "POST",
            "/v1/agents/run",
            None,
        ),
        tool(
            "memory.set",
            "Replace the agent's memory blob wholesale.",
            json!({"type":"object","required":["memory"],"properties":{"memory":{"type":"object"}}}),
            "POST",
            "/v1/agents/run",
            None,
        ),
        tool(
            "memory.append",
            "Shallow-merge the given keys into the agent's memory blob.",
            json!({"type":"object","required":["append"],"properties":{"append":{"type":"object"}}}),
            "POST",
            "/v1/agents/run",
            None,
        ),
        tool(
            "target.set",
            "Update the agent's current goal and/or next target.",
            json!({"type":"object","properties":{"currentGoal":{"type":"string"},"nextTarget":{"type":"string"}}}),
            "POST",
            "/v1/agents/run",
            None,
        ),
    ];

    json!({
        "schema_version": "v1",
        "name": "ame",
        "description": "ame — study, sweetened. Read and write assessments, attempts, and study plans.",
        "auth": { "type": "bearer", "format": "<id>_<secret>" },
        "entrypoint": "/llms.txt",
        "run": {
            "endpoint": "/v1/agents/run",
            "method": "POST",
            "body": { "tool": "<tool name>", "params": { "...": "..." } },
            "runnable_tools": [
                "assessment.list",
                "assessment.get",
                "assessment.stats",
                "question.list",
                "activity.list",
                "learning.journey.get",
                "stats.user",
                "attempt.list",
                "assessment.create",
                "assessment.batchCreate",
                "assessment.update",
                "assessment.addQuestion",
                "assessment.delete",
                "assessment.archive",
                "assessment.publish",
                "question.create",
                "question.promote",
                "question.update",
                "deepDive.request",
                "deepDive.list",
                "deepDive.publish",
                "attempt.grade",
                "profile.get",
                "memory.set",
                "memory.append",
                "target.set"
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
    let tool = body.tool.clone();
    // The path-based activity middleware records every run-door call as
    // "POST /v1/agents/run" — it can't see which inner tool ran. Capture the
    // context to log the real tool name here so the activity feed and stats
    // (e.g. attempt.grade counts) stay meaningful. Only agent callers are
    // logged: tb_activity_log.agent_id references tb_agents, so a human
    // caller's id would violate the FK. The middleware skips /v1/agents/run to
    // avoid double-logging.
    let log_ctx = (auth.user.role == crate::domain::user::Role::Agent)
        .then(|| (state.pool.clone(), auth.user.id));

    let result = dispatch_run(state, auth, body).await;

    if let Some((pool, agent_id)) = log_ctx {
        let status = match &result {
            Ok(_) => 200i32,
            Err(e) => e.status_code().as_u16() as i32,
        };
        tokio::spawn(async move {
            crate::http::activity::insert_activity_log(
                &pool,
                agent_id,
                &tool,
                "POST",
                "/v1/agents/run",
                status,
            )
            .await;
        });
    }

    result
}

async fn dispatch_run(
    state: AppState,
    auth: AuthenticatedUser,
    body: RunBody,
) -> Result<Json<RunResponse>, ApiError> {
    let user_id = auth.user.id;
    match body.tool.as_str() {
        "assessment.create" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_create(&state, &auth, body.params).await
        }
        "assessment.batchCreate" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_batch_create(&state, &auth, body.params).await
        }
        "assessment.update" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_update(&state, &auth, body.params).await
        }
        "assessment.addQuestion" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_add_question(&state, &auth, body.params).await
        }
        "question.create" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_question_create(&state, &auth, body.params).await
        }
        "question.promote" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_question_promote(&state, &auth, body.params).await
        }
        "question.update" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_question_update(&state, &auth, body.params).await
        }
        "attempt.grade" => {
            require_scope(&auth, Scope::AttemptWrite)?;
            run_attempt_grade(&state, &auth, body.params).await
        }
        "assessment.list" => {
            require_scope(&auth, Scope::AssessmentRead)?;
            run_assessment_list(&state, &auth, body.params).await
        }
        "assessment.get" => {
            require_scope(&auth, Scope::AssessmentRead)?;
            run_assessment_get(&state, &auth, body.params).await
        }
        "assessment.delete" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_delete(&state, &auth, body.params).await
        }
        "assessment.archive" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_archive(&state, &auth, body.params).await
        }
        "assessment.publish" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_assessment_publish(&state, &auth, body.params).await
        }
        "question.list" => {
            require_scope(&auth, Scope::AssessmentRead)?;
            run_question_list(&state, &auth, body.params).await
        }
        "question.deepen" => {
            require_scope(&auth, Scope::AssessmentRead)?;
            run_question_deepen(&state, &auth, body.params).await
        }
        "deepDive.request" => {
            require_scope(&auth, Scope::AttemptWrite)?;
            run_deep_dive_request(&state, &auth, body.params).await
        }
        "deepDive.list" => {
            require_scope(&auth, Scope::AssessmentRead)?;
            run_deep_dive_list(&state, &auth, body.params).await
        }
        "deepDive.publish" => {
            require_scope(&auth, Scope::AssessmentWrite)?;
            run_deep_dive_publish(&state, &auth, body.params).await
        }
        "assessment.stats" => {
            require_scope(&auth, Scope::StatsRead)?;
            run_assessment_stats(&state, &auth, body.params).await
        }
        "activity.list" => {
            require_scope(&auth, Scope::AssessmentRead)?;
            run_activity_list(&state, &auth, body.params).await
        }
        "learning.journey.get" => {
            require_scope(&auth, Scope::PlanRead)?;
            run_learning_journey_get(&state, &auth, body.params).await
        }
        "stats.user" => {
            require_scope(&auth, Scope::StatsRead)?;
            run_stats_user(&state, &auth, body.params).await
        }
        "attempt.list" => {
            require_scope(&auth, Scope::AttemptRead)?;
            run_attempt_list(&state, &auth, body.params).await
        }
        "profile.get" => run_profile_get(&state, &auth).await,
        "memory.set" => run_memory_set(&state, &user_id, body.params).await,
        "memory.append" => run_memory_append(&state, &user_id, body.params).await,
        "target.set" => run_target_set(&state, &user_id, body.params).await,
        other => Err(ApiError::UnknownTool(other.to_string())),
    }
}

async fn run_learning_journey_get(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let journey_id = parse_id(&params)?;
    let response =
        super::learning::get_journey(State(state.clone()), auth.clone(), Path(journey_id)).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "learning.journey.get".into(),
        result: serde_json::to_value(response.0).unwrap_or(Value::Null),
        error: None,
    }))
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

async fn run_assessment_delete(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    super::assessments::delete_assessment(auth.clone(), State(state.clone()), Path(id)).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.delete".into(),
        result: Value::Null,
        error: None,
    }))
}

async fn run_assessment_archive(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let req = crate::domain::assessment::UpdateAssessmentRequest {
        title: None,
        description: None,
        status: Some(crate::domain::assessment::AssessmentStatus::Archived),
        objectives: None,
    };
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conn = &mut *acquired;
    crate::http::db::set_rls_guc(conn, auth.owner_id, false).await?;
    let db = crate::http::db::DbConn(acquired);

    let res = super::assessments::patch_assessment(
        auth.clone(),
        State(state.clone()),
        db,
        Path(id),
        Json(req),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.archive".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_assessment_publish(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let req = crate::domain::assessment::UpdateAssessmentRequest {
        title: None,
        description: None,
        status: Some(crate::domain::assessment::AssessmentStatus::Active),
        objectives: None,
    };
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conn = &mut *acquired;
    crate::http::db::set_rls_guc(conn, auth.owner_id, false).await?;
    let db = crate::http::db::DbConn(acquired);

    let res = super::assessments::patch_assessment(
        auth.clone(),
        State(state.clone()),
        db,
        Path(id),
        Json(req),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.publish".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_assessment_batch_create(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    use sqlx::QueryBuilder;
    use uuid::Uuid;

    #[derive(Debug, Deserialize)]
    struct BatchParams {
        items: Vec<crate::domain::assessment::CreateAssessmentRequest>,
    }

    let batch: BatchParams = serde_json::from_value(params).map_err(|e| {
        ApiError::Validation(vec![FieldError {
            field: "params".into(),
            message: format!("invalid batch request: {e}"),
        }])
    })?;

    if batch.items.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "items".into(),
            message: "items array must not be empty".into(),
        }]));
    }

    let (burst, rate) = (
        state.config.ratelimit.free.burst,
        state.config.ratelimit.free.rate as f64,
    );
    let max_batch = state.config.batch.free;
    let n = batch.items.len();
    if n > max_batch {
        return Err(ApiError::Validation(vec![FieldError {
            field: "items".into(),
            message: format!("batch size {n} exceeds the local maximum of {max_batch}"),
        }]));
    }

    let write_cost = state.config.ratelimit.cost.write;
    let extra = (n as u32).saturating_sub(write_cost);
    let limit_key = format!("ame:limiter:owner:{}", auth.owner_id);
    if extra > 0
        && !state
            .limiter
            .try_consume(&limit_key, burst, rate, extra)
            .await
    {
        return Err(ApiError::TooManyRequests);
    }

    let total_assessments = batch.items.len() as i64;
    let total_questions: i64 = batch
        .items
        .iter()
        .map(|item| item.questions.len() as i64)
        .sum();

    crate::http::quota::check_quota(
        &state.pool,
        Some(&state.valkey),
        &state.config,
        auth.owner_id,
        crate::http::quota::QuotaKind::Assessment,
        total_assessments,
    )
    .await?;

    if total_questions > 0 {
        crate::http::quota::check_quota(
            &state.pool,
            Some(&state.valkey),
            &state.config,
            auth.owner_id,
            crate::http::quota::QuotaKind::Question,
            total_questions,
        )
        .await?;
    }

    // Validate all items upfront and collect precomputed values
    #[derive(Debug)]
    struct ValidatedItem {
        title: String,
        description: Option<String>,
        mode: String,
        status: String,
        objectives: Vec<String>,
        course: Option<String>,
        duration_min: Option<i32>,
        time_limit_seconds: Option<i32>,
        passing_points: Option<i32>,
        show_results_during: bool,
        affects_rating: bool,
        method: String,
        questions: Vec<crate::domain::assessment::QuestionImport>,
        assessment_id: Uuid,
        section_id: Uuid,
        total_points: i32,
        question_count: i64,
    }

    let mut validated = Vec::new();
    for (idx, item) in batch.items.into_iter().enumerate() {
        if item.title.trim().is_empty() {
            return Err(ApiError::Validation(vec![FieldError {
                field: "title".into(),
                message: format!("batch item {}: title must not be empty", idx),
            }]));
        }

        let status = if let Some(ref s) = item.status {
            super::assessments::parse_status(s).map_err(|e| {
                ApiError::Validation(vec![FieldError {
                    field: "status".into(),
                    message: format!("batch item {}: {}", idx, e),
                }])
            })?;
            s.clone()
        } else {
            "draft".to_string()
        };

        let question_count = item.questions.len() as i64;
        let total_points: i32 = item.questions.iter().map(|q| q.points.unwrap_or(1)).sum();

        validated.push(ValidatedItem {
            title: item.title,
            description: item.description,
            mode: item.mode.to_string(),
            status,
            objectives: item.objectives,
            course: item.course,
            duration_min: item.duration_min,
            time_limit_seconds: item.time_limit_seconds,
            passing_points: item.passing_points,
            show_results_during: item.show_results_during,
            affects_rating: item.affects_rating,
            method: item.method,
            questions: item.questions,
            assessment_id: Uuid::now_v7(),
            section_id: Uuid::now_v7(),
            total_points,
            question_count,
        });
    }

    // Open transaction
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let is_admin = auth.user.role == crate::domain::user::Role::Admin;
    crate::http::db::set_rls_guc(&mut tx, auth.owner_id, is_admin).await?;

    // Bulk insert assessments (16 columns)
    {
        let mut qb = QueryBuilder::new(
            "INSERT INTO tb_assessments \
             (id, title, description, mode, status, objectives, course, duration_min, \
              time_limit_seconds, passing_points, show_results_during, affects_rating, \
              method, created_by, owner_id, total_points) ",
        );

        qb.push_values(&validated, |mut b, item| {
            b.push_bind(item.assessment_id)
                .push_bind(&item.title)
                .push_bind(&item.description)
                .push_bind(&item.mode)
                .push_bind(&item.status)
                .push_bind(&item.objectives)
                .push_bind(&item.course)
                .push_bind(item.duration_min)
                .push_bind(item.time_limit_seconds)
                .push_bind(item.passing_points)
                .push_bind(item.show_results_during)
                .push_bind(item.affects_rating)
                .push_bind(&item.method)
                .push_bind(auth.user.id)
                .push_bind(auth.owner_id)
                .push_bind(item.total_points);
        });

        qb.build()
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    // Bulk insert sections (one default section per assessment)
    {
        let mut qb = QueryBuilder::new(
            "INSERT INTO tb_assessment_sections (id, assessment_id, title, order_index, weight, items_count) ",
        );

        qb.push_values(&validated, |mut b, item| {
            b.push_bind(item.section_id)
                .push_bind(item.assessment_id)
                .push_bind("Main Section")
                .push_bind(0i32)
                .push_bind(1.0f64)
                .push_bind(item.question_count as i32);
        });

        qb.build()
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    // Flatten all questions: collect (question_id, assessment_item) tuples
    #[derive(Debug)]
    struct QuestionRecord {
        question_id: Uuid,
        owner_id: Uuid,
        kind: String,
        prompt: String,
        payload: serde_json::Value,
        explanation: Option<String>,
        points: i32,
        created_by: Uuid,
    }

    #[derive(Debug)]
    struct AssessmentItemRecord {
        section_id: Uuid,
        question_id: Uuid,
        order_index: i32,
    }

    let mut all_questions = Vec::new();
    let mut all_assessment_items = Vec::new();
    let mut all_tags: std::collections::HashMap<String, Vec<Uuid>> =
        std::collections::HashMap::new();

    for validated_item in &validated {
        for (q_idx, q) in validated_item.questions.iter().enumerate() {
            let question_id = Uuid::now_v7();
            all_questions.push(QuestionRecord {
                question_id,
                owner_id: auth.owner_id,
                kind: q.kind.as_str().to_string(),
                prompt: q.prompt.clone(),
                payload: q.payload.clone(),
                explanation: q.explanation.clone(),
                points: q.points.unwrap_or(1),
                created_by: auth.user.id,
            });

            all_assessment_items.push(AssessmentItemRecord {
                section_id: validated_item.section_id,
                question_id,
                order_index: q_idx as i32,
            });

            // Collect tags: map tag_name -> [question_ids]
            for tag_name in &q.tags {
                all_tags
                    .entry(tag_name.clone())
                    .or_default()
                    .push(question_id);
            }
        }
    }

    // Bulk insert questions (9 columns: id, owner_id, kind, prompt, payload, explanation, status, points, created_by)
    if !all_questions.is_empty() {
        let mut qb = QueryBuilder::new(
            "INSERT INTO tb_questions (id, owner_id, kind, prompt, payload, explanation, status, points, created_by) ",
        );

        qb.push_values(&all_questions, |mut b, q| {
            b.push_bind(q.question_id)
                .push_bind(q.owner_id)
                .push_bind(&q.kind)
                .push_bind(&q.prompt)
                .push_bind(&q.payload)
                .push_bind(&q.explanation)
                .push_bind("live")
                .push_bind(q.points)
                .push_bind(q.created_by);
        });

        qb.build()
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    // Bulk insert assessment items
    if !all_assessment_items.is_empty() {
        let mut qb = QueryBuilder::new(
            "INSERT INTO tb_assessment_items (section_id, question_id, order_index) ",
        );

        qb.push_values(&all_assessment_items, |mut b, item| {
            b.push_bind(item.section_id)
                .push_bind(item.question_id)
                .push_bind(item.order_index);
        });

        qb.build()
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
    }

    // Handle tags: collect all unique tag names, upsert into tb_tags, then bulk insert tb_question_tags
    if !all_tags.is_empty() {
        let tag_names: Vec<&String> = all_tags.keys().collect();

        // Upsert tags (insert or do nothing if exists)
        {
            let mut qb = QueryBuilder::new("INSERT INTO tb_tags (name) ");
            qb.push_values(&tag_names, |mut b, name| {
                b.push_bind(name);
            });
            qb.push(" ON CONFLICT (name) DO NOTHING");

            qb.build()
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
        }

        // Fetch tag id -> name mapping
        let tag_rows: Vec<(Uuid, String)> =
            sqlx::query_as("SELECT id, name FROM tb_tags WHERE name = ANY($1::text[])")
                .bind(&tag_names)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

        let tag_id_map: std::collections::HashMap<String, Uuid> =
            tag_rows.into_iter().map(|(id, name)| (name, id)).collect();

        // Build question_tags records
        #[derive(Debug)]
        struct QuestionTagRecord {
            question_id: Uuid,
            tag_id: Uuid,
        }

        let mut all_question_tags = Vec::new();
        for (tag_name, question_ids) in &all_tags {
            if let Some(&tag_id) = tag_id_map.get(tag_name) {
                for &question_id in question_ids {
                    all_question_tags.push(QuestionTagRecord {
                        question_id,
                        tag_id,
                    });
                }
            }
        }

        // Bulk insert question tags with conflict handling
        if !all_question_tags.is_empty() {
            let mut qb = QueryBuilder::new("INSERT INTO tb_question_tags (question_id, tag_id) ");

            qb.push_values(&all_question_tags, |mut b, qt| {
                b.push_bind(qt.question_id).push_bind(qt.tag_id);
            });
            qb.push(" ON CONFLICT (question_id, tag_id) DO NOTHING");

            qb.build()
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
        }
    }

    // Commit transaction
    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Build response: created array in same order as input
    let mut created = Vec::new();
    for item in &validated {
        created.push(json!({
            "id": item.assessment_id,
            "status": item.status,
        }));
    }

    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.batchCreate".into(),
        result: json!({
            "created": created,
            "count": created.len(),
        }),
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
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conn = &mut *acquired;
    crate::http::db::set_rls_guc(conn, auth.owner_id, false).await?;
    let db = crate::http::db::DbConn(acquired);

    let res = super::assessments::patch_assessment(
        auth.clone(),
        State(state.clone()),
        db,
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

async fn run_assessment_add_question(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let assessment_id = parse_id(&params)?;
    let body: super::assessments::AddAssessmentQuestionBody = serde_json::from_value(params)
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid add question request: {e}"),
            }])
        })?;
    let mut conn = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let is_admin = auth.user.role == crate::domain::user::Role::Admin;
    crate::http::db::set_rls_guc(&mut conn, auth.owner_id, is_admin).await?;
    let db = crate::http::db::DbConn(conn);
    let res = super::assessments::add_assessment_question(
        State(state.clone()),
        db,
        auth.clone(),
        Path(assessment_id),
        Json(body),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.addQuestion".into(),
        result: serde_json::to_value(res.1.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_question_create(
    state: &AppState,
    auth: &AuthenticatedUser,
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
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conn = &mut *acquired;
    let owner_id = auth.owner_id;

    crate::http::db::set_rls_guc(conn, owner_id, false).await?;

    let created = repo::create_questions(conn, auth.user.id, owner_id, questions).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "question.create".into(),
        result: json!({ "created": created.len(), "questions": created }),
        error: None,
    }))
}

async fn run_question_promote(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conn = &mut *acquired;
    let owner_id = auth.owner_id;

    crate::http::db::set_rls_guc(conn, owner_id, false).await?;

    let question = crate::bank::questions::promote_question(conn, id).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "question.promote".into(),
        result: serde_json::to_value(question).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_question_update(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let patch: crate::bank::questions::QuestionPatch = serde_json::from_value(params.clone())
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid patch params: {e}"),
            }])
        })?;

    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conn = &mut *acquired;
    let owner_id = auth.owner_id;

    crate::http::db::set_rls_guc(conn, owner_id, false).await?;

    let updated = crate::bank::questions::update_question(conn, id, patch).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "question.update".into(),
        result: serde_json::to_value(updated).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_question_deepen(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let exclude_str: Option<String> = params
        .get("exclude")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let conn = &mut *acquired;
    let owner_id = auth.owner_id;

    crate::http::db::set_rls_guc(conn, owner_id, false).await?;

    use crate::bank::questions as repo;
    let question = repo::get_question(conn, id)
        .await?
        .ok_or(ApiError::NotFound {
            resource: "question",
        })?;

    // Check ownership if not admin
    if auth.user.role != crate::domain::user::Role::Admin
        && !repo::question_in_owner_scope(conn, id, owner_id).await?
    {
        return Err(ApiError::NotFound {
            resource: "question",
        });
    }

    // Reuse existing in-progress/pending session gate (no key reveal pre-submit)
    if auth.user.role != crate::domain::user::Role::Admin {
        let active: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM tb_sessions s
                WHERE s.owner_id = $1
                  AND s.status = 'in_progress'
                  AND s.question_plan->'items' @> jsonb_build_array(jsonb_build_object('question_id', $2))
            )
            "#
        )
        .bind(auth.user.id)
        .bind(id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

        if active {
            return Err(ApiError::Forbidden(std::borrow::Cow::Borrowed(
                "cannot access deepening details while an active session is in progress",
            )));
        }
    }

    let exclude_ids: Vec<Uuid> = exclude_str
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| Uuid::parse_str(s.trim()).ok())
        .collect();

    let related = repo::get_related_questions(conn, id, &exclude_ids).await?;

    let res = super::questions::QuestionDeepenResponse { question, related };

    Ok(Json(RunResponse {
        ok: true,
        tool: "question.deepen".into(),
        result: serde_json::to_value(res).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_deep_dive_request(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let body: super::deep_dives::CreateDeepDiveBody =
        serde_json::from_value(params).map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid deep dive request params: {e}"),
            }])
        })?;
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    crate::http::db::set_rls_guc(&mut acquired, auth.owner_id, false).await?;
    let db = crate::http::db::DbConn(acquired);
    let res = super::deep_dives::create_deep_dive(
        State(state.clone()),
        db,
        RequireAnyScope::new(auth.clone()),
        Json(body),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "deepDive.request".into(),
        result: serde_json::to_value(res.1.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_deep_dive_list(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let query: super::deep_dives::ListDeepDivesQuery =
        serde_json::from_value(params).map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid deep dive list params: {e}"),
            }])
        })?;
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    crate::http::db::set_rls_guc(&mut acquired, auth.owner_id, false).await?;
    let db = crate::http::db::DbConn(acquired);
    let res = super::deep_dives::list_deep_dives(
        State(state.clone()),
        db,
        RequireAnyScope::new(auth.clone()),
        Query(query),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "deepDive.list".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunDeepDivePublishParams {
    id: Uuid,
    body_markdown: String,
    status: Option<String>,
    category: Option<String>,
}

async fn run_deep_dive_publish(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let params: RunDeepDivePublishParams = serde_json::from_value(params).map_err(|e| {
        ApiError::Validation(vec![FieldError {
            field: "params".into(),
            message: format!("invalid deep dive publish params: {e}"),
        }])
    })?;
    let mut acquired = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    crate::http::db::set_rls_guc(&mut acquired, auth.owner_id, false).await?;
    let db = crate::http::db::DbConn(acquired);
    let body = super::deep_dives::PublishDeepDiveBody {
        body_markdown: params.body_markdown,
        status: params.status,
        category: params.category,
    };
    let res = super::deep_dives::publish_deep_dive(
        State(state.clone()),
        db,
        RequireAnyScope::new(auth.clone()),
        Path(params.id),
        Json(body),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "deepDive.publish".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

// ── behavioral tools (P1 sub-account profile) ──────────────────────────────────

/// Loads the calling agent's profile row, returning a 404-style error if the
/// caller has no profile (i.e. is not an agent sub-account).
async fn load_agent_profile(
    state: &AppState,
    agent_id: &Uuid,
) -> Result<sqlx::postgres::PgRow, ApiError> {
    sqlx::query(
        "SELECT i.label, i.metadata FROM tb_agents a
         JOIN tb_identities i ON i.id = a.id
         WHERE a.id = $1 AND a.revoked_at IS NULL AND i.status = 'active'",
    )
    .bind(agent_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound {
        resource: "agent_profile",
    })
}

/// profile.get — the agent's own config plus owner shared truth (per-tag ratings).
async fn run_profile_get(
    state: &AppState,
    auth: &AuthenticatedUser,
) -> Result<Json<RunResponse>, ApiError> {
    let profile = load_agent_profile(state, &auth.user.id).await?;
    let owner_id = auth.owner_id();

    let rating_rows = sqlx::query(
        "SELECT t.name AS tag, r.rating AS rating \
         FROM tb_user_tag_ratings r JOIN tb_tags t ON t.id = r.tag_id \
         WHERE r.user_id = $1 ORDER BY r.rating DESC",
    )
    .bind(owner_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let ratings: Vec<Value> = rating_rows
        .iter()
        .map(|row| {
            json!({
                "tag": row.get::<String, _>("tag"),
                "rating": row.get::<f64, _>("rating"),
            })
        })
        .collect();

    let metadata = profile.get::<Value, _>("metadata");
    let agent = json!({
        "label": profile.get::<String, _>("label"),
        "focusTags": metadata.get("focus_tags").cloned().unwrap_or_else(|| json!([])),
        "currentGoal": metadata.get("current_goal").cloned().unwrap_or(Value::Null),
        "nextTarget": metadata.get("next_target").cloned().unwrap_or(Value::Null),
        "memory": metadata.get("memory").cloned().unwrap_or_else(|| json!({})),
    });

    Ok(Json(RunResponse {
        ok: true,
        tool: "profile.get".into(),
        result: json!({
            "agent": agent,
            "owner": { "id": owner_id, "ratings": ratings },
        }),
        error: None,
    }))
}

/// memory.set — replace the agent's memory blob wholesale.
async fn run_memory_set(
    state: &AppState,
    agent_id: &Uuid,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let memory = params.get("memory").cloned().ok_or_else(|| {
        ApiError::Validation(vec![FieldError {
            field: "memory".into(),
            message: "missing memory object".into(),
        }])
    })?;
    update_profile_memory(state, agent_id, memory, false).await
}

/// memory.append — shallow-merge the given keys into the agent's memory blob.
async fn run_memory_append(
    state: &AppState,
    agent_id: &Uuid,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let append = params.get("append").cloned().ok_or_else(|| {
        ApiError::Validation(vec![FieldError {
            field: "append".into(),
            message: "missing append object".into(),
        }])
    })?;
    update_profile_memory(state, agent_id, append, true).await
}

async fn update_profile_memory(
    state: &AppState,
    agent_id: &Uuid,
    value: Value,
    merge: bool,
) -> Result<Json<RunResponse>, ApiError> {
    // `memory || $1` shallow-merges; `$1` replaces.
    let sql = if merge {
        "UPDATE tb_identities SET metadata = metadata || jsonb_build_object('memory',
         COALESCE(metadata->'memory', '{}'::jsonb) || $1::jsonb)
         WHERE id = $2 RETURNING metadata->'memory' AS memory"
    } else {
        "UPDATE tb_identities SET metadata = jsonb_set(metadata, '{memory}', $1::jsonb)
         WHERE id = $2 RETURNING metadata->'memory' AS memory"
    };
    let row = sqlx::query(sql)
        .bind(&value)
        .bind(agent_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::NotFound {
            resource: "agent_profile",
        })?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "memory.set".into(),
        result: json!({ "memory": row.get::<Value, _>("memory") }),
        error: None,
    }))
}

/// target.set — update the agent's current goal and/or next target.
async fn run_target_set(
    state: &AppState,
    agent_id: &Uuid,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let current_goal = params.get("currentGoal").and_then(|v| v.as_str());
    let next_target = params.get("nextTarget").and_then(|v| v.as_str());

    let row = sqlx::query(
        "UPDATE tb_identities SET metadata = metadata
         || CASE WHEN $1::text IS NULL THEN '{}'::jsonb ELSE jsonb_build_object('current_goal', $1) END
         || CASE WHEN $2::text IS NULL THEN '{}'::jsonb ELSE jsonb_build_object('next_target', $2) END
         WHERE id = $3
         RETURNING metadata->>'current_goal' AS current_goal,
                   metadata->>'next_target' AS next_target",
    )
    .bind(current_goal)
    .bind(next_target)
    .bind(agent_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound {
        resource: "agent_profile",
    })?;

    Ok(Json(RunResponse {
        ok: true,
        tool: "target.set".into(),
        result: json!({
            "currentGoal": row.get::<Option<String>, _>("current_goal"),
            "nextTarget": row.get::<Option<String>, _>("next_target"),
        }),
        error: None,
    }))
}

async fn run_attempt_grade(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    #[derive(Debug, Deserialize)]
    struct GradeParams {
        id: Uuid,
        score: f64,
        notes: Option<String>,
    }
    let p: GradeParams = serde_json::from_value(params).map_err(|e| {
        ApiError::Validation(vec![crate::domain::error::FieldError {
            field: "params".into(),
            message: e.to_string(),
        }])
    })?;

    if p.score < 0.0 || p.score > 1.0 {
        return Err(ApiError::Validation(vec![
            crate::domain::error::FieldError {
                field: "score".into(),
                message: "must be between 0.0 and 1.0".into(),
            },
        ]));
    }

    let row = sqlx::query(
        "SELECT a.grade_status \
          FROM tb_attempts a \
          JOIN tb_questions q ON q.id = a.question_id \
          WHERE a.id = $1 \
            AND (q.created_by = $2 \
                 OR EXISTS (SELECT 1 FROM tb_agents ag WHERE ag.id = q.created_by AND ag.owner_user_id = $2))",
    )
    .bind(p.id)
    .bind(auth.owner_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let row = row.ok_or(ApiError::NotFound {
        resource: "attempt",
    })?;

    let grade_status: String = row.get("grade_status");
    if grade_status != "pending_manual" {
        return Err(ApiError::Validation(vec![
            crate::domain::error::FieldError {
                field: "grade_status".into(),
                message: "attempt is not pending manual grading".into(),
            },
        ]));
    }

    let updated = sqlx::query(
        "UPDATE tb_attempts \
         SET score = $2, is_correct = ($2 > 0), grade_status = 'graded', grader_notes = $3 \
         WHERE id = $1 \
         RETURNING id, user_id, question_id, question_version, session_id, response, presentation, \
                   is_correct, score, grade_status, correct_answer, grader_notes, time_to_answer_ms, \
                   rating_before_user_avg, rating_before_question, user_tag_deltas, \
                   question_delta, created_at",
    )
    .bind(p.id)
    .bind(p.score)
    .bind(&p.notes)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let attempt = crate::http::sessions::row_to_attempt(updated)?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "attempt.grade".into(),
        result: serde_json::to_value(attempt).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_assessment_list(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let query_params: super::assessments::ListParams =
        serde_json::from_value(params).map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid list params: {e}"),
            }])
        })?;
    let res = super::assessments::list_assessments(
        auth.clone(),
        State(state.clone()),
        Query(query_params),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.list".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_assessment_get(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let mut conn = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let is_admin = auth.user.role == crate::domain::user::Role::Admin;
    crate::http::db::set_rls_guc(&mut conn, auth.owner_id, is_admin).await?;
    let db = crate::http::db::DbConn(conn);
    let res = super::assessments::get_assessment(auth.clone(), db, Path(id)).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.get".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_question_list(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let query_params: super::questions::ListQuestionsQuery = serde_json::from_value(params)
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid question list params: {e}"),
            }])
        })?;
    let mut conn = state
        .pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let is_admin = auth.user.role == crate::domain::user::Role::Admin;
    crate::http::db::set_rls_guc(&mut conn, auth.owner_id, is_admin).await?;
    let db = crate::http::db::DbConn(conn);
    let res = super::questions::list_questions(
        db,
        RequireAnyScope::new(auth.clone()),
        Query(query_params),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "question.list".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_assessment_stats(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let id = parse_id(&params)?;
    let stats_params: super::stats::AssessmentStatsParams = serde_json::from_value(params)
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid stats params: {e}"),
            }])
        })?;
    let res = super::stats::assessment_stats(
        State(state.clone()),
        RequireScope::new(auth.clone()),
        Path(id),
        Query(stats_params),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "assessment.stats".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_activity_list(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let query_params: ActivityQuery = serde_json::from_value(params).map_err(|e| {
        ApiError::Validation(vec![FieldError {
            field: "params".into(),
            message: format!("invalid activity params: {e}"),
        }])
    })?;
    let res = activity(
        State(state.clone()),
        RequireAnyScope::new(auth.clone()),
        Query(query_params),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "activity.list".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_stats_user(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let query_params: super::stats::MeStatsQuery = serde_json::from_value(params).map_err(|e| {
        ApiError::Validation(vec![FieldError {
            field: "params".into(),
            message: format!("invalid stats params: {e}"),
        }])
    })?;
    let res =
        super::stats::me_stats(State(state.clone()), auth.clone(), Query(query_params)).await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "stats.user".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
        error: None,
    }))
}

async fn run_attempt_list(
    state: &AppState,
    auth: &AuthenticatedUser,
    params: Value,
) -> Result<Json<RunResponse>, ApiError> {
    let query_params: super::me::ListAttemptsQuery =
        serde_json::from_value(params).map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "params".into(),
                message: format!("invalid attempts params: {e}"),
            }])
        })?;
    // list_attempts is owner-scoped (binds auth.owner_id) and reads tb_attempts
    // directly off the pool, so no RLS conn is needed here.
    let res = super::me::list_attempts(
        State(state.clone()),
        RequireAnyScope::new(auth.clone()),
        Query(query_params),
    )
    .await?;
    Ok(Json(RunResponse {
        ok: true,
        tool: "attempt.list".into(),
        result: serde_json::to_value(res.0).unwrap_or(Value::Null),
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
        .route("/llms.txt", get(llms_txt))
        .route("/skill.json", get(skill_manifest))
        .with_state(state)
}

pub fn logged_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/agents/activity", get(activity))
        .route("/v1/agents/run", post(run))
        .with_state(state)
}
