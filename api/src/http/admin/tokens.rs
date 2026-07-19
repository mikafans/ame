use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::scope::RequireScope, domain::error::ApiError, http::AppState};

use super::AdminScope;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenEntry {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    // tb_users.email is nullable (agent sub-accounts may have none), so this
    // must be optional — decoding a NULL into String would panic the handler.
    pub owner_email: Option<String>,
    pub owner_display_name: Option<String>,
    pub owner_role: String,
    pub status: String,
    pub scopes: Vec<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub revoked_at: Option<time::OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub expires_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListTokensResponse {
    pub tokens: Vec<TokenEntry>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ListTokensQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub q: Option<String>,
    pub status: Option<String>,
    pub role: Option<String>,
    pub owner_id: Option<Uuid>,
}

/// GET /v1/admin/tokens — list all API tokens with filtering
#[utoipa::path(
    get,
    path = "/v1/admin/tokens",
    params(ListTokensQuery),
    responses(
        (status = 200, description = "Token list successfully retrieved", body = ListTokensResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn list_tokens(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
    Query(query): Query<ListTokensQuery>,
) -> Result<Json<ListTokensResponse>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * page_size;

    // 1. Get total count
    let mut count_qb = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM tb_api_tokens t \
         JOIN tb_agents a ON a.id = t.agent_id \
         JOIN tb_identities i ON i.id = a.id \
         JOIN tb_users o ON a.owner_user_id = o.id",
    );
    let mut has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        count_qb.push(" WHERE (t.name ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR o.email ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR i.label ILIKE ");
        count_qb.push_bind(search_pat);
        count_qb.push(")");
        has_where = true;
    }

    if let Some(role) = query.role.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb.push(" AND 'agent' = ");
        } else {
            count_qb.push(" WHERE 'agent' = ");
            has_where = true;
        }
        count_qb.push_bind(role);
    }

    if let Some(owner_id) = query.owner_id {
        if has_where {
            count_qb.push(" AND t.agent_id = ");
        } else {
            count_qb.push(" WHERE t.agent_id = ");
            has_where = true;
        }
        count_qb.push_bind(owner_id);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb.push(" AND ");
        } else {
            count_qb.push(" WHERE ");
        }
        match status {
            "active" => {
                count_qb.push("(t.revoked_at IS NULL AND t.expires_at > now())");
            }
            "revoked" => {
                count_qb.push("(t.revoked_at IS NOT NULL)");
            }
            "expired" => {
                count_qb.push("(t.revoked_at IS NULL AND t.expires_at <= now())");
            }
            _ => {} // ignore invalid status
        }
    }

    let total: i64 = count_qb
        .build_query_as::<(i64,)>()
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .0;

    // 2. Get paginated tokens
    let mut qb = sqlx::QueryBuilder::new(
        "SELECT t.id, t.name, t.agent_id as user_id, \
         o.email as email, i.label as display_name, 'agent' as role, \
         t.scopes, t.last_used_at, t.revoked_at, t.expires_at, t.created_at \
         FROM tb_api_tokens t \
         JOIN tb_agents a ON a.id = t.agent_id \
         JOIN tb_identities i ON i.id = a.id \
         JOIN tb_users o ON a.owner_user_id = o.id",
    );

    has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        qb.push(" WHERE (t.name ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR o.email ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR i.label ILIKE ");
        qb.push_bind(search_pat);
        qb.push(")");
        has_where = true;
    }

    if let Some(role) = query.role.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND 'agent' = ");
        } else {
            qb.push(" WHERE 'agent' = ");
            has_where = true;
        }
        qb.push_bind(role);
    }

    if let Some(owner_id) = query.owner_id {
        if has_where {
            qb.push(" AND t.agent_id = ");
        } else {
            qb.push(" WHERE t.agent_id = ");
            has_where = true;
        }
        qb.push_bind(owner_id);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND ");
        } else {
            qb.push(" WHERE ");
        }
        match status {
            "active" => {
                qb.push("(t.revoked_at IS NULL AND t.expires_at > now())");
            }
            "revoked" => {
                qb.push("(t.revoked_at IS NOT NULL)");
            }
            "expired" => {
                qb.push("(t.revoked_at IS NULL AND t.expires_at <= now())");
            }
            _ => {} // ignore invalid status
        }
    }

    qb.push(" ORDER BY t.created_at DESC, t.id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let now = time::OffsetDateTime::now_utc();
    let tokens = rows
        .iter()
        .map(|r| {
            let role_str: String = r.get("role");
            let scopes: Vec<String> = r.get("scopes");
            let revoked_at: Option<time::OffsetDateTime> = r.get("revoked_at");
            let expires_at: time::OffsetDateTime = r.get("expires_at");
            let status = if revoked_at.is_some() {
                "revoked".to_string()
            } else if expires_at <= now {
                "expired".to_string()
            } else {
                "active".to_string()
            };

            TokenEntry {
                id: r.get("id"),
                name: r.get("name"),
                owner_id: r.get("user_id"),
                owner_email: r.get("email"),
                owner_display_name: r.get("display_name"),
                owner_role: role_str,
                status,
                scopes,
                last_used_at: r.get("last_used_at"),
                revoked_at,
                expires_at,
                created_at: r.get("created_at"),
            }
        })
        .collect();

    Ok(Json(ListTokensResponse {
        tokens,
        total,
        page,
        page_size,
    }))
}

/// DELETE /v1/admin/tokens/{id} — revoke an API token (idempotent)
#[utoipa::path(
    delete,
    path = "/v1/admin/tokens/{id}",
    params(("id" = Uuid, Path, description = "Token ID")),
    responses(
        (status = 204, description = "Token revoked successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "Token not found"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn delete_token_admin(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. Fetch token details (owner, name, status) for auditing and existence check.
    let token_info: Option<(Uuid, String, Option<time::OffsetDateTime>)> = sqlx::query_as(
        "SELECT agent_id as owner_id, name, revoked_at \
         FROM tb_api_tokens WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let (owner_id, name, revoked_at) = match token_info {
        Some(info) => info,
        None => {
            return Err(ApiError::NotFound {
                resource: "api_token",
            });
        }
    };

    // If it's already revoked, return 204 immediately (idempotent)
    if revoked_at.is_some() {
        return Ok(StatusCode::NO_CONTENT);
    }

    // Revoke the token
    sqlx::query("UPDATE tb_api_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Bust the Valkey token cache so the revoke takes effect immediately,
    // not after the 30s cache TTL — otherwise a leaked token keeps working.
    crate::auth::extractor::invalidate_token(&state.valkey, id).await;

    crate::audit::audit(
        state.pool.clone(),
        Some(admin.0.user.id),
        "token.revoke_admin",
        Some("api_token"),
        Some(id),
        serde_json::json!({
            "owner_id": owner_id,
            "name": name,
        }),
    );

    Ok(StatusCode::NO_CONTENT)
}
