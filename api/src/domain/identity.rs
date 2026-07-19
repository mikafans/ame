//! Domain contracts for self-host-first learner identity and browser sessions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationMode {
    #[default]
    Open,
    Invite,
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLearner {
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnerAccount {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub status: AccountStatus,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Active,
    Deactivated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBrowserSession {
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdentityRepositoryError {
    #[error("identity field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("learner account already exists")]
    AccountAlreadyExists,
    #[error("learner account not found")]
    AccountNotFound,
    #[error("learner email does not match the authenticated account")]
    AccountEmailMismatch,
    #[error("browser session not found")]
    SessionNotFound,
    #[error("identity repository storage failure: {0}")]
    Storage(String),
}

impl IdentityRepositoryError {
    pub(crate) fn storage(error: impl Display) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn get_learner(&self, user_id: Uuid) -> Result<LearnerAccount, IdentityRepositoryError>;

    async fn find_learner_by_email(
        &self,
        email: &str,
    ) -> Result<Option<LearnerAccount>, IdentityRepositoryError>;

    async fn create_learner(
        &self,
        input: CreateLearner,
    ) -> Result<LearnerAccount, IdentityRepositoryError>;

    async fn create_browser_session(
        &self,
        input: CreateBrowserSession,
    ) -> Result<BrowserSession, IdentityRepositoryError>;

    async fn get_browser_session(
        &self,
        session_id: Uuid,
    ) -> Result<BrowserSession, IdentityRepositoryError>;
}
