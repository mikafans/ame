//! In-memory authentication contract implementation and contract tests.

use crate::auth::token::verify_token_secret;
use crate::domain::auth::{
    AuthenticatedPrincipal, AuthenticationRepository, AuthenticationRepositoryError, PrincipalRole,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryAuthenticationRepository {
    sessions: Arc<Mutex<HashMap<Uuid, AuthenticationFixture>>>,
}

#[derive(Debug, Clone)]
pub struct AuthenticationFixture {
    pub credential_id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub email: Option<String>,
    pub display_name: String,
    pub created_at: OffsetDateTime,
    pub role: PrincipalRole,
    pub expires_at: OffsetDateTime,
    pub revoked: bool,
    pub owner_active: bool,
}

impl InMemoryAuthenticationRepository {
    pub fn with_login_session(fixture: AuthenticationFixture) -> Self {
        let repository = Self::default();
        repository.insert_login_session(fixture);
        repository
    }

    pub fn insert_login_session(&self, fixture: AuthenticationFixture) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(fixture.credential_id, fixture);
        }
    }
}

#[async_trait]
impl AuthenticationRepository for InMemoryAuthenticationRepository {
    async fn authenticate_login_session(
        &self,
        session_id: Uuid,
        secret: &str,
        now: OffsetDateTime,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError> {
        let fixture = self
            .sessions
            .lock()
            .map_err(|error| AuthenticationRepositoryError::Storage(error.to_string()))?
            .get(&session_id)
            .cloned()
            .ok_or(AuthenticationRepositoryError::NotFound)?;
        if fixture.revoked {
            return Err(AuthenticationRepositoryError::Revoked);
        }
        if fixture.expires_at <= now {
            return Err(AuthenticationRepositoryError::Expired);
        }
        if !fixture.owner_active {
            return Err(AuthenticationRepositoryError::OwnerInactive);
        }
        if !verify_token_secret(&fixture.token_hash, secret) {
            return Err(AuthenticationRepositoryError::InvalidSecret);
        }
        Ok(AuthenticatedPrincipal {
            user_id: fixture.user_id,
            role: fixture.role,
            email: fixture.email,
            display_name: fixture.display_name,
            created_at: fixture.created_at,
            credential_id: fixture.credential_id,
            expires_at: fixture.expires_at,
            actor_identity_id: fixture.user_id,
            scope: ame_platform_domain::auth::PrincipalScope::Learner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::token::hash_secret;

    #[tokio::test]
    async fn human_session_produces_owner_principal() {
        let secret = "human-secret";
        let user_id = Uuid::now_v7();
        let fixture = AuthenticationFixture {
            credential_id: Uuid::now_v7(),
            user_id,
            token_hash: hash_secret(secret),
            email: Some("learner@example.test".into()),
            display_name: "Learner".into(),
            created_at: OffsetDateTime::now_utc(),
            role: PrincipalRole::Learner,
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
            revoked: false,
            owner_active: true,
        };
        let id = fixture.credential_id;
        let repository = InMemoryAuthenticationRepository::with_login_session(fixture);
        let principal = repository
            .authenticate_login_session(id, secret, OffsetDateTime::now_utc())
            .await
            .unwrap();
        assert_eq!(principal.user_id, user_id);
        assert_eq!(principal.role, PrincipalRole::Learner);
    }
}
