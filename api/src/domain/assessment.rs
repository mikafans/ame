//! Domain contracts for versioned practice and graded assessments.

use crate::domain::question::QuestionRepositoryError;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentMode {
    Practice,
    Graded,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Draft,
    Published,
    Retired,
}

impl Default for AssessmentStatus {
    fn default() -> Self {
        Self::Draft
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentItemInput {
    pub id: Uuid,
    pub question_version_id: Uuid,
    pub order_index: i32,
    pub points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAssessment {
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub activity_id: Uuid,
    pub mode: AssessmentMode,
    pub items: Vec<AssessmentItemInput>,
    pub status: AssessmentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub activity_id: Uuid,
    pub version: u32,
    pub mode: AssessmentMode,
    pub items: Vec<AssessmentItemInput>,
    pub status: AssessmentStatus,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssessmentRepositoryError {
    EmptyItems,
    InvalidItem(String),
    NotFound,
    SubjectMismatch,
    ActivityAlreadyHasAssessment,
    Question(QuestionRepositoryError),
    Storage(String),
}

impl Display for AssessmentRepositoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyItems => write!(formatter, "assessment must contain at least one item"),
            Self::InvalidItem(message) => write!(formatter, "invalid assessment item: {message}"),
            Self::NotFound => write!(formatter, "assessment not found"),
            Self::SubjectMismatch => write!(formatter, "assessment belongs to another subject"),
            Self::ActivityAlreadyHasAssessment => {
                write!(formatter, "activity already has an assessment")
            }
            Self::Question(error) => write!(formatter, "{error}"),
            Self::Storage(message) => write!(formatter, "assessment storage failure: {message}"),
        }
    }
}

impl std::error::Error for AssessmentRepositoryError {}

pub fn validate_assessment(input: &CreateAssessment) -> Result<(), AssessmentRepositoryError> {
    if input.items.is_empty() {
        return Err(AssessmentRepositoryError::EmptyItems);
    }
    let mut orders = std::collections::HashSet::new();
    for item in &input.items {
        if item.order_index < 0 {
            return Err(AssessmentRepositoryError::InvalidItem(
                "order must not be negative".to_string(),
            ));
        }
        if item.points == 0 {
            return Err(AssessmentRepositoryError::InvalidItem(
                "points must be greater than zero".to_string(),
            ));
        }
        if !orders.insert(item.order_index) {
            return Err(AssessmentRepositoryError::InvalidItem(
                "item order must be unique".to_string(),
            ));
        }
    }
    Ok(())
}
