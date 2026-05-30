//! Profile, key and webhook management routes: /v1/me, /v1/me/keys, /v1/me/webhooks, /v1/me/attempts.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireAnyScope, ScopeOneOf},
        token::{generate_secret, hash_secret},
    },
    domain::{
        attempt::Attempt,
        error::{ApiError, FieldError},
        user::{Role, Scope},
    },
    http::AppState,
};

// ── key shapes ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeySummary {
    pub id: Uuid,
    pub name: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListKeysResponse {
    pub keys: Vec<KeySummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateKeyBody {
    pub label: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateKeyResponse {
    pub api_key: String,
    pub prefix: String,
    pub id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RotateKeyResponse {
    pub api_key: String,
}

// ── agent shapes ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub id: Uuid,
    pub label: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentSummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentBody {
    pub label: String,
    pub scopes: Vec<String>,
    pub focus_tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentResponse {
    pub api_key: String,
    pub id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentBody {
    pub label: Option<String>,
    pub focus_tags: Option<Vec<String>>,
    pub current_goal: Option<String>,
    pub next_target: Option<String>,
}

// ── webhook shapes ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookSummary {
    pub id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookSummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookBody {
    pub url: String,
    pub events: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookResponse {
    pub id: Uuid,
    pub secret: String,
}

// ── key handlers ──────────────────────────────────────────────────────────────

pub async fn list_keys(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ListKeysResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, name, scopes, last_used_at, created_at
         FROM tb_api_tokens
         WHERE user_id = $1 AND revoked_at IS NULL
         ORDER BY created_at DESC",
    )
    .bind(user.user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let keys = rows
        .iter()
        .map(|r| KeySummary {
            id: r.get("id"),
            name: r.get("name"),
            scopes: r.get("scopes"),
            last_used_at: r.get("last_used_at"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListKeysResponse { keys }))
}

pub async fn create_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateKeyBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.label.trim().is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "label".into(),
            message: "must not be empty".into(),
        }]));
    }
    if body.scopes.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "scopes".into(),
            message: "must include at least one scope".into(),
        }]));
    }

    // Only admin users may create keys that include the 'admin' scope
    let requesting_admin = user.token_scopes.contains(&Scope::Admin)
        || matches!(user.user.role, crate::domain::user::Role::Admin);
    if body.scopes.contains(&"admin".to_string()) && !requesting_admin {
        return Err(ApiError::ScopeRequired(Scope::Admin.as_str()));
    }

    // Validate all requested scopes are known
    for s in &body.scopes {
        if s.parse::<Scope>().is_err() {
            return Err(ApiError::Validation(vec![FieldError {
                field: "scopes".into(),
                message: format!("unknown scope: {s}"),
            }]));
        }
    }

    let token_id = Uuid::now_v7();
    let secret = generate_secret();
    let hash = hash_secret(&secret);
    let prefix = format!("hk_{}", &secret[..8]);

    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user.user.id)
    .bind(&body.label)
    .bind(&hash)
    .bind(&body.scopes)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateKeyResponse {
            api_key: format!("{token_id}_{secret}"),
            prefix,
            id: token_id,
        }),
    ))
}

pub async fn rotate_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<RotateKeyResponse>, ApiError> {
    // Verify key belongs to caller and is active
    let row = sqlx::query(
        "SELECT id, name, scopes FROM tb_api_tokens
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(key_id)
    .bind(user.user.id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound { resource: "key" })?;

    let name: String = row.get("name");
    let scopes: Vec<String> = row.get("scopes");

    // Revoke old key
    sqlx::query("UPDATE tb_api_tokens SET revoked_at = now() WHERE id = $1")
        .bind(key_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Issue new key with same name and scopes
    let new_token_id = Uuid::now_v7();
    let new_secret = generate_secret();
    let new_hash = hash_secret(&new_secret);

    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(new_token_id)
    .bind(user.user.id)
    .bind(&name)
    .bind(&new_hash)
    .bind(&scopes)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(RotateKeyResponse {
        api_key: format!("{new_token_id}_{new_secret}"),
    }))
}

pub async fn revoke_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(key_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let affected = sqlx::query(
        "UPDATE tb_api_tokens SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(key_id)
    .bind(user.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound { resource: "key" });
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── agent handlers ────────────────────────────────────────────────────────────

/// List agents owned by the current user.
#[utoipa::path(
    get,
    path = "/v1/me/agents",
    responses(
        (status = 200, description = "Agent list", body = ListAgentsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn list_agents(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ListAgentsResponse>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT 
            u.id, u.display_name as label, u.created_at,
            t.scopes, t.last_used_at
        FROM tb_users u
        LEFT JOIN tb_api_tokens t ON t.user_id = u.id AND t.revoked_at IS NULL
        WHERE u.owner_user_id = $1 AND u.role = 'agent'
        ORDER BY u.created_at DESC
        "#,
    )
    .bind(user.user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let agents = rows
        .iter()
        .map(|r| {
            let scopes: Option<Vec<String>> = r.get("scopes");
            AgentSummary {
                id: r.get("id"),
                label: r.get("label"),
                scopes: scopes.unwrap_or_default(),
                last_used_at: r.get("last_used_at"),
                created_at: r.get("created_at"),
            }
        })
        .collect();

    Ok(Json(ListAgentsResponse { agents }))
}

/// Create a new agent sub-account.
#[utoipa::path(
    post,
    path = "/v1/me/agents",
    request_body = CreateAgentBody,
    responses(
        (status = 201, description = "Agent created", body = CreateAgentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn create_agent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateAgentBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.label.trim().is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "label".into(),
            message: "must not be empty".into(),
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

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let agent_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_users (id, owner_user_id, display_name, role) VALUES ($1, $2, $3, 'agent')",
    )
    .bind(agent_id)
    .bind(user.user.id)
    .bind(&body.label)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let token_id = Uuid::now_v7();
    let secret = generate_secret();
    let hash = hash_secret(&secret);

    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(agent_id)
    .bind(&body.label)
    .bind(&hash)
    .bind(&body.scopes)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    sqlx::query(
        "INSERT INTO tb_agent_profiles (agent_user_id, label, focus_tags) VALUES ($1, $2, $3)",
    )
    .bind(agent_id)
    .bind(&body.label)
    .bind(body.focus_tags.unwrap_or_default())
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateAgentResponse {
            api_key: format!("{token_id}_{secret}"),
            id: agent_id,
        }),
    ))
}

/// Update an agent's label.
#[utoipa::path(
    patch,
    path = "/v1/me/agents/{id}",
    params(
        ("id" = Uuid, Path, description = "Agent ID"),
    ),
    request_body = UpdateAgentBody,
    responses(
        (status = 204, description = "Agent updated"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn update_agent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<UpdateAgentBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Check ownership/existence even for empty patches
    let owner_check = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tb_users WHERE id = $1 AND owner_user_id = $2 AND role = 'agent')",
    )
    .bind(agent_id)
    .bind(user.user.id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if !owner_check {
        return Err(ApiError::NotFound { resource: "agent" });
    }

    if let Some(label) = &body.label {
        if label.trim().is_empty() {
            return Err(ApiError::Validation(vec![FieldError {
                field: "label".into(),
                message: "must not be empty".into(),
            }]));
        }

        sqlx::query(
            "UPDATE tb_users SET display_name = $1 WHERE id = $2 AND owner_user_id = $3 AND role = 'agent'",
        )
        .bind(label)
        .bind(agent_id)
        .bind(user.user.id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Delete an agent sub-account.
#[utoipa::path(
    delete,
    path = "/v1/me/agents/{id}",
    params(
        ("id" = Uuid, Path, description = "Agent ID"),
    ),
    responses(
        (status = 204, description = "Agent deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn delete_agent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(agent_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let affected =
        sqlx::query("DELETE FROM tb_users WHERE id = $1 AND owner_user_id = $2 AND role = 'agent'")
            .bind(agent_id)
            .bind(user.user.id)
            .execute(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound { resource: "agent" });
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── webhook handlers ──────────────────────────────────────────────────────────

pub async fn list_webhooks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ListWebhooksResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, url, events, created_at
         FROM tb_webhooks
         WHERE user_id = $1 AND revoked_at IS NULL
         ORDER BY created_at DESC",
    )
    .bind(user.user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let webhooks = rows
        .iter()
        .map(|r| WebhookSummary {
            id: r.get("id"),
            url: r.get("url"),
            events: r.get("events"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListWebhooksResponse { webhooks }))
}

pub async fn create_webhook(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateWebhookBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.url.trim().is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "url".into(),
            message: "must not be empty".into(),
        }]));
    }

    let webhook_id = Uuid::now_v7();
    let secret = generate_secret();
    let hash = hash_secret(&secret);

    sqlx::query(
        "INSERT INTO tb_webhooks (id, user_id, url, events, secret_hash, signing_key)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(webhook_id)
    .bind(user.user.id)
    .bind(&body.url)
    .bind(&body.events)
    .bind(&hash)
    .bind(&secret)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // Secret shown once; signing_key stored for outbound HMAC dispatch
    Ok((
        StatusCode::CREATED,
        Json(CreateWebhookResponse {
            id: webhook_id,
            secret,
        }),
    ))
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(webhook_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let affected = sqlx::query(
        "UPDATE tb_webhooks SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(webhook_id)
    .bind(user.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "webhook",
        });
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── profile ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub role: Role,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

/// Get the authenticated user's profile.
#[utoipa::path(
    get,
    path = "/v1/me",
    responses(
        (status = 200, description = "Current user profile", body = MeResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
async fn get_me(auth: AuthenticatedUser) -> Result<Json<MeResponse>, ApiError> {
    Ok(Json(MeResponse {
        id: auth.user.id,
        display_name: auth.user.display_name,
        email: auth.user.email,
        role: auth.user.role,
        created_at: auth.user.created_at,
    }))
}

// ── attempts ──────────────────────────────────────────────────────────────────

pub struct AttemptReadScopes;
impl ScopeOneOf for AttemptReadScopes {
    const SCOPES: &'static [Scope] = &[Scope::AttemptRead];
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAttemptsQuery {
    #[serde(default = "default_attempts_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    /// Filter to attempts within a specific session.
    pub session_id: Option<Uuid>,
}

fn default_attempts_limit() -> i64 {
    50
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListAttemptsResponse {
    pub attempts: Vec<Attempt>,
    pub total: i64,
}

/// List the current user's attempts.
#[utoipa::path(
    get,
    path = "/v1/me/attempts",
    params(
        ("limit" = Option<i64>, Query, description = "Page size (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Page offset"),
        ("session_id" = Option<Uuid>, Query, description = "Filter by session"),
    ),
    responses(
        (status = 200, description = "Attempt list", body = ListAttemptsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
async fn list_attempts(
    State(state): State<AppState>,
    auth: RequireAnyScope<AttemptReadScopes>,
    Query(q): Query<ListAttemptsQuery>,
) -> Result<Json<ListAttemptsResponse>, ApiError> {
    let rows = if let Some(sid) = q.session_id {
        sqlx::query(
            "SELECT id, user_id, question_id, question_version, session_id, response,
                    presentation, is_correct, score, grade_status, grader_notes, time_to_answer_ms,
                    rating_before_user_avg, rating_before_question,
                    user_tag_deltas, question_delta, created_at,
                    COUNT(*) OVER() AS total
             FROM tb_attempts
             WHERE user_id = $1 AND session_id = $2
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4",
        )
        .bind(auth.0.user.id)
        .bind(sid)
        .bind(q.limit)
        .bind(q.offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
    } else {
        sqlx::query(
            "SELECT id, user_id, question_id, question_version, session_id, response,
                    presentation, is_correct, score, grade_status, grader_notes, time_to_answer_ms,
                    rating_before_user_avg, rating_before_question,
                    user_tag_deltas, question_delta, created_at,
                    COUNT(*) OVER() AS total
             FROM tb_attempts
             WHERE user_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(auth.0.user.id)
        .bind(q.limit)
        .bind(q.offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
    };

    let total: i64 = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let attempts = rows
        .into_iter()
        .map(|r| Attempt {
            id: r.get("id"),
            user_id: r.get("user_id"),
            question_id: r.get("question_id"),
            question_version: r.get("question_version"),
            session_id: r.get("session_id"),
            response: serde_json::from_value(r.get("response")).unwrap_or(
                crate::domain::attempt::AttemptResponse::Short {
                    answer: String::new(),
                },
            ),
            presentation: serde_json::from_value(r.get("presentation")).unwrap_or_default(),
            is_correct: r.get("is_correct"),
            score: r.get("score"),
            grade_status: r.get("grade_status"),
            grader_notes: r.get("grader_notes"),
            time_to_answer_ms: r.get("time_to_answer_ms"),
            rating_before_user_avg: r.get("rating_before_user_avg"),
            rating_before_question: r.get("rating_before_question"),
            user_tag_deltas: serde_json::from_value(r.get("user_tag_deltas")).unwrap_or_default(),
            question_delta: r.get("question_delta"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListAttemptsResponse { attempts, total }))
}

// ── cohort stats ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CohortStatsQuery {
    pub quiz_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CohortStatsBucket {
    pub bucket_start: f64,
    pub bucket_end: f64,
    pub count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CohortStatsResponse {
    pub cohort_avg: Option<f64>,
    pub percentile: Option<i32>,
    pub completion_rate: Option<f64>,
    pub histogram: Vec<CohortStatsBucket>,
}

#[utoipa::path(
    get,
    path = "/v1/me/cohort-stats",
    params(
        ("quizId" = Uuid, Query, description = "Quiz to compare against"),
    ),
    responses(
        (status = 200, description = "Cohort stats", body = CohortStatsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
async fn get_cohort_stats(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(q): Query<CohortStatsQuery>,
) -> Result<Json<CohortStatsResponse>, ApiError> {
    let quiz_id = q.quiz_id;
    let user_id = auth.user.id;

    let cohort_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT cm.cohort_id FROM tb_cohort_memberships cm WHERE cm.user_id = $1 LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let Some(cohort_id) = cohort_id else {
        return Ok(Json(CohortStatsResponse {
            cohort_avg: None,
            percentile: None,
            completion_rate: None,
            histogram: empty_histogram(),
        }));
    };

    let total_members: i64 =
        sqlx::query_scalar("SELECT count(*) FROM tb_cohort_memberships WHERE cohort_id = $1")
            .bind(cohort_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    if total_members == 0 {
        return Ok(Json(CohortStatsResponse {
            cohort_avg: None,
            percentile: None,
            completion_rate: None,
            histogram: empty_histogram(),
        }));
    }

    let agg_row = sqlx::query(
        "SELECT
            avg((s.result->>'percent')::double precision) as cohort_avg,
            count(distinct s.user_id) as finishers
         FROM tb_sessions s
         WHERE s.quiz_id = $1
           AND s.status = 'finished'
           AND s.result IS NOT NULL
           AND s.user_id IN (
               SELECT cm.user_id FROM tb_cohort_memberships cm WHERE cm.cohort_id = $2
           )",
    )
    .bind(quiz_id)
    .bind(cohort_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let cohort_avg: Option<f64> = agg_row.get("cohort_avg");
    let finishers: i64 = agg_row.get("finishers");
    let completion_rate = Some(finishers as f64 / total_members as f64);

    let user_score: Option<f64> = sqlx::query_scalar(
        "SELECT (result->>'percent')::double precision
         FROM tb_sessions
         WHERE quiz_id = $1
           AND user_id = $2
           AND status = 'finished'
           AND result IS NOT NULL
         ORDER BY (result->>'percent')::double precision DESC
         LIMIT 1",
    )
    .bind(quiz_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .flatten();

    let percentile: Option<i32> = if let Some(score) = user_score {
        let count_at_or_below: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM (
                SELECT DISTINCT ON (s.user_id) s.user_id,
                    (s.result->>'percent')::double precision as best_score
                FROM tb_sessions s
                WHERE s.quiz_id = $1
                  AND s.status = 'finished'
                  AND s.result IS NOT NULL
                  AND s.user_id IN (
                      SELECT cm.user_id FROM tb_cohort_memberships cm WHERE cm.cohort_id = $2
                  )
                ORDER BY s.user_id, (s.result->>'percent')::double precision DESC
             ) ranked
             WHERE ranked.best_score <= $3",
        )
        .bind(quiz_id)
        .bind(cohort_id)
        .bind(score)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
        Some(((count_at_or_below as f64 / total_members as f64) * 100.0).round() as i32)
    } else {
        None
    };

    let hist_rows = sqlx::query(
        "SELECT
            floor((s.result->>'percent')::double precision * 10)::int as bucket,
            count(*) as count
         FROM tb_sessions s
         WHERE s.quiz_id = $1
           AND s.status = 'finished'
           AND s.result IS NOT NULL
           AND s.user_id IN (
               SELECT cm.user_id FROM tb_cohort_memberships cm WHERE cm.cohort_id = $2
           )
         GROUP BY 1
         ORDER BY 1",
    )
    .bind(quiz_id)
    .bind(cohort_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let mut histogram = empty_histogram();
    for row in &hist_rows {
        let bucket: i32 = row.get::<i32, _>("bucket").clamp(0, 9);
        let count: i64 = row.get("count");
        histogram[bucket as usize].count += count;
    }

    Ok(Json(CohortStatsResponse {
        cohort_avg,
        percentile,
        completion_rate,
        histogram,
    }))
}

fn empty_histogram() -> Vec<CohortStatsBucket> {
    (0..10)
        .map(|i| CohortStatsBucket {
            bucket_start: i as f64 * 0.1,
            bucket_end: (i + 1) as f64 * 0.1,
            count: 0,
        })
        .collect()
}

// ── cohort management ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCohortBody {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCohortResponse {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddCohortMemberBody {
    pub user_id: Uuid,
}

pub struct CohortWriteScopes;
impl ScopeOneOf for CohortWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::Admin];
}

async fn create_cohort(
    State(state): State<AppState>,
    _auth: RequireAnyScope<CohortWriteScopes>,
    Json(body): Json<CreateCohortBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "name".into(),
            message: "must not be empty".into(),
        }]));
    }
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_cohorts (id, name, description) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(&body.name)
        .bind(&body.description)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateCohortResponse {
            id,
            name: body.name,
        }),
    ))
}

async fn add_cohort_member(
    State(state): State<AppState>,
    _auth: RequireAnyScope<CohortWriteScopes>,
    Path(cohort_id): Path<Uuid>,
    Json(body): Json<AddCohortMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    let exists: bool = sqlx::query_scalar("SELECT exists(SELECT 1 FROM tb_cohorts WHERE id = $1)")
        .bind(cohort_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    if !exists {
        return Err(ApiError::NotFound { resource: "cohort" });
    }

    sqlx::query(
        "INSERT INTO tb_cohort_memberships (id, cohort_id, user_id) VALUES ($1, $2, $3)
         ON CONFLICT (cohort_id, user_id) DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(cohort_id)
    .bind(body.user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/me", get(get_me))
        .route("/v1/me/keys", get(list_keys))
        .route("/v1/me/keys", post(create_key))
        .route("/v1/me/keys/{id}/rotate", post(rotate_key))
        .route("/v1/me/keys/{id}", delete(revoke_key))
        .route("/v1/me/agents", get(list_agents))
        .route("/v1/me/agents", post(create_agent))
        .route("/v1/me/agents/{id}", patch(update_agent))
        .route("/v1/me/agents/{id}", delete(delete_agent))
        .route("/v1/me/webhooks", get(list_webhooks))
        .route("/v1/me/webhooks", post(create_webhook))
        .route("/v1/me/webhooks/{id}", delete(delete_webhook))
        .route("/v1/me/attempts", get(list_attempts))
        .route("/v1/me/cohort-stats", get(get_cohort_stats))
        .route("/v1/cohorts", post(create_cohort))
        .route("/v1/cohorts/{id}/members", post(add_cohort_member))
        .with_state(state)
}
