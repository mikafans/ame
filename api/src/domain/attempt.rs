//! Domain contract for resumable assessment attempts.

use crate::assessment::AssessmentGrade;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    InProgress,
    Submitted,
    Graded,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartAttempt {
    pub subject_user_id: Uuid,
    pub learning_session_id: Uuid,
    pub activity_id: Uuid,
    pub assessment_id: Uuid,
    pub assessment_version: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttemptAnswer {
    pub assessment_item_id: Uuid,
    pub question_version_id: Uuid,
    pub response: serde_json::Value,
    pub correctness: Option<f32>,
    pub awarded_points: Option<f32>,
    pub evaluation_status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attempt {
    pub id: Uuid,
    pub input: StartAttempt,
    pub status: AttemptStatus,
    pub responses: std::collections::HashMap<Uuid, serde_json::Value>,
    pub item_question_versions: std::collections::HashMap<Uuid, Uuid>,
    pub answer_results: Vec<AttemptAnswer>,
    pub stored_score: Option<f32>,
    pub stored_max_points: Option<f32>,
    pub grade: Option<AssessmentGrade>,
    pub created_at: OffsetDateTime,
    pub submitted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptError {
    NotFound,
    SubjectMismatch,
    AlreadyFinished,
    ResultConflict,
    StaleQuestionVersion,
    ItemNotInAssessment,
    InvalidTransition,
    Storage(String),
}

impl Display for AttemptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "attempt not found"),
            Self::SubjectMismatch => write!(formatter, "attempt belongs to another subject"),
            Self::AlreadyFinished => write!(formatter, "attempt is already finished"),
            Self::ResultConflict => {
                write!(formatter, "attempt was finished with different answers")
            }
            Self::StaleQuestionVersion => {
                write!(formatter, "answer references a stale question version")
            }
            Self::ItemNotInAssessment => write!(formatter, "answer item is not in the assessment"),
            Self::InvalidTransition => write!(formatter, "attempt transition is invalid"),
            Self::Storage(message) => write!(formatter, "attempt storage failure: {message}"),
        }
    }
}

impl std::error::Error for AttemptError {}
