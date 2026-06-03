use std::str::FromStr;

use crate::domain::user::{Scope, User};

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use sqlx::Row;

use crate::{domain::error::ApiError, http::AppState};

use super::token::{parse_bearer_token, parse_token_value, verify_token_secret};

use uuid::Uuid;

/// Authenticated request context produced by [`AuthenticatedUser::from_request_parts`].
///
/// `token_scopes` holds the parsed [`Scope`] enum (not raw strings) so downstream
/// code cannot accidentally accept a scope that drifts from the `api_tokens.scopes`
/// CHECK constraint in `db/migrations/20260519092355_init.sql`.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user: User,
    pub token_scopes: Vec<Scope>,
    pub owner_id: Uuid,
    pub owner_plan: String,
}

impl AuthenticatedUser {
    /// Returns the ID of the human owner this request acts on behalf of.
    ///
    /// For human users, this is their own ID. For agents, this is their
    /// owner sub-account link (the human who created the agent).
    /// Downstream code uses this for "my content" filters, quotas, and
    /// shared truth (level/progress) reads.
    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 0. Check if already extracted by middleware
        if let Some(auth) = parts.extensions.get::<Self>() {
            return Ok(auth.clone());
        }

        // Try to extract token from Authorization: Bearer header first,
        // then fall back to HttpOnly ame_token cookie.
        let parsed = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(parse_bearer_token)
            .or_else(|| {
                parts
                    .headers
                    .get(header::COOKIE)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|cookies| {
                        cookies.split(';').find_map(|c| {
                            let c = c.trim();
                            c.strip_prefix("ame_token=").map(|v| v.to_owned())
                        })
                    })
                    .and_then(|v| parse_token_value(&v))
            })
            .ok_or(ApiError::Unauthorized)?;

        // 1. Try to fetch from Valkey token cache
        let cache_key = format!("ame:token:{}", parsed.id);
        let mut cached_info: Option<CachedTokenInfo> = None;

        if let Ok(mut conn) = state.valkey.get().await {
            use redis::AsyncCommands;
            if let Some(info) = conn
                .get::<_, String>(&cache_key)
                .await
                .ok()
                .and_then(|val| serde_json::from_str::<CachedTokenInfo>(&val).ok())
            {
                cached_info = Some(info);
            }
        }

        // 2. Fall back to PostgreSQL if cache miss or error
        let info = if let Some(info) = cached_info {
            info
        } else {
            let record = sqlx::query(
                r#"
                SELECT 
                    t.token_hash, t.scopes, t.revoked_at, t.expires_at,
                    u.id as user_id, u.email, u.display_name, u.role, u.plan, u.created_at, u.owner_user_id,
                    u.deactivated_at as user_deactivated_at,
                    o.deactivated_at as owner_deactivated_at,
                    COALESCE(o.plan, u.plan) as owner_plan
                FROM tb_api_tokens t
                JOIN tb_users u ON t.user_id = u.id
                LEFT JOIN tb_users o ON u.owner_user_id = o.id
                WHERE t.id = $1
                "#,
            )
            .bind(parsed.id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                eprintln!("EXTRACTOR QUERY ERROR: {:?}", e);
                ApiError::Internal(e.into())
            })?
            .ok_or(ApiError::Unauthorized)?;

            let db_info = CachedTokenInfo {
                token_hash: record.get("token_hash"),
                scopes: record.get("scopes"),
                revoked_at: record.get("revoked_at"),
                expires_at: record.get("expires_at"),
                user_id: record.get("user_id"),
                email: record.get("email"),
                display_name: record.get("display_name"),
                role: record.get("role"),
                plan: record.get("plan"),
                created_at: record.get("created_at"),
                owner_user_id: record.get("owner_user_id"),
                user_deactivated_at: record.get("user_deactivated_at"),
                owner_deactivated_at: record.get("owner_deactivated_at"),
                owner_plan: record.get("owner_plan"),
            };

            // Attempt to cache in Valkey with 30s TTL
            if let Ok(mut conn) = state.valkey.get().await {
                use redis::AsyncCommands;
                if let Ok(serialized) = serde_json::to_string(&db_info) {
                    let _: Result<(), _> = conn.set_ex(&cache_key, serialized, 30).await;
                }
            }

            db_info
        };

        if info.revoked_at.is_some() || info.expires_at < time::OffsetDateTime::now_utc() {
            return Err(ApiError::Unauthorized);
        }

        if info.user_deactivated_at.is_some() || info.owner_deactivated_at.is_some() {
            return Err(ApiError::Unauthorized);
        }

        if !verify_token_secret(&info.token_hash, &parsed.secret) {
            return Err(ApiError::Unauthorized);
        }

        // Update last_used_at in background
        let pool = state.pool.clone();
        let token_id = parsed.id;
        tokio::spawn(async move {
            let _ = sqlx::query("UPDATE tb_api_tokens SET last_used_at = now() WHERE id = $1")
                .bind(token_id)
                .execute(&pool)
                .await;
        });

        let role = match info.role.as_str() {
            "user" => crate::domain::user::Role::User,
            "admin" => crate::domain::user::Role::Admin,
            "agent" => crate::domain::user::Role::Agent,
            _ => {
                return Err(ApiError::Internal(anyhow::anyhow!(
                    "unknown role in users.role: {}",
                    info.role
                )));
            }
        };

        let user = User {
            id: info.user_id,
            owner_user_id: info.owner_user_id,
            email: info.email,
            display_name: info.display_name,
            role,
            plan: info.plan,
            created_at: info.created_at,
        };
        let owner_id = user.owner_user_id.unwrap_or(user.id);

        let token_scopes = info
            .scopes
            .into_iter()
            .map(|s| Scope::from_str(&s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid scope in api_tokens: {e}")))?;

        Ok(AuthenticatedUser {
            user,
            token_scopes,
            owner_id,
            owner_plan: info.owner_plan,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedTokenInfo {
    pub token_hash: String,
    pub scopes: Vec<String>,
    pub revoked_at: Option<time::OffsetDateTime>,
    pub expires_at: time::OffsetDateTime,
    pub user_id: Uuid,
    pub email: Option<String>,
    pub display_name: String,
    pub role: String,
    pub plan: String,
    pub created_at: time::OffsetDateTime,
    pub owner_user_id: Option<Uuid>,
    pub user_deactivated_at: Option<time::OffsetDateTime>,
    pub owner_deactivated_at: Option<time::OffsetDateTime>,
    pub owner_plan: String,
}

pub async fn invalidate_token(valkey: &deadpool_redis::Pool, token_id: Uuid) {
    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        let cache_key = format!("ame:token:{}", token_id);
        let _: Result<(), _> = conn.del(&cache_key).await;
    }
}

pub async fn invalidate_user_tokens(
    pool: &sqlx::PgPool,
    valkey: &deadpool_redis::Pool,
    user_id: Uuid,
) {
    let token_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT t.id FROM tb_api_tokens t
        JOIN tb_users u ON t.user_id = u.id
        WHERE u.id = $1 OR u.owner_user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        for tid in token_ids {
            let cache_key = format!("ame:token:{}", tid);
            let _: Result<(), _> = conn.del(&cache_key).await;
        }
    }

    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        let limit_key = format!("ame:limiter:owner:{}", user_id);
        let _: Result<(), _> = conn.del(&limit_key).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::Role;
    use time::OffsetDateTime;

    #[test]
    fn test_authenticated_user_owner_id_resolution() {
        let user_id = Uuid::now_v7();
        let agent_id = Uuid::now_v7();
        let now = OffsetDateTime::now_utc();

        // Human user: owner_id == user.id
        let human = AuthenticatedUser {
            user: User {
                id: user_id,
                owner_user_id: None,
                email: Some("alice@example.com".into()),
                display_name: "Alice".into(),
                role: Role::User,
                plan: "free".into(),
                created_at: now,
            },
            token_scopes: vec![],
            owner_id: user_id,
            owner_plan: "free".into(),
        };
        assert_eq!(human.owner_id(), user_id);

        // Agent: owner_id == owner_user_id
        let agent = AuthenticatedUser {
            user: User {
                id: agent_id,
                owner_user_id: Some(user_id),
                email: None,
                display_name: "Agent 007".into(),
                role: Role::Agent,
                plan: "free".into(),
                created_at: now,
            },
            token_scopes: vec![],
            owner_id: user_id,
            owner_plan: "free".into(),
        };
        assert_eq!(agent.owner_id(), user_id);
    }
}
