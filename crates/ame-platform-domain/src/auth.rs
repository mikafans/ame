//! Authentication contracts for the learner-only self-host baseline.

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalRole {
    Learner,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalScope {
    Learner,
    CourseAuthor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    /// The authenticated learner and the owner of every learner resource.
    pub user_id: Uuid,
    pub role: PrincipalRole,
    pub plan: String,
    pub email: Option<String>,
    pub display_name: String,
    pub created_at: OffsetDateTime,
    pub credential_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub actor_identity_id: Uuid,
    pub scope: PrincipalScope,
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

#[async_trait]
pub trait AuthenticationRepository: Send + Sync {
    async fn authenticate_login_session(
        &self,
        session_id: Uuid,
        secret: &str,
        now: OffsetDateTime,
    ) -> Result<AuthenticatedPrincipal, AuthenticationRepositoryError>;
}
