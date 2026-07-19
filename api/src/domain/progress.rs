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
    pub attempt_id: Uuid,
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
    InvalidTimezone,
    InvalidStreakDay,
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
            Self::InvalidTimezone => write!(formatter, "learner timezone is not recognized"),
            Self::InvalidStreakDay => write!(
                formatter,
                "qualifying day does not match the completed attempt"
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
    attempt_id_from_event_key(&input.qualifying_event_key)?;
    Ok(())
}

pub fn qualifying_day_for_attempt(
    graded_at: OffsetDateTime,
    learner_timezone: &str,
) -> Result<time::Date, ProgressError> {
    use chrono::Datelike;

    let timezone = learner_timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| ProgressError::InvalidTimezone)?;
    let utc = chrono::DateTime::<chrono::Utc>::from_timestamp(
        graded_at.unix_timestamp(),
        graded_at.nanosecond(),
    )
    .ok_or(ProgressError::InvalidTimezone)?;
    let local = utc.with_timezone(&timezone);
    time::Date::from_calendar_date(
        local.year(),
        time::Month::try_from(local.month() as u8).map_err(|_| ProgressError::InvalidTimezone)?,
        local.day() as u8,
    )
    .map_err(|_| ProgressError::InvalidTimezone)
}

pub fn attempt_id_from_event_key(event_key: &str) -> Result<Uuid, ProgressError> {
    let attempt_id = event_key
        .strip_prefix("attempt:")
        .filter(|value| !value.is_empty())
        .ok_or(ProgressError::EmptyField {
            field: "qualifying_event_key",
        })?;
    Uuid::parse_str(attempt_id).map_err(|_| ProgressError::EmptyField {
        field: "qualifying_event_key",
    })
}
