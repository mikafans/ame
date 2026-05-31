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

        // Find token and user
        let record = sqlx::query(
            r#"
            SELECT 
                t.token_hash, t.scopes, t.revoked_at,
                u.id as user_id, u.email, u.display_name, u.role, u.created_at, u.owner_user_id,
                u.deactivated_at as user_deactivated_at,
                o.deactivated_at as owner_deactivated_at
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

        let revoked_at: Option<time::OffsetDateTime> = record.get("revoked_at");
        if revoked_at.is_some() {
            return Err(ApiError::Unauthorized);
        }

        let user_deactivated_at: Option<time::OffsetDateTime> = record.get("user_deactivated_at");
        let owner_deactivated_at: Option<time::OffsetDateTime> = record.get("owner_deactivated_at");
        if user_deactivated_at.is_some() || owner_deactivated_at.is_some() {
            return Err(ApiError::Unauthorized);
        }

        let token_hash: String = record.get("token_hash");
        if !verify_token_secret(&token_hash, &parsed.secret) {
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

        let role: String = record.get("role");
        let role = match role.as_str() {
            "user" => crate::domain::user::Role::User,
            "admin" => crate::domain::user::Role::Admin,
            "agent" => crate::domain::user::Role::Agent,
            _ => {
                return Err(ApiError::Internal(anyhow::anyhow!(
                    "unknown role in users.role: {role}"
                )));
            }
        };

        let user = User {
            id: record.get("user_id"),
            owner_user_id: record.get("owner_user_id"),
            email: record.get("email"),
            display_name: record.get("display_name"),
            role,
            created_at: record.get("created_at"),
        };
        let owner_id = user.owner_user_id.unwrap_or(user.id);

        // The DB CHECK constraint on api_tokens.scopes restricts values to the
        // three known wire strings. We still parse defensively: a row that
        // bypasses the constraint (manual SQL, dropped check, etc.) becomes a
        // 500 instead of a silently accepted unknown scope.
        let raw_scopes: Vec<String> = record.get("scopes");
        let token_scopes = raw_scopes
            .into_iter()
            .map(|s| Scope::from_str(&s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid scope in api_tokens: {e}")))?;

        Ok(AuthenticatedUser {
            user,
            token_scopes,
            owner_id,
        })
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
                created_at: now,
            },
            token_scopes: vec![],
            owner_id: user_id,
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
                created_at: now,
            },
            token_scopes: vec![],
            owner_id: user_id,
        };
        assert_eq!(agent.owner_id(), user_id);
    }
}
