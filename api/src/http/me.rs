//! Me-surface routes: profile, API keys, agents, activity, stats.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
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
    },
    domain::{
        attempt::Attempt,
        error::{ApiError, FieldError},
        user::{Role, Scope},
    },
    http::AppState,
};

// ── API keys ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeySummary {
    pub id: Uuid,
    pub name: String,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListKeysResponse {
    pub keys: Vec<KeySummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateKeyBody {
    pub name: String,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateKeyResponse {
    pub id: Uuid,
    pub secret: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RotateKeyResponse {
    pub secret: String,
}

// ── agents ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub id: Uuid,
    pub label: String,
    pub scopes: Vec<String>,
    pub focus_tags: Vec<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentSummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentBody {
    pub label: String,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub focus_tags: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentResponse {
    pub id: Uuid,
    pub api_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentBody {
    pub label: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub focus_tags: Option<Vec<String>>,
}

// ── webhooks ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebhookSummary {
    pub id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookSummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWebhookBody {
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookResponse {
    pub id: Uuid,
}

// ── handlers ──────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/me/keys",
    responses(
        (status = 200, description = "List of API keys", body = ListKeysResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn list_keys(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ListKeysResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, name, last_used_at, created_at FROM tb_api_tokens WHERE user_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC"
    )
    .bind(user.user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let keys = rows
        .into_iter()
        .map(|r| KeySummary {
            id: r.get("id"),
            name: r.get("name"),
            last_used_at: r.get("last_used_at"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListKeysResponse { keys }))
}

#[utoipa::path(
    post,
    path = "/v1/me/keys",
    request_body = CreateKeyBody,
    responses(
        (status = 201, description = "Key created", body = CreateKeyResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn create_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateKeyBody>,
) -> Result<(StatusCode, Json<CreateKeyResponse>), ApiError> {
    let effective_scopes: Vec<String> = body.scopes.unwrap_or_else(|| {
        vec![
            "assessment.read".into(),
            "assessment.write".into(),
            "attempt.read".into(),
            "attempt.write".into(),
            "stats.read".into(),
            "feedback.write".into(),
        ]
    });

    // Validate scopes
    if effective_scopes.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "scopes".into(),
            message: "must contain at least one scope".into(),
        }]));
    }
    for scope_str in &effective_scopes {
        if scope_str.parse::<crate::domain::user::Scope>().is_err() {
            return Err(ApiError::Validation(vec![FieldError {
                field: "scopes".into(),
                message: format!("unknown scope: {}", scope_str),
            }]));
        }
    }

    // Non-admin users may not create keys with the admin scope.
    if effective_scopes.iter().any(|s| s == "admin") && user.user.role != Role::Admin {
        return Err(ApiError::ScopeRequired("admin".into()));
    }

    let token_id = Uuid::now_v7();
    let secret = crate::auth::token::generate_secret();
    let hash = crate::auth::token::hash_secret(&secret);

    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(token_id)
    .bind(user.user.id)
    .bind(&body.name)
    .bind(&hash)
    .bind(effective_scopes)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateKeyResponse {
            id: token_id,
            secret: format!("{}_{}", token_id, secret),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/me/keys/{id}/rotate",
    params(("id" = Uuid, Path, description = "Key id")),
    responses(
        (status = 200, description = "Key rotated", body = RotateKeyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such key"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn rotate_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RotateKeyResponse>, ApiError> {
    let secret = crate::auth::token::generate_secret();
    let hash = crate::auth::token::hash_secret(&secret);

    let affected = sqlx::query(
        "UPDATE tb_api_tokens SET token_hash = $1 WHERE id = $2 AND user_id = $3 AND revoked_at IS NULL"
    )
    .bind(&hash)
    .bind(id)
    .bind(user.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "api_key",
        });
    }

    crate::auth::extractor::invalidate_token(&state.valkey, id).await;

    Ok(Json(RotateKeyResponse {
        secret: format!("{}_{}", id, secret),
    }))
}

#[utoipa::path(
    delete,
    path = "/v1/me/keys/{id}",
    params(("id" = Uuid, Path, description = "Key id")),
    responses(
        (status = 204, description = "Key revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such key"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn revoke_key(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let affected = sqlx::query(
        "UPDATE tb_api_tokens SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL"
    )
    .bind(id)
    .bind(user.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "api_key",
        });
    }

    crate::auth::extractor::invalidate_token(&state.valkey, id).await;

    Ok(StatusCode::NO_CONTENT)
}

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
        "SELECT a.id, a.label as display_name, t.scopes, a.focus_tags, t.last_used_at, a.created_at
         FROM tb_agents a
         JOIN tb_api_tokens t ON t.agent_id = a.id
         WHERE a.owner_user_id = $1 AND a.deactivated_at IS NULL AND t.revoked_at IS NULL",
    )
    .bind(user.user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let agents = rows
        .into_iter()
        .map(|r| AgentSummary {
            id: r.get("id"),
            label: r.get("display_name"),
            scopes: r.get("scopes"),
            focus_tags: r.get("focus_tags"),
            last_used_at: r.get("last_used_at"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListAgentsResponse { agents }))
}

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
) -> Result<(StatusCode, Json<CreateAgentResponse>), ApiError> {
    crate::http::quota::check_quota(
        &state.pool,
        Some(&state.valkey),
        &state.config,
        user.owner_id,
        crate::http::quota::QuotaKind::AgentCreation,
        1,
    )
    .await?;

    // Validate scopes
    if body.scopes.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "scopes".into(),
            message: "must contain at least one scope".into(),
        }]));
    }
    for scope_str in &body.scopes {
        if scope_str.parse::<crate::domain::user::Scope>().is_err() {
            return Err(ApiError::Validation(vec![FieldError {
                field: "scopes".into(),
                message: format!("unknown scope: {}", scope_str),
            }]));
        }
    }

    // Non-admin users may not create agents with the admin scope.
    if body.scopes.iter().any(|s| s == "admin") && user.user.role != Role::Admin {
        return Err(ApiError::ScopeRequired("admin".into()));
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let agent_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tb_agents (id, owner_user_id, label, focus_tags) VALUES ($1, $2, $3, $4)",
    )
    .bind(agent_id)
    .bind(user.user.id)
    .bind(&body.label)
    .bind(&body.focus_tags)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let token_id = Uuid::now_v7();
    let secret = crate::auth::token::generate_secret();
    let hash = crate::auth::token::hash_secret(&secret);

    sqlx::query(
        "INSERT INTO tb_api_tokens (id, agent_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(token_id)
    .bind(agent_id)
    .bind(&body.label)
    .bind(&hash)
    .bind(&body.scopes)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateAgentResponse {
            id: agent_id,
            api_key: format!("{}_{}", token_id, secret),
        }),
    ))
}

#[utoipa::path(
    patch,
    path = "/v1/me/agents/{id}",
    params(("id" = Uuid, Path, description = "Agent id")),
    request_body = UpdateAgentBody,
    responses(
        (status = 204, description = "Agent updated"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such agent"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn update_agent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateAgentBody>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // 1. Verify existence and ownership
    let exists = sqlx::query(
        "SELECT 1 FROM tb_agents WHERE id = $1 AND owner_user_id = $2 AND deactivated_at IS NULL",
    )
    .bind(id)
    .bind(user.user.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if exists.is_none() {
        return Err(ApiError::NotFound { resource: "agent" });
    }

    if let Some(label) = &body.label {
        sqlx::query("UPDATE tb_agents SET label = $1 WHERE id = $2 AND owner_user_id = $3")
            .bind(label)
            .bind(id)
            .bind(user.user.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
    }

    if let Some(tags) = &body.focus_tags {
        sqlx::query(
            "UPDATE tb_agents \
             SET focus_tags = $1 \
             WHERE id = $2 \
               AND owner_user_id = $3 \
               AND deactivated_at IS NULL",
        )
        .bind(tags)
        .bind(id)
        .bind(user.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    if let Some(scopes) = &body.scopes {
        // Validate scopes
        if scopes.is_empty() {
            return Err(ApiError::Validation(vec![FieldError {
                field: "scopes".into(),
                message: "must contain at least one scope".into(),
            }]));
        }
        for scope_str in scopes {
            if scope_str.parse::<crate::domain::user::Scope>().is_err() {
                return Err(ApiError::Validation(vec![FieldError {
                    field: "scopes".into(),
                    message: format!("unknown scope: {}", scope_str),
                }]));
            }
        }

        // Non-admin users may not assign the admin scope to agents.
        if scopes.iter().any(|s| s == "admin") && user.user.role != Role::Admin {
            return Err(ApiError::ScopeRequired("admin".into()));
        }
        sqlx::query(
            "UPDATE tb_api_tokens \
             SET scopes = $1 \
             WHERE agent_id = $2 \
               AND EXISTS (SELECT 1 FROM tb_agents WHERE id = $2 AND owner_user_id = $3 AND deactivated_at IS NULL)"
        )
        .bind(scopes)
        .bind(id)
        .bind(user.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    crate::auth::extractor::invalidate_user_tokens(&state.pool, &state.valkey, id).await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/v1/me/agents/{id}",
    params(("id" = Uuid, Path, description = "Agent id")),
    responses(
        (status = 204, description = "Agent deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such agent"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn delete_agent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let token_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM tb_api_tokens WHERE agent_id = $1")
            .bind(id)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let is_admin = user.user.role == Role::Admin;
    crate::http::db::set_rls_guc(&mut tx, user.owner_id, is_admin).await?;

    // 1. Deactivate the agent record
    let affected =
        sqlx::query("UPDATE tb_agents SET deactivated_at = now() WHERE id = $1 AND owner_user_id = $2 AND deactivated_at IS NULL")
            .bind(id)
            .bind(user.user.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound { resource: "agent" });
    }

    // 2. Revoke all active tokens for this agent
    sqlx::query(
        "UPDATE tb_api_tokens SET revoked_at = now() WHERE agent_id = $1 AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    if let Ok(mut conn) = state.valkey.get().await {
        use redis::AsyncCommands;
        for tid in token_ids {
            let cache_key = format!("ame:token:{}", tid);
            let _: Result<(), _> = conn.del(&cache_key).await;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/v1/me/webhooks",
    responses(
        (status = 200, description = "List of webhooks", body = ListWebhooksResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn list_webhooks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ListWebhooksResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, url, events, is_active, created_at FROM tb_webhooks WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(user.user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let webhooks = rows
        .into_iter()
        .map(|r| WebhookSummary {
            id: r.get("id"),
            url: r.get("url"),
            events: r.get("events"),
            is_active: r.get("is_active"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListWebhooksResponse { webhooks }))
}

#[utoipa::path(
    post,
    path = "/v1/me/webhooks",
    request_body = CreateWebhookBody,
    responses(
        (status = 201, description = "Webhook created", body = CreateWebhookResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn create_webhook(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateWebhookBody>,
) -> Result<(StatusCode, Json<CreateWebhookResponse>), ApiError> {
    let id = Uuid::now_v7();
    let secret = body
        .secret
        .unwrap_or_else(crate::auth::token::generate_secret);
    let secret_hash = crate::auth::token::hash_secret(&secret);

    sqlx::query(
        "INSERT INTO tb_webhooks (id, user_id, url, events, secret_hash, signing_key) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(user.user.id)
    .bind(&body.url)
    .bind(&body.events)
    .bind(&secret_hash)
    .bind(&secret)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((StatusCode::CREATED, Json(CreateWebhookResponse { id })))
}

#[utoipa::path(
    delete,
    path = "/v1/me/webhooks/{id}",
    params(("id" = Uuid, Path, description = "Webhook id")),
    responses(
        (status = 204, description = "Webhook deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such webhook"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn delete_webhook(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let affected = sqlx::query("DELETE FROM tb_webhooks WHERE id = $1 AND user_id = $2")
        .bind(id)
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
    #[serde(with = "time::serde::rfc3339")]
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
pub async fn get_me(auth: AuthenticatedUser) -> Result<Json<MeResponse>, ApiError> {
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
pub async fn list_attempts(
    State(state): State<AppState>,
    auth: RequireAnyScope<AttemptReadScopes>,
    Query(q): Query<ListAttemptsQuery>,
) -> Result<Json<ListAttemptsResponse>, ApiError> {
    let limit = q.limit.clamp(1, 50);
    let rows = if let Some(sid) = q.session_id {
        sqlx::query(
            "SELECT id, user_id, question_id, question_version, session_id, response,
                    presentation, is_correct, score, grade_status, correct_answer, grader_notes, time_to_answer_ms,
                    rating_before_user_avg, rating_before_question,
                    user_tag_deltas, question_delta, created_at,
                    COUNT(*) OVER() AS total
             FROM tb_attempts
             WHERE user_id = $1 AND session_id = $2
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4",
        )
        // Owner-scoped: an agent sub-account reads its owner's attempts
        // (owner_id == user.id for a human, so their view is unchanged).
        .bind(auth.0.owner_id)
        .bind(sid)
        .bind(limit)
        .bind(q.offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
    } else {
        sqlx::query(
            "SELECT id, user_id, question_id, question_version, session_id, response,
                    presentation, is_correct, score, grade_status, correct_answer, grader_notes, time_to_answer_ms,
                    rating_before_user_avg, rating_before_question,
                    user_tag_deltas, question_delta, created_at,
                    COUNT(*) OVER() AS total
             FROM tb_attempts
             WHERE user_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        // Owner-scoped: see the session_id branch above.
        .bind(auth.0.owner_id)
        .bind(limit)
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
            correct_answer: r.get("correct_answer"),
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
    pub assessment_id: Option<Uuid>,
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
        ("assessmentId" = Option<Uuid>, Query, description = "Assessment to compare against"),
    ),
    responses(
        (status = 200, description = "Cohort stats", body = CohortStatsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn get_cohort_stats(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(q): Query<CohortStatsQuery>,
) -> Result<Json<CohortStatsResponse>, ApiError> {
    let assessment_id = q.assessment_id.ok_or_else(|| {
        ApiError::Validation(vec![FieldError {
            field: "assessmentId".into(),
            message: "required".into(),
        }])
    })?;
    // Owner-scoped so an agent token resolves the owner's cohort/percentile.
    let user_id = auth.owner_id;

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
         WHERE s.assessment_id = $1
           AND s.status = 'finished'
           AND s.result IS NOT NULL
           AND s.user_id IN (
               SELECT cm.user_id FROM tb_cohort_memberships cm WHERE cm.cohort_id = $2
           )",
    )
    .bind(assessment_id)
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
         WHERE assessment_id = $1
           AND user_id = $2
           AND status = 'finished'
           AND result IS NOT NULL
         ORDER BY (result->>'percent')::double precision DESC
         LIMIT 1",
    )
    .bind(assessment_id)
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
                WHERE s.assessment_id = $1
                  AND s.status = 'finished'
                  AND s.result IS NOT NULL
                  AND s.user_id IN (
                      SELECT cm.user_id FROM tb_cohort_memberships cm WHERE cm.cohort_id = $2
                  )
                ORDER BY s.user_id, (s.result->>'percent')::double precision DESC
             ) ranked
             WHERE ranked.best_score <= $3",
        )
        .bind(assessment_id)
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
         WHERE s.assessment_id = $1
           AND s.status = 'finished'
           AND s.result IS NOT NULL
           AND s.user_id IN (
               SELECT cm.user_id FROM tb_cohort_memberships cm WHERE cm.cohort_id = $2
           )
         GROUP BY 1
         ORDER BY 1",
    )
    .bind(assessment_id)
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
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCohortResponse {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddCohortMemberBody {
    pub user_id: Uuid,
}

pub struct CohortWriteScopes;
impl ScopeOneOf for CohortWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::Admin];
}

#[utoipa::path(
    post,
    path = "/v1/cohorts",
    request_body = CreateCohortBody,
    responses(
        (status = 201, description = "Cohort created", body = CreateCohortResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn create_cohort(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(body): Json<CreateCohortBody>,
) -> Result<(StatusCode, Json<CreateCohortResponse>), ApiError> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_cohorts (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(&body.name)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((StatusCode::CREATED, Json(CreateCohortResponse { id })))
}

#[utoipa::path(
    post,
    path = "/v1/cohorts/{id}/members",
    params(("id" = Uuid, Path, description = "Cohort id")),
    request_body = AddCohortMemberBody,
    responses(
        (status = 204, description = "Member added"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "me"
)]
pub async fn add_cohort_member(
    State(state): State<AppState>,
    _auth: RequireAnyScope<CohortWriteScopes>,
    Path(id): Path<Uuid>,
    Json(body): Json<AddCohortMemberBody>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("INSERT INTO tb_cohort_memberships (cohort_id, user_id) VALUES ($1, $2)")
        .bind(id)
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
