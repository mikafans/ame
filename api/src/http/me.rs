//! Profile, key and webhook management routes: /v1/me, /v1/me/keys, /v1/me/webhooks, /v1/me/attempts.

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use rand::rngs::OsRng;
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
         FROM api_tokens
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
    let hash = hash_secret(&secret)?;
    let prefix = format!("hk_{}", &secret[..8]);

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
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
        "SELECT id, name, scopes FROM api_tokens
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
    sqlx::query("UPDATE api_tokens SET revoked_at = now() WHERE id = $1")
        .bind(key_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Issue new key with same name and scopes
    let new_token_id = Uuid::now_v7();
    let new_secret = generate_secret();
    let new_hash = hash_secret(&new_secret)?;

    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4, $5)",
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
        "UPDATE api_tokens SET revoked_at = now()
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

// ── webhook handlers ──────────────────────────────────────────────────────────

pub async fn list_webhooks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ListWebhooksResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, url, events, created_at
         FROM webhooks
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
    let hash = hash_secret(&secret)?;

    sqlx::query(
        "INSERT INTO webhooks (id, user_id, url, events, secret_hash, signing_key)
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
        "UPDATE webhooks SET revoked_at = now()
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

// ── helpers ───────────────────────────────────────────────────────────────────

pub fn generate_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_secret(secret: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("argon2 hash failed: {e}")))
}

// ── profile ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
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
                    presentation, is_correct, score, time_to_answer_ms,
                    rating_before_user_avg, rating_before_question,
                    user_tag_deltas, question_delta, created_at,
                    COUNT(*) OVER() AS total
             FROM attempts
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
                    presentation, is_correct, score, time_to_answer_ms,
                    rating_before_user_avg, rating_before_question,
                    user_tag_deltas, question_delta, created_at,
                    COUNT(*) OVER() AS total
             FROM attempts
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

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/me", get(get_me))
        .route("/v1/me/keys", get(list_keys))
        .route("/v1/me/keys", post(create_key))
        .route("/v1/me/keys/{id}/rotate", post(rotate_key))
        .route("/v1/me/keys/{id}", delete(revoke_key))
        .route("/v1/me/webhooks", get(list_webhooks))
        .route("/v1/me/webhooks", post(create_webhook))
        .route("/v1/me/webhooks/{id}", delete(delete_webhook))
        .route("/v1/me/attempts", get(list_attempts))
        .with_state(state)
}
