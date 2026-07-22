use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::{
    auth::token::{TokenKind, parse_bearer_token, parse_token_value},
    authentication_postgres::PgAuthenticationRepository,
    domain::{auth::AuthenticationRepository, error::ApiError},
    http::AppState,
};

/// The authenticated learner projection used by HTTP handlers.
///
/// There is deliberately no delegated-agent identity or per-client scope
/// list. The authenticated user is also the owner of learner resources.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user: AuthenticatedUserView,
    pub owner_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUserView {
    pub id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub email: Option<String>,
    pub display_name: String,
    pub role: crate::domain::user::Role,
    pub created_at: time::OffsetDateTime,
}

impl AuthenticatedUser {
    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    fn from_principal(principal: crate::domain::auth::AuthenticatedPrincipal) -> Self {
        let role = match principal.role {
            crate::domain::auth::PrincipalRole::Learner => crate::domain::user::Role::User,
            crate::domain::auth::PrincipalRole::Admin => crate::domain::user::Role::Admin,
        };
        Self {
            user: AuthenticatedUserView {
                id: principal.user_id,
                owner_user_id: None,
                email: principal.email,
                display_name: principal.display_name,
                role,
                created_at: principal.created_at,
            },
            owner_id: principal.user_id,
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
            .filter(|token| token.kind == TokenKind::Login)
            .ok_or(ApiError::Unauthorized)?;

        let principal = PgAuthenticationRepository::new(state.pool.clone())
            .authenticate_login_session(parsed.id, &parsed.secret, time::OffsetDateTime::now_utc())
            .await
            .map_err(|error| match error {
                crate::domain::auth::AuthenticationRepositoryError::Storage(error) => {
                    ApiError::Internal(anyhow::anyhow!(error))
                }
                _ => ApiError::Unauthorized,
            })?;

        Ok(Self::from_principal(principal))
    }
}

pub use ame_platform_postgres::auth_cache::{
    invalidate_token, invalidate_user_caches, invalidate_user_tokens,
};
