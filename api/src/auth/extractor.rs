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

        let mut info_opt: Option<CachedTokenInfo> = None;
        let mut is_login = false;

        // 1. Try cache `ame:login:{id}`
        let login_key = format!("ame:login:{}", parsed.id);
        if let Ok(mut conn) = state.valkey.get().await {
            use redis::AsyncCommands;
            if let Some(info) = conn
                .get::<_, String>(&login_key)
                .await
                .ok()
                .and_then(|val| serde_json::from_str::<CachedTokenInfo>(&val).ok())
            {
                info_opt = Some(info);
                is_login = true;
            }
        }

        // 2. Else try cache `ame:token:{id}`
        let token_cache_key = format!("ame:token:{}", parsed.id);
        if info_opt.is_none()
            && let Ok(mut conn) = state.valkey.get().await
        {
            use redis::AsyncCommands;
            if let Some(info) = conn
                .get::<_, String>(&token_cache_key)
                .await
                .ok()
                .and_then(|val| serde_json::from_str::<CachedTokenInfo>(&val).ok())
            {
                info_opt = Some(info);
                is_login = false;
            }
        }

        // 3. Else PG `tb_login_sessions` by id. A login session is always a
        // top-level human (only agents have an owner, and agents authenticate via
        // tb_api_tokens, never login sessions) — so owner == self: owner_user_id is
        // NULL, owner_plan is the user's own plan, and there is no owner to deactivate.
        if info_opt.is_none() {
            let record = sqlx::query(
                r#"
                SELECT s.token_hash, s.scopes, s.expires_at, s.user_id,
                       u.email, u.display_name, u.role, u.plan, u.created_at,
                       u.deactivated_at AS user_deactivated_at
                FROM tb_login_sessions s
                JOIN tb_users u ON s.user_id = u.id
                WHERE s.id = $1
                "#,
            )
            .bind(parsed.id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

            if let Some(row) = record {
                let plan: String = row.get("plan");
                let db_info = CachedTokenInfo {
                    token_hash: row.get("token_hash"),
                    scopes: row.get("scopes"),
                    revoked_at: None,
                    expires_at: row.get("expires_at"),
                    user_id: row.get("user_id"),
                    email: row.get("email"),
                    display_name: row.get("display_name"),
                    role: row.get("role"),
                    plan: plan.clone(),
                    created_at: row.get("created_at"),
                    owner_user_id: None,
                    user_deactivated_at: row.get("user_deactivated_at"),
                    owner_deactivated_at: None,
                    owner_plan: plan,
                };

                // Warm cache (fail-open)
                if let Ok(mut conn) = state.valkey.get().await {
                    use redis::AsyncCommands;
                    if let Ok(serialized) = serde_json::to_string(&db_info) {
                        let ttl = state.config.login.ttl_seconds;
                        let _: Result<(), _> = conn.set_ex(&login_key, serialized, ttl).await;
                    }
                }

                info_opt = Some(db_info);
                is_login = true;
            }
        }

        // 4. Else PG `tb_api_tokens` (existing query)
        let info = if let Some(info) = info_opt {
            info
        } else {
            let record = sqlx::query(
                r#"
                SELECT 
                    t.token_hash, t.scopes, t.revoked_at, t.expires_at,
                    COALESCE(t.user_id, t.agent_id) as user_id,
                    CASE WHEN t.agent_id IS NOT NULL THEN NULL ELSE u.email END as email,
                    CASE WHEN t.agent_id IS NOT NULL THEN a.label ELSE u.display_name END as display_name,
                    CASE WHEN t.agent_id IS NOT NULL THEN 'agent' ELSE u.role END as role,
                    CASE WHEN t.agent_id IS NOT NULL THEN COALESCE(o.plan, 'free') ELSE u.plan END as plan,
                    CASE WHEN t.agent_id IS NOT NULL THEN a.created_at ELSE u.created_at END as created_at,
                    CASE WHEN t.agent_id IS NOT NULL THEN a.owner_user_id ELSE u.owner_user_id END as owner_user_id,
                    CASE WHEN t.agent_id IS NOT NULL THEN a.deactivated_at ELSE u.deactivated_at END as user_deactivated_at,
                    o.deactivated_at as owner_deactivated_at,
                    COALESCE(o.plan, u.plan) as owner_plan
                FROM tb_api_tokens t
                LEFT JOIN tb_users u ON t.user_id = u.id
                LEFT JOIN tb_agents a ON t.agent_id = a.id
                LEFT JOIN tb_users o ON COALESCE(u.owner_user_id, a.owner_user_id) = o.id
                WHERE t.id = $1
                "#,
            )
            .bind(parsed.id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
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
                    let _: Result<(), _> = conn.set_ex(&token_cache_key, serialized, 30).await;
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

        if is_login {
            let now = time::OffsetDateTime::now_utc();
            let ttl = state.config.login.ttl_seconds;
            let refresh_window = (ttl / 4).clamp(1, 60) as i64;
            let due = now
                >= info.expires_at - time::Duration::seconds(ttl as i64)
                    + time::Duration::seconds(refresh_window);
            if due {
                let new_expires = now + time::Duration::seconds(ttl as i64);
                // Update cache blob inline (fail-open) so the next request sees the slide.
                if let Ok(mut conn) = state.valkey.get().await {
                    use redis::AsyncCommands;
                    let mut refreshed = info.clone();
                    refreshed.expires_at = new_expires;
                    if let Ok(serialized) = serde_json::to_string(&refreshed) {
                        let _: Result<(), _> = conn
                            .set_ex(format!("ame:login:{}", parsed.id), serialized, ttl)
                            .await;
                    }
                }
                // Persist to PG in the background (durability / cross-replica truth).
                let pool = state.pool.clone();
                let id = parsed.id;
                tokio::spawn(async move {
                    let _ =
                        sqlx::query("UPDATE tb_login_sessions SET expires_at = $2 WHERE id = $1")
                            .bind(id)
                            .bind(new_expires)
                            .execute(&pool)
                            .await;
                });
            }
        } else {
            // Update last_used_at in the background, throttled to at most once per
            // minute per token. Without the staleness guard, every request from a
            // single token contends on that one row's lock (concurrent requests
            // serialize and pile up — observed as multi-second waits on a PK
            // update). The guard makes all but the first writer in each window a
            // no-op, so they take no lasting lock.
            let pool = state.pool.clone();
            let token_id = parsed.id;
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE tb_api_tokens SET last_used_at = now() \
                     WHERE id = $1 \
                       AND (last_used_at IS NULL OR last_used_at < now() - interval '1 minute')",
                )
                .bind(token_id)
                .execute(&pool)
                .await;
            });
        }

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
            deactivated_at: info.user_deactivated_at,
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
    // 1. Get database-backed token IDs
    let token_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT t.id FROM tb_api_tokens t
        LEFT JOIN tb_agents a ON t.agent_id = a.id
        WHERE t.user_id = $1 OR a.owner_user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 2. Login sessions for this user: collect ids, delete rows, drop caches.
    let login_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM tb_login_sessions WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

    let _ = sqlx::query("DELETE FROM tb_login_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;

    // 3. Delete DB tokens from cache & login tokens from Valkey
    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        for tid in token_ids {
            let cache_key = format!("ame:token:{}", tid);
            let _: Result<(), _> = conn.del(&cache_key).await;
        }
        for lid in login_ids {
            let login_key = format!("ame:login:{}", lid);
            let _: Result<(), _> = conn.del(&login_key).await;
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
                deactivated_at: None,
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
                deactivated_at: None,
            },
            token_scopes: vec![],
            owner_id: user_id,
            owner_plan: "free".into(),
        };
        assert_eq!(agent.owner_id(), user_id);
    }
}
