//! Authentication contracts for the clean self-host baseline.

use crate::domain::user::Scope;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalKind {
    Human,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalRole {
    Learner,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    /// Identity that authored the request and owns audit/idempotency records.
    pub actor_identity_id: Uuid,
    /// Human learner whose data the request may access.
    pub owner_user_id: Uuid,
    pub kind: PrincipalKind,
    pub role: PrincipalRole,
    pub email: Option<String>,
    pub display_name: String,
    pub scopes: Vec<Scope>,
    pub credential_id: Uuid,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthenticationRepositoryError {
    #[error("authentication credential not found")]
    NotFound,
    #[error("authentication credential is revoked")]
    Revoked,
    #[error("authentication credential is expired")]
    Expired,
    #[error("authentication owner is inactive")]
    OwnerInactive,
    #[error("authentication secret is invalid")]
    InvalidSecret,
    #[error("authentication data is invalid: {0}")]
    InvalidData(String),
    #[error("authentication repository storage failure: {0}")]
    Storage(String),
}

/// The only persistence boundary an HTTP authentication extractor should use.
///
/// Implementations own the joins between credentials, identities, and users.
/// This keeps request code independent from table layout and makes the clean
/// baseline contract testable without retaining the retired auth schema.
#[async_trait]
pub trait AuthenticationRepository: Send + Sync {
    async fn authenticate_login_session(
        &self,
        session_id: Uuid,
        secret: &str,
        now: OffsetDateTime,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError>;

    async fn authenticate_api_token(
        &self,
        token_id: Uuid,
        secret: &str,
        now: OffsetDateTime,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError>;
}
