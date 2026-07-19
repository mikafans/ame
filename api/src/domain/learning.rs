//! Domain contracts for the first agent-first learning journey.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Proposed,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JourneyStatus {
    Onboarding,
    Active,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGoal {
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub template_version_id: Option<Uuid>,
    pub raw_intent: String,
    pub normalized_statement: String,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningGoal {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub template_version_id: Option<Uuid>,
    pub raw_intent: String,
    pub normalized_statement: String,
    pub status: GoalStatus,
    pub idempotency_key: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateJourney {
    pub goal_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub promise: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningJourney {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub promise: String,
    pub status: JourneyStatus,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LearningRepositoryError {
    #[error("learning field {field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("learning resource not found: {resource}")]
    NotFound { resource: &'static str },
    #[error("learning resource belongs to another subject")]
    SubjectMismatch,
    #[error("idempotency key was reused with a different goal")]
    IdempotencyConflict,
    #[error("a journey already exists for this goal")]
    JourneyAlreadyExists,
    #[error("learning repository storage failure: {0}")]
    Storage(String),
}

impl LearningRepositoryError {
    pub(crate) fn storage(error: impl Display) -> Self {
        Self::Storage(error.to_string())
    }
}

#[async_trait]
pub trait LearningRepository: Send + Sync {
    async fn create_goal(&self, input: CreateGoal)
    -> Result<LearningGoal, LearningRepositoryError>;

    async fn get_goal(
        &self,
        subject_user_id: Uuid,
        goal_id: Uuid,
    ) -> Result<LearningGoal, LearningRepositoryError>;

    async fn ensure_journey(
        &self,
        input: CreateJourney,
    ) -> Result<LearningJourney, LearningRepositoryError>;

    async fn get_journey(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
    ) -> Result<LearningJourney, LearningRepositoryError>;

    async fn set_journey_status(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        status: JourneyStatus,
    ) -> Result<LearningJourney, LearningRepositoryError>;
}

pub(crate) fn validate_goal(input: &CreateGoal) -> Result<(), LearningRepositoryError> {
    if input.raw_intent.trim().is_empty() {
        return Err(LearningRepositoryError::EmptyField {
            field: "raw_intent",
        });
    }
    if input.normalized_statement.trim().is_empty() {
        return Err(LearningRepositoryError::EmptyField {
            field: "normalized_statement",
        });
    }
    if input
        .idempotency_key
        .as_ref()
        .is_some_and(|key| key.trim().is_empty())
    {
        return Err(LearningRepositoryError::EmptyField {
            field: "idempotency_key",
        });
    }
    Ok(())
}

pub(crate) fn validate_journey(input: &CreateJourney) -> Result<(), LearningRepositoryError> {
    if input.promise.trim().is_empty() {
        return Err(LearningRepositoryError::EmptyField { field: "promise" });
    }
    Ok(())
}
