//! Pure grading logic for question attempts.

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use utoipa::ToSchema;

use crate::domain::attempt::{AttemptPresentation, AttemptResponse};
use crate::domain::question::{
    CodePayload, EssayPayload, McPayload, Normalize, QuestionKind, ShortPayload, TfPayload,
};

fn strip_accents(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

impl GradeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GradeStatus::Graded => "graded",
            GradeStatus::PendingManual => "pending_manual",
        }
    }
}

impl std::str::FromStr for GradeStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "graded" => Ok(GradeStatus::Graded),
            "pending_manual" => Ok(GradeStatus::PendingManual),
            _ => Err(format!("unknown grade status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GradeStatus {
    Graded,
    PendingManual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct GradeOutcome {
    pub status: GradeStatus,
    pub correct: bool,
    pub points_awarded: i32,
    pub max: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[schema(value_type = Object)]
    pub correct_answer: serde_json::Value,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GradeError {
    #[error("payload does not match question kind {kind}: {reason}")]
    InvalidPayload { kind: QuestionKind, reason: String },
    #[error("response does not match question kind {0}")]
    ResponseKindMismatch(QuestionKind),
    #[error("missing presentation field: {0}")]
    MissingPresentation(&'static str),
    #[error("invalid presentation: {0}")]
    InvalidPresentation(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

/// Grade a user's response to a question.
pub fn grade_response(
    kind: QuestionKind,
    payload: &serde_json::Value,
    response: &AttemptResponse,
    presentation: &AttemptPresentation,
    max_points: i32,
) -> Result<GradeOutcome, GradeError> {
    match (kind, response) {
        (QuestionKind::Mc, AttemptResponse::Mc { selected_position }) => {
            let payload: McPayload = serde_json::from_value(payload.clone()).map_err(|e| {
                GradeError::InvalidPayload {
                    kind,
                    reason: e.to_string(),
                }
            })?;

            let option_order = presentation
                .option_order
                .as_ref()
                .ok_or(GradeError::MissingPresentation("option_order"))?;

            if *selected_position >= option_order.len() {
                return Err(GradeError::InvalidResponse(
                    "selected_position out of bounds".to_string(),
                ));
            }

            let canonical_index = option_order[*selected_position];
            let correct = canonical_index == payload.correct_index;

            Ok(GradeOutcome {
                status: GradeStatus::Graded,
                correct,
                points_awarded: if correct { max_points } else { 0 },
                max: max_points,
                correct_answer: serde_json::json!({ "correct_index": payload.correct_index }),
                note: None,
            })
        }
        (QuestionKind::Tf, AttemptResponse::Tf { answer }) => {
            let payload: TfPayload = serde_json::from_value(payload.clone()).map_err(|e| {
                GradeError::InvalidPayload {
                    kind,
                    reason: e.to_string(),
                }
            })?;

            let correct = *answer == payload.correct;

            Ok(GradeOutcome {
                status: GradeStatus::Graded,
                correct,
                points_awarded: if correct { max_points } else { 0 },
                max: max_points,
                correct_answer: serde_json::json!({ "correct": payload.correct }),
                note: None,
            })
        }
        (QuestionKind::Short, AttemptResponse::Short { answer }) => {
            let payload: ShortPayload = serde_json::from_value(payload.clone()).map_err(|e| {
                GradeError::InvalidPayload {
                    kind,
                    reason: e.to_string(),
                }
            })?;

            let given = match payload.normalize {
                Normalize::Exact => answer.trim().to_string(),
                Normalize::CaseInsensitiveStripAccents => {
                    strip_accents(&answer.trim().to_lowercase())
                }
            };

            let correct = payload.accepted.iter().any(|a| {
                let accepted = match payload.normalize {
                    Normalize::Exact => a.trim().to_string(),
                    Normalize::CaseInsensitiveStripAccents => {
                        strip_accents(&a.trim().to_lowercase())
                    }
                };
                given == accepted
            });

            Ok(GradeOutcome {
                status: GradeStatus::Graded,
                correct,
                points_awarded: if correct { max_points } else { 0 },
                max: max_points,
                correct_answer: serde_json::json!({ "accepted": payload.accepted }),
                note: None,
            })
        }
        (QuestionKind::Essay, AttemptResponse::Essay { body, .. }) => {
            let payload: EssayPayload = serde_json::from_value(payload.clone()).map_err(|e| {
                GradeError::InvalidPayload {
                    kind,
                    reason: e.to_string(),
                }
            })?;

            let word_count = body.split_whitespace().count() as i32;

            if let Some(min_words) = payload.min_words
                && word_count < min_words as i32
            {
                return Ok(GradeOutcome {
                    status: GradeStatus::Graded,
                    correct: false,
                    points_awarded: 0,
                    max: max_points,
                    correct_answer: serde_json::json!({ "rubric": payload.rubric }),
                    note: Some(format!(
                        "Word count {} is below minimum {}",
                        word_count, min_words
                    )),
                });
            }

            Ok(GradeOutcome {
                status: GradeStatus::PendingManual,
                correct: false,
                points_awarded: 0,
                max: max_points,
                correct_answer: serde_json::json!({ "rubric": payload.rubric }),
                note: None,
            })
        }
        (QuestionKind::Code, AttemptResponse::Code { source, .. }) => {
            let payload: CodePayload = serde_json::from_value(payload.clone()).map_err(|e| {
                GradeError::InvalidPayload {
                    kind,
                    reason: e.to_string(),
                }
            })?;

            if let Some(ref exemplar) = payload.exemplar {
                let trimmed_source = source.trim();
                let trimmed_exemplar = exemplar.trim();
                if trimmed_source == trimmed_exemplar {
                    return Ok(GradeOutcome {
                        status: GradeStatus::Graded,
                        correct: true,
                        points_awarded: max_points,
                        max: max_points,
                        correct_answer: serde_json::json!({ "language": payload.language, "exemplar": payload.exemplar }),
                        note: Some("Matches exemplar exactly".to_string()),
                    });
                }
            }

            Ok(GradeOutcome {
                status: GradeStatus::PendingManual,
                correct: false,
                points_awarded: 0,
                max: max_points,
                correct_answer: serde_json::json!({ "language": payload.language, "exemplar": payload.exemplar }),
                note: None,
            })
        }
        _ => Err(GradeError::ResponseKindMismatch(kind)),
    }
}
