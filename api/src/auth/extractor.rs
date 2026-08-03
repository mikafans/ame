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

/// The authenticated learner projection used by HTTP handlers. A delegated
/// author retains the learner owner but has an agent actor identity and a
/// deliberately narrow scope.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user: AuthenticatedUserView,
    pub owner_id: Uuid,
    pub actor_identity_id: Uuid,
    pub scope: ame_platform_postgres::domain::auth::PrincipalScope,
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

    pub fn actor_identity_id(&self) -> Uuid {
        self.actor_identity_id
    }

    pub fn is_delegated_course_author(&self) -> bool {
        self.scope == ame_platform_postgres::domain::auth::PrincipalScope::CourseAuthor
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
            actor_identity_id: principal.actor_identity_id,
            scope: principal.scope,
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
                    .filter(|token| token.kind == TokenKind::Login)
            })
            .ok_or(ApiError::Unauthorized)?;

        let now = time::OffsetDateTime::now_utc();
        let repository = PgAuthenticationRepository::new(state.pool.clone());
        let is_login = parsed.kind == TokenKind::Login;
        let principal = (if is_login {
            repository
                .authenticate_login_session(parsed.id, &parsed.secret, now)
                .await
        } else if parsed.kind == TokenKind::Delegation {
            repository
                .authenticate_agent_delegation(parsed.id, &parsed.secret, now)
                .await
        } else {
            Err(crate::domain::auth::AuthenticationRepositoryError::NotFound)
        })
        .map_err(|error| match error {
            crate::domain::auth::AuthenticationRepositoryError::Storage(error) => {
                ApiError::Internal(anyhow::anyhow!(error))
            }
            _ => ApiError::Unauthorized,
        })?;

        // Sliding-window sessions: each authenticated request extends the
        // session TTL so active learners are not logged out mid-session.
        if is_login {
            let renewed_expiry =
                now + time::Duration::seconds(state.config.login.ttl_seconds as i64);
            repository
                .renew_login_session(parsed.id, renewed_expiry)
                .await
                .map_err(|error| ApiError::Internal(anyhow::anyhow!(format!("{error:?}"))))?;
        }

        Ok(Self::from_principal(principal))
    }
}

pub use ame_platform_postgres::auth_cache::{
    invalidate_token, invalidate_user_caches, invalidate_user_tokens,
};
