//! In-memory authentication contract implementation and shared contract tests.

use crate::auth::token::verify_token_secret;
use crate::domain::auth::{
    AuthenticatedPrincipal, AuthenticationRepository, AuthenticationRepositoryError, PrincipalKind,
    PrincipalRole,
};
use crate::domain::user::Scope;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryAuthenticationRepository {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Clone)]
pub struct AuthenticationFixture {
    pub credential_id: Uuid,
    pub actor_identity_id: Uuid,
    pub owner_user_id: Uuid,
    pub token_hash: String,
    pub email: Option<String>,
    pub display_name: String,
    pub created_at: OffsetDateTime,
    pub role: PrincipalRole,
    pub scopes: Vec<Scope>,
    pub expires_at: OffsetDateTime,
    pub revoked: bool,
    pub owner_active: bool,
}

#[derive(Default)]
struct State {
    login_sessions: HashMap<Uuid, AuthenticationFixture>,
    api_tokens: HashMap<Uuid, AuthenticationFixture>,
}

impl InMemoryAuthenticationRepository {
    pub fn with_login_session(fixture: AuthenticationFixture) -> Self {
        let repository = Self::default();
        repository.insert_login_session(fixture);
        repository
    }

    pub fn insert_login_session(&self, fixture: AuthenticationFixture) {
        if let Ok(mut state) = self.state.lock() {
            state.login_sessions.insert(fixture.credential_id, fixture);
        }
    }

    pub fn insert_api_token(&self, fixture: AuthenticationFixture) {
        if let Ok(mut state) = self.state.lock() {
            state.api_tokens.insert(fixture.credential_id, fixture);
        }
    }

    fn authenticate(
        &self,
        fixture: &AuthenticationFixture,
        secret: &str,
        now: OffsetDateTime,
        kind: PrincipalKind,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError> {
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
            actor_identity_id: fixture.actor_identity_id,
            owner_user_id: fixture.owner_user_id,
            kind,
            role: fixture.role,
            email: fixture.email.clone(),
            display_name: fixture.display_name.clone(),
            created_at: fixture.created_at,
            scopes: fixture.scopes.clone(),
            credential_id: fixture.credential_id,
            expires_at: fixture.expires_at,
        })
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
        let state = self
            .state
            .lock()
            .map_err(|error| AuthenticationRepositoryError::Storage(error.to_string()))?;
        let fixture = state
            .login_sessions
            .get(&session_id)
            .cloned()
            .ok_or(AuthenticationRepositoryError::NotFound)?;
        drop(state);
        self.authenticate(&fixture, secret, now, PrincipalKind::Human)
    }

    async fn authenticate_api_token(
        &self,
        token_id: Uuid,
        secret: &str,
        now: OffsetDateTime,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|error| AuthenticationRepositoryError::Storage(error.to_string()))?;
        let fixture = state
            .api_tokens
            .get(&token_id)
            .cloned()
            .ok_or(AuthenticationRepositoryError::NotFound)?;
        drop(state);
        self.authenticate(&fixture, secret, now, PrincipalKind::Agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::token::hash_secret;

    fn fixture(secret: &str) -> AuthenticationFixture {
        let owner_user_id = Uuid::now_v7();
        AuthenticationFixture {
            credential_id: Uuid::now_v7(),
            actor_identity_id: owner_user_id,
            owner_user_id,
            token_hash: hash_secret(secret),
            email: Some("learner@example.test".to_string()),
            display_name: "Learner".to_string(),
            created_at: OffsetDateTime::now_utc(),
            role: PrincipalRole::Learner,
            scopes: vec![Scope::AttemptWrite],
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
            revoked: false,
            owner_active: true,
        }
    }

    #[tokio::test]
    async fn human_session_produces_clean_principal() {
        let secret = "human-secret";
        let fixture = fixture(secret);
        let id = fixture.credential_id;
        let repository = InMemoryAuthenticationRepository::with_login_session(fixture);

        let principal = repository
            .authenticate_login_session(id, secret, OffsetDateTime::now_utc())
            .await
            .expect("active session authenticates");
        assert_eq!(principal.kind, PrincipalKind::Human);
        assert_eq!(principal.owner_user_id, principal.actor_identity_id);
        assert_eq!(principal.scopes, vec![Scope::AttemptWrite]);
    }

    #[tokio::test]
    async fn agent_token_preserves_actor_owner_and_scopes() {
        let secret = "agent-secret";
        let mut fixture = fixture(secret);
        fixture.actor_identity_id = Uuid::now_v7();
        let id = fixture.credential_id;
        let owner = fixture.owner_user_id;
        let repository = InMemoryAuthenticationRepository::default();
        repository.insert_api_token(fixture);

        let principal = repository
            .authenticate_api_token(id, secret, OffsetDateTime::now_utc())
            .await
            .expect("active agent token authenticates");
        assert_eq!(principal.kind, PrincipalKind::Agent);
        assert_eq!(principal.owner_user_id, owner);
        assert_eq!(principal.scopes, vec![Scope::AttemptWrite]);
    }

    #[tokio::test]
    async fn invalid_secret_revoked_and_expired_credentials_are_rejected() {
        let secret = "secret";
        let mut fixture = fixture(secret);
        let id = fixture.credential_id;
        let repository = InMemoryAuthenticationRepository::with_login_session(fixture.clone());
        assert_eq!(
            repository
                .authenticate_login_session(id, "wrong", OffsetDateTime::now_utc())
                .await,
            Err(AuthenticationRepositoryError::InvalidSecret)
        );

        fixture.revoked = true;
        let revoked = InMemoryAuthenticationRepository::with_login_session(fixture.clone());
        assert_eq!(
            revoked
                .authenticate_login_session(id, secret, OffsetDateTime::now_utc())
                .await,
            Err(AuthenticationRepositoryError::Revoked)
        );

        fixture.revoked = false;
        fixture.expires_at = OffsetDateTime::now_utc() - time::Duration::seconds(1);
        let expired = InMemoryAuthenticationRepository::with_login_session(fixture);
        assert_eq!(
            expired
                .authenticate_login_session(id, secret, OffsetDateTime::now_utc())
                .await,
            Err(AuthenticationRepositoryError::Expired)
        );
    }
}
