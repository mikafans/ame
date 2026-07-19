use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::{
    auth::token::{TokenKind, parse_bearer_token, parse_token_value},
    authentication_postgres::PgAuthenticationRepository,
    domain::{
        auth::{AuthenticatedPrincipal, AuthenticationRepository},
        error::ApiError,
        user::Role,
    },
    http::AppState,
};

/// Clean request-facing projection of the unified authentication principal.
///
/// This is a view for existing handlers, not a second credential model. The
/// repository is the sole authority for credential verification and joins.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user: AuthenticatedUserView,
    pub token_scopes: Vec<crate::domain::user::Scope>,
    pub owner_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUserView {
    pub id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub email: Option<String>,
    pub display_name: String,
    pub role: Role,
    pub created_at: time::OffsetDateTime,
}

impl AuthenticatedUser {
    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    pub fn is_agent(&self) -> bool {
        matches!(self.user.role, Role::Agent)
    }

    fn from_principal(principal: AuthenticatedPrincipal) -> Self {
        let role = match principal.kind {
            crate::domain::auth::PrincipalKind::Human => match principal.role {
                crate::domain::auth::PrincipalRole::Learner => Role::User,
                crate::domain::auth::PrincipalRole::Admin => Role::Admin,
            },
            crate::domain::auth::PrincipalKind::Agent => Role::Agent,
        };
        Self {
            user: AuthenticatedUserView {
                id: principal.actor_identity_id,
                owner_user_id: (principal.kind == crate::domain::auth::PrincipalKind::Agent)
                    .then_some(principal.owner_user_id),
                email: principal.email,
                display_name: principal.display_name,
                role,
                created_at: principal.created_at,
            },
            token_scopes: principal.scopes,
            owner_id: principal.owner_user_id,
        }
    }
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(auth) = parts.extensions.get::<Self>() {
            return Ok(auth.clone());
        }

        let parsed = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_bearer_token)
            .or_else(|| {
                parts
                    .headers
                    .get(header::COOKIE)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|cookies| {
                        cookies.split(';').find_map(|cookie| {
                            cookie.trim().strip_prefix("ame_token=").map(str::to_owned)
                        })
                    })
                    .and_then(|value| parse_token_value(&value))
            })
            .ok_or(ApiError::Unauthorized)?;

        let now = time::OffsetDateTime::now_utc();
        let repository = PgAuthenticationRepository::new(state.pool.clone());
        let principal = match parsed.kind {
            TokenKind::Login => {
                repository
                    .authenticate_login_session(parsed.id, &parsed.secret, now)
                    .await
            }
            TokenKind::Agent => {
                repository
                    .authenticate_api_token(parsed.id, &parsed.secret, now)
                    .await
            }
        }
        .map_err(|error| match error {
            crate::domain::auth::AuthenticationRepositoryError::Storage(error) => {
                ApiError::Internal(anyhow::anyhow!(error))
            }
            _ => ApiError::Unauthorized,
        })?;

        Ok(Self::from_principal(principal))
    }
}

pub async fn invalidate_token(valkey: &deadpool_redis::Pool, token_id: Uuid) {
    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        let _: Result<(), _> = conn.del(format!("ame:token:{token_id}")).await;
    }
}

pub async fn invalidate_user_caches(
    pool: &sqlx::PgPool,
    valkey: &deadpool_redis::Pool,
    user_id: Uuid,
) {
    let token_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT t.id FROM tb_api_tokens t JOIN tb_agents a ON t.agent_id = a.id WHERE a.owner_user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let login_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM tb_login_sessions WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        for id in token_ids {
            let _: Result<(), _> = conn.del(format!("ame:token:{id}")).await;
        }
        for id in login_ids {
            let _: Result<(), _> = conn.del(format!("ame:login:{id}")).await;
        }
        let _: Result<(), _> = conn.del(format!("ame:limiter:owner:{user_id}")).await;
    }
}

pub async fn invalidate_user_tokens(
    pool: &sqlx::PgPool,
    valkey: &deadpool_redis::Pool,
    user_id: Uuid,
) {
    let token_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT t.id FROM tb_api_tokens t JOIN tb_agents a ON t.agent_id = a.id WHERE a.owner_user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
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
    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        for id in token_ids {
            let _: Result<(), _> = conn.del(format!("ame:token:{id}")).await;
        }
        for id in login_ids {
            let _: Result<(), _> = conn.del(format!("ame:login:{id}")).await;
        }
        let _: Result<(), _> = conn.del(format!("ame:limiter:owner:{user_id}")).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::{PrincipalKind, PrincipalRole};

    #[test]
    fn principal_projection_keeps_human_and_agent_ownership_distinct() {
        let owner_id = Uuid::now_v7();
        let principal = AuthenticatedPrincipal {
            actor_identity_id: Uuid::now_v7(),
            owner_user_id: owner_id,
            kind: PrincipalKind::Agent,
            role: PrincipalRole::Learner,
            email: None,
            display_name: "Tutor agent".into(),
            created_at: time::OffsetDateTime::now_utc(),
            scopes: vec![],
            credential_id: Uuid::now_v7(),
            expires_at: time::OffsetDateTime::now_utc(),
        };
        let auth = AuthenticatedUser::from_principal(principal);
        assert_eq!(auth.owner_id(), owner_id);
        assert!(auth.is_agent());
        assert_eq!(auth.user.owner_user_id, Some(owner_id));
    }
}
