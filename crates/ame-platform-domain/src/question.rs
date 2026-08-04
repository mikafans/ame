//! Domain contracts for versioned questions and deterministic evaluation.

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct GradedAnswer {
    pub correctness: Option<f32>,
    pub awarded_points: f32,
    pub status: AnswerEvaluationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerEvaluationStatus {
    Correct,
    Incorrect,
    Partial,
    ManualReview,
}

use super::generation::GenerationError;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    MultipleChoice,
    TrueFalse,
    ShortAnswer,
    Numeric,
    Essay,
    Code,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentReviewStatus {
    #[default]
    Draft,
    Review,
    Approved,
    Rejected,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuestionOption {
    pub id: String,
    pub text: String,
    pub is_correct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateQuestion {
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub generation_run_id: Uuid,
    pub kind: QuestionKind,
    pub prompt: String,
    pub options: Vec<QuestionOption>,
    pub accepted_answers: Vec<String>,
    pub explanation: Option<String>,
    pub rationale: Option<String>,
    pub difficulty: Option<String>,
    pub points: u32,
    pub review_status: ContentReviewStatus,
    pub source_references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub source_actor_id: Uuid,
    pub generation_run_id: Uuid,
    pub kind: QuestionKind,
    pub current_version: u32,
    pub status: ContentReviewStatus,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionVersion {
    pub id: Uuid,
    pub question_id: Uuid,
    pub generation_run_id: Uuid,
    pub version: u32,
    pub kind: QuestionKind,
    pub prompt: String,
    pub options: Vec<QuestionOption>,
    pub accepted_answers: Vec<String>,
    pub explanation: Option<String>,
    pub rationale: Option<String>,
    pub difficulty: Option<String>,
    pub points: u32,
    pub review_status: ContentReviewStatus,
    pub source_references: Vec<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionRepositoryError {
    EmptyField { field: &'static str },
    InvalidQuestion(String),
    NotFound { resource: &'static str },
    SubjectMismatch,
    VersionConflict,
    Generation(GenerationError),
    Storage(String),
}

impl Display for QuestionRepositoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField { field } => {
                write!(formatter, "question field {field} must not be empty")
            }
            Self::InvalidQuestion(message) => write!(formatter, "invalid question: {message}"),
            Self::NotFound { resource } => {
                write!(formatter, "question resource not found: {resource}")
            }
            Self::SubjectMismatch => write!(formatter, "question belongs to another subject"),
            Self::VersionConflict => write!(formatter, "question version is not the next version"),
            Self::Generation(error) => write!(formatter, "question generation provenance: {error}"),
            Self::Storage(message) => {
                write!(formatter, "question repository storage failure: {message}")
            }
        }
    }
}

impl std::error::Error for QuestionRepositoryError {}

pub fn validate_question(input: &CreateQuestion) -> Result<(), QuestionRepositoryError> {
    if input.prompt.trim().is_empty() {
        return Err(QuestionRepositoryError::EmptyField { field: "prompt" });
    }
    if input.points == 0 {
        return Err(QuestionRepositoryError::InvalidQuestion(
            "points must be greater than zero".to_string(),
        ));
    }
    match input.kind {
        QuestionKind::MultipleChoice => {
            if input.options.len() < 2 {
                return Err(QuestionRepositoryError::InvalidQuestion(
                    "multiple-choice questions need at least two options".to_string(),
                ));
            }
            if input
                .options
                .iter()
                .any(|option| option.id.trim().is_empty() || option.text.trim().is_empty())
            {
                return Err(QuestionRepositoryError::InvalidQuestion(
                    "multiple-choice options need stable ids and text".to_string(),
                ));
            }
            if input
                .options
                .iter()
                .filter(|option| option.is_correct)
                .count()
                != 1
            {
                return Err(QuestionRepositoryError::InvalidQuestion(
                    "multiple-choice questions need exactly one correct option".to_string(),
                ));
            }
        }
        QuestionKind::TrueFalse => {
            if !input.accepted_answers.is_empty() {
                return Err(QuestionRepositoryError::InvalidQuestion(
                    "true/false answers use the option payload".to_string(),
                ));
            }
        }
        QuestionKind::ShortAnswer => {
            if input.accepted_answers.is_empty() {
                return Err(QuestionRepositoryError::InvalidQuestion(
                    "short-answer questions need at least one accepted answer".to_string(),
                ));
            }
        }
        QuestionKind::Numeric => {
            // accepted_answers[0] is the numeric target; an optional
            // accepted_answers[1] is the tolerance ("0.5" absolute or "2%" relative).
            let target = input.accepted_answers.first().ok_or_else(|| {
                QuestionRepositoryError::InvalidQuestion(
                    "numeric questions need a target value in accepted_answers[0]".to_string(),
                )
            })?;
            if target.trim().parse::<f64>().is_err() {
                return Err(QuestionRepositoryError::InvalidQuestion(
                    "numeric target must be a number".to_string(),
                ));
            }
            if let Some(tolerance) = input.accepted_answers.get(1) {
                let raw = tolerance
                    .trim()
                    .strip_suffix('%')
                    .unwrap_or(tolerance.trim());
                if raw.parse::<f64>().is_err() {
                    return Err(QuestionRepositoryError::InvalidQuestion(
                        "numeric tolerance must be a number or percentage".to_string(),
                    ));
                }
            }
        }
        QuestionKind::Essay | QuestionKind::Code => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::QuestionOption;

    #[test]
    fn question_options_use_camel_case_on_the_wire() {
        let option = QuestionOption {
            id: "correct".into(),
            text: "The JobManager".into(),
            is_correct: true,
        };

        let json = serde_json::to_value(&option).unwrap();
        assert_eq!(json["isCorrect"], true);
        assert!(json.get("is_correct").is_none());

        let decoded: QuestionOption = serde_json::from_value(serde_json::json!({
            "id": "correct",
            "text": "The JobManager",
            "isCorrect": true
        }))
        .unwrap();
        assert!(decoded.is_correct);
    }
}
