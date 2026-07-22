//! Domain contract for evidence-linked explanatory deep dives.

use crate::domain::generation::GenerationError;
use crate::domain::question::ContentReviewStatus;
use std::fmt::Display;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDeepDive {
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub generation_run_id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub objective_id: Uuid,
    pub triggering_evidence_id: Uuid,
    pub title: String,
    pub body: String,
    pub example: String,
    pub caveats: Vec<String>,
    pub source_references: Vec<String>,
    pub application_task: String,
    pub review_status: ContentReviewStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepDive {
    pub id: Uuid,
    pub input: CreateDeepDive,
    pub content_version: u32,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepDiveError {
    EmptyField { field: &'static str },
    MissingSource,
    NotFound,
    SubjectMismatch,
    Generation(GenerationError),
    Storage(String),
}

impl Display for DeepDiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField { field } => {
                write!(formatter, "deep-dive field {field} must not be empty")
            }
            Self::MissingSource => {
                write!(formatter, "deep-dive needs at least one source reference")
            }
            Self::NotFound => write!(formatter, "deep-dive not found"),
            Self::SubjectMismatch => write!(formatter, "deep-dive belongs to another subject"),
            Self::Generation(error) => {
                write!(formatter, "deep-dive generation provenance: {error}")
            }
            Self::Storage(message) => write!(formatter, "deep-dive storage failure: {message}"),
        }
    }
}

impl std::error::Error for DeepDiveError {}

pub fn validate_deep_dive(input: &CreateDeepDive) -> Result<(), DeepDiveError> {
    for (field, value) in [
        ("title", input.title.as_str()),
        ("body", input.body.as_str()),
        ("example", input.example.as_str()),
        ("application_task", input.application_task.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(DeepDiveError::EmptyField { field });
        }
    }
    if input.source_references.is_empty() {
        return Err(DeepDiveError::MissingSource);
    }
    Ok(())
}
