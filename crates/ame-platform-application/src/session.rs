//! Session credential issuance for the self-host-first onboarding flow.

use crate::auth::token::{TokenKind, format_token, generate_secret, hash_secret};
use crate::domain::identity::{
    AccountStatus, BrowserSession, CreateBrowserSession, IdentityRepository,
    IdentityRepositoryError,
};
use async_trait::async_trait;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[async_trait]
pub trait SessionSecretGenerator: Send + Sync {
    async fn generate(&self) -> String;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OsSessionSecretGenerator;

#[async_trait]
impl SessionSecretGenerator for OsSessionSecretGenerator {
    async fn generate(&self) -> String {
        generate_secret()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedBrowserSession {
    pub token: String,
    pub session: BrowserSession,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionCredentialError {
    #[error("session lifetime must be greater than zero")]
    InvalidLifetime,
    #[error("learner account is not active")]
    InactiveAccount,
    #[error(transparent)]
    Identity(#[from] IdentityRepositoryError),
}

#[derive(Clone)]
pub struct BrowserSessionService<I, G = OsSessionSecretGenerator> {
    identity: I,
    secrets: G,
}

impl<I> BrowserSessionService<I, OsSessionSecretGenerator>
where
    I: IdentityRepository,
{
    pub fn new(identity: I) -> Self {
        Self {
            identity,
            secrets: OsSessionSecretGenerator,
        }
    }
}

impl<I, G> BrowserSessionService<I, G>
where
    I: IdentityRepository,
    G: SessionSecretGenerator,
{
    pub fn with_secret_generator(identity: I, secrets: G) -> Self {
        Self { identity, secrets }
    }

    pub async fn issue(
        &self,
        user_id: Uuid,
        lifetime: Duration,
    ) -> Result<IssuedBrowserSession, SessionCredentialError> {
        if lifetime <= Duration::ZERO {
            return Err(SessionCredentialError::InvalidLifetime);
        }

        let account = self.identity.get_learner(user_id).await?;
        if account.status != AccountStatus::Active {
            return Err(SessionCredentialError::InactiveAccount);
        }

        let secret = self.secrets.generate().await;
        let session = self
            .identity
            .create_browser_session(CreateBrowserSession {
                user_id,
                token_hash: hash_secret(&secret),
                expires_at: OffsetDateTime::now_utc() + lifetime,
            })
            .await?;

        Ok(IssuedBrowserSession {
            token: format_token(TokenKind::Login, session.id, &secret),
            session,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BrowserSessionService, SessionCredentialError, SessionSecretGenerator};
    use crate::auth::token::{TokenKind, hash_secret, parse_token_value, verify_token_secret};
    use crate::domain::identity::{CreateLearner, IdentityRepository};
    use crate::identity::InMemoryIdentityRepository;
    use async_trait::async_trait;
    use std::sync::Arc;
    use time::Duration;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    #[derive(Clone)]
    struct FixedSecretGenerator {
        secret: Arc<Mutex<String>>,
    }

    #[async_trait]
    impl SessionSecretGenerator for FixedSecretGenerator {
        async fn generate(&self) -> String {
            self.secret.lock().await.clone()
        }
    }

    #[tokio::test]
    async fn issue_creates_verifiable_login_credential() {
        let identity = InMemoryIdentityRepository::default();
        let account = identity
            .create_learner(CreateLearner {
                email: "learner@example.test".to_string(),
                display_name: "Learner".to_string(),
            })
            .await
            .expect("account creates");
        let service = BrowserSessionService::with_secret_generator(
            identity.clone(),
            FixedSecretGenerator {
                secret: Arc::new(Mutex::new("fixed-session-secret".to_string())),
            },
        );

        let issued = service
            .issue(account.id, Duration::hours(1))
            .await
            .expect("session issues");
        let parsed = parse_token_value(&issued.token).expect("token parses");

        assert_eq!(parsed.kind, TokenKind::Login);
        assert_eq!(parsed.id, issued.session.id);
        assert_eq!(parsed.secret, "fixed-session-secret");
        assert_eq!(issued.session.user_id, account.id);
        assert_eq!(
            issued.session.token_hash,
            hash_secret("fixed-session-secret")
        );
        assert!(verify_token_secret(
            &issued.session.token_hash,
            &parsed.secret
        ));
        assert_eq!(
            identity
                .get_browser_session(issued.session.id)
                .await
                .expect("session reads"),
            issued.session
        );
    }

    #[tokio::test]
    async fn issue_rejects_invalid_lifetime_and_unknown_account() {
        let identity = InMemoryIdentityRepository::default();
        let service = BrowserSessionService::with_secret_generator(
            identity,
            FixedSecretGenerator {
                secret: Arc::new(Mutex::new("fixed-session-secret".to_string())),
            },
        );

        assert_eq!(
            service.issue(Uuid::now_v7(), Duration::ZERO).await,
            Err(SessionCredentialError::InvalidLifetime)
        );
        assert_eq!(
            service.issue(Uuid::now_v7(), Duration::hours(1)).await,
            Err(SessionCredentialError::Identity(
                crate::domain::identity::IdentityRepositoryError::AccountNotFound
            ))
        );
    }
}
