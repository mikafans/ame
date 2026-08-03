//! Learner-created, revocable capabilities for an external course author.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        token::{TokenKind, format_token, generate_secret, hash_secret},
    },
    domain::error::{ApiError, FieldError},
    http::AppState,
};

const MINUTES_MIN: i64 = 15;
const MINUTES_MAX: i64 = 24 * 60;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDelegationBody {
    pub goal: String,
    pub expires_in_minutes: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DelegationResponse {
    pub id: Uuid,
    pub goal: String,
    pub scope: String,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub revoked_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDelegationResponse {
    pub delegation: DelegationResponse,
    /// Returned once only. This is a scoped capability, never a login token.
    pub handoff: String,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/agent-delegations", post(create).get(list))
        .route("/v1/agent-delegations/{id}", delete(revoke))
        .with_state(state)
}

#[utoipa::path(post, path = "/api/v1/agent-delegations", request_body = CreateDelegationBody, responses((status = 201, body = CreateDelegationResponse)), security(("bearer_auth" = [])), tag = "agent delegation")]
pub async fn create(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    headers: HeaderMap,
    Json(body): Json<CreateDelegationBody>,
) -> Result<(StatusCode, Json<CreateDelegationResponse>), ApiError> {
    require_learner(&auth)?;
    let goal = body.goal.trim();
    if goal.is_empty() {
        return Err(field("goal", "must not be empty"));
    }
    if !(MINUTES_MIN..=MINUTES_MAX).contains(&body.expires_in_minutes) {
        return Err(field("expiresInMinutes", "must be between 15 and 1440"));
    }
    let id = Uuid::now_v7();
    let actor_identity_id = Uuid::now_v7();
    let secret = generate_secret();
    let now = OffsetDateTime::now_utc();
    let expires_at = now + time::Duration::minutes(body.expires_in_minutes);
    let mut tx = state.pool.begin().await.map_err(db_error)?;
    sqlx::query("INSERT INTO tb_identities (id, identity_type, owner_user_id, label, metadata) VALUES ($1, 'agent', $2, 'Delegated course author', jsonb_build_object('delegationId', $3))")
        .bind(actor_identity_id)
        .bind(auth.owner_id())
        .bind(id)
        .execute(&mut *tx).await.map_err(db_error)?;
    let row = sqlx::query("INSERT INTO tb_agent_delegations (id, subject_user_id, actor_identity_id, token_hash, goal, expires_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, goal, scope, created_at, expires_at, revoked_at, last_used_at")
        .bind(id).bind(auth.owner_id()).bind(actor_identity_id).bind(hash_secret(&secret)).bind(goal).bind(expires_at)
        .fetch_one(&mut *tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    let token = format_token(TokenKind::Delegation, id, &secret);
    let origin = public_origin(&headers, state.config.server.public_base_url.as_deref())?;
    let handoff = format!(
        "Use AME to create one source-grounded course for this learner. Read {origin}/public/llms.txt first.\nAuthorization: Bearer {token}\nScope: course authoring only; expires {}. Goal: {goal}. Import and certify sources, create a complete course, validate, review, publish, then return /learning/journeys/{{journeyId}}. Do not ask for or use a learner login token.",
        expires_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    );
    Ok((
        StatusCode::CREATED,
        Json(CreateDelegationResponse {
            delegation: response(row),
            handoff,
        }),
    ))
}

fn public_origin(
    headers: &HeaderMap,
    configured_base_url: Option<&str>,
) -> Result<String, ApiError> {
    let configured = configured_base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let origin = configured
        .or_else(|| {
            let host = headers
                .get(header::HOST)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())?;
            let proto = headers
                .get("x-forwarded-proto")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .map(str::trim)
                .filter(|value| matches!(*value, "http" | "https"))
                .unwrap_or("http");
            Some(format!("{proto}://{host}"))
        })
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("request has no public origin")))?;

    let origin = origin.trim_end_matches('/');
    let authority = origin
        .split_once("://")
        .map(|(_, authority)| authority)
        .unwrap_or_default();
    if !(origin.starts_with("http://") || origin.starts_with("https://"))
        || origin.contains('\n')
        || origin.contains('\r')
        || authority.is_empty()
        || authority.contains('/')
    {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "configured public base URL must be an HTTP origin"
        )));
    }
    Ok(origin.to_owned())
}

#[cfg(test)]
mod tests {
    use super::public_origin;
    use axum::http::HeaderMap;

    #[test]
    fn configured_origin_takes_precedence() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "internal:8080".parse().unwrap());
        assert_eq!(
            public_origin(&headers, Some("https://ame.example.com/")).unwrap(),
            "https://ame.example.com"
        );
    }

    #[test]
    fn forwarded_request_uses_public_host_and_protocol() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "ame.example.com".parse().unwrap());
        headers.insert("x-forwarded-proto", "https, http".parse().unwrap());
        assert_eq!(
            public_origin(&headers, None).unwrap(),
            "https://ame.example.com"
        );
    }
}

#[utoipa::path(get, path = "/api/v1/agent-delegations", responses((status = 200, body = [DelegationResponse])), security(("bearer_auth" = [])), tag = "agent delegation")]
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<DelegationResponse>>, ApiError> {
    require_learner(&auth)?;
    let rows = sqlx::query("SELECT id, goal, scope, created_at, expires_at, revoked_at, last_used_at FROM tb_agent_delegations WHERE subject_user_id = $1 ORDER BY created_at DESC")
        .bind(auth.owner_id()).fetch_all(&state.pool).await.map_err(db_error)?;
    Ok(Json(rows.into_iter().map(response).collect()))
}

#[utoipa::path(delete, path = "/api/v1/agent-delegations/{id}", params(("id" = Uuid, Path)), responses((status = 204)), security(("bearer_auth" = [])), tag = "agent delegation")]
pub async fn revoke(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    require_learner(&auth)?;
    let result = sqlx::query("UPDATE tb_agent_delegations SET revoked_at = COALESCE(revoked_at, now()) WHERE id = $1 AND subject_user_id = $2")
        .bind(id).bind(auth.owner_id()).execute(&state.pool).await.map_err(db_error)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "agent delegation",
        });
    }
    Ok(StatusCode::NO_CONTENT)
}

fn require_learner(auth: &AuthenticatedUser) -> Result<(), ApiError> {
    if auth.is_delegated_course_author() {
        Err(ApiError::Forbidden(
            "delegated course authors cannot manage delegations".into(),
        ))
    } else {
        Ok(())
    }
}
fn response(row: sqlx::postgres::PgRow) -> DelegationResponse {
    DelegationResponse {
        id: row.get("id"),
        goal: row.get("goal"),
        scope: row.get("scope"),
        created_at: row.get("created_at"),
        expires_at: row.get("expires_at"),
        revoked_at: row.get("revoked_at"),
        last_used_at: row.get("last_used_at"),
    }
}
fn field(name: &str, message: &str) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: name.into(),
        message: message.into(),
    }])
}
fn db_error(error: sqlx::Error) -> ApiError {
    ApiError::Internal(error.into())
}
