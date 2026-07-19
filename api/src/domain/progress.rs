//! Domain contracts for evidence-backed progress and recommendations.

use std::fmt::Display;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct MasteryEvidenceInput {
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub activity_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub value: f32,
    pub derivation_version: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MasteryEvidence {
    pub id: Uuid,
    pub input: MasteryEvidenceInput,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MasterySnapshot {
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub mastery: f32,
    pub confidence: f32,
    pub evidence_count: u32,
    pub calculated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreakEventInput {
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Uuid,
    pub qualifying_event_key: String,
    pub learner_timezone: String,
    pub qualifying_day: time::Date,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreakEvent {
    pub id: Uuid,
    pub input: StreakEventInput,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recommendation {
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub activity_id: Uuid,
    pub reason: String,
    pub evidence_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressError {
    EmptyField { field: &'static str },
    InvalidValue,
    DuplicateStreakEvent,
    SubjectMismatch,
    NotFound,
    Storage(String),
}

impl Display for ProgressError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField { field } => {
                write!(formatter, "progress field {field} must not be empty")
            }
            Self::InvalidValue => write!(
                formatter,
                "mastery evidence value must be between zero and one"
            ),
            Self::DuplicateStreakEvent => write!(formatter, "streak event was already recorded"),
            Self::SubjectMismatch => write!(formatter, "progress belongs to another subject"),
            Self::NotFound => write!(formatter, "progress record not found"),
            Self::Storage(message) => write!(formatter, "progress storage failure: {message}"),
        }
    }
}

impl std::error::Error for ProgressError {}

pub fn validate_evidence(input: &MasteryEvidenceInput) -> Result<(), ProgressError> {
    if !input.value.is_finite() || !(0.0..=1.0).contains(&input.value) {
        return Err(ProgressError::InvalidValue);
    }
    if input.derivation_version == 0 {
        return Err(ProgressError::EmptyField {
            field: "derivation_version",
        });
    }
    Ok(())
}

pub fn validate_streak_event(input: &StreakEventInput) -> Result<(), ProgressError> {
    if input.qualifying_event_key.trim().is_empty() {
        return Err(ProgressError::EmptyField {
            field: "qualifying_event_key",
        });
    }
    if input.learner_timezone.trim().is_empty() {
        return Err(ProgressError::EmptyField {
            field: "learner_timezone",
        });
    }
    Ok(())
}
