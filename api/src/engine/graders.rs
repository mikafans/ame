//! Pure grading logic for question attempts.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::attempt::{AttemptPresentation, AttemptResponse};
use crate::domain::question::{
    CodePayload, EssayPayload, Judge, McPayload, Normalize, QuestionKind, ShortPayload, TfPayload,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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
    #[error("invalid max points: {0}")]
    InvalidMaxPoints(i32),
}

pub fn grade_response(
    kind: QuestionKind,
    payload: &serde_json::Value,
    response: &AttemptResponse,
    presentation: &AttemptPresentation,
    max_points: i32,
) -> Result<GradeOutcome, GradeError> {
    if max_points < 0 {
        return Err(GradeError::InvalidMaxPoints(max_points));
    }

    match kind {
        QuestionKind::Mc => grade_mc(
            decode_payload(kind, payload)?,
            response,
            presentation,
            max_points,
        ),
        QuestionKind::Tf => grade_tf(decode_payload(kind, payload)?, response, max_points),
        QuestionKind::Short => grade_short(decode_payload(kind, payload)?, response, max_points),
        QuestionKind::Essay => grade_essay(decode_payload(kind, payload)?, response, max_points),
        QuestionKind::Code => grade_code(decode_payload(kind, payload)?, response, max_points),
    }
}

fn decode_payload<T>(kind: QuestionKind, payload: &serde_json::Value) -> Result<T, GradeError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(payload.clone()).map_err(|e| GradeError::InvalidPayload {
        kind,
        reason: e.to_string(),
    })
}

fn grade_mc(
    payload: McPayload,
    response: &AttemptResponse,
    presentation: &AttemptPresentation,
    max_points: i32,
) -> Result<GradeOutcome, GradeError> {
    let selected_position = match response {
        AttemptResponse::Mc { selected_position } => *selected_position,
        _ => return Err(GradeError::ResponseKindMismatch(QuestionKind::Mc)),
    };

    if payload.correct_index >= payload.options.len() {
        return Err(GradeError::InvalidPayload {
            kind: QuestionKind::Mc,
            reason: "correct_index is outside options".to_string(),
        });
    }

    let option_order = presentation
        .option_order
        .as_ref()
        .ok_or(GradeError::MissingPresentation("option_order"))?;

    if selected_position >= option_order.len() {
        return Err(GradeError::InvalidResponse(format!(
            "selected_position {selected_position} is outside option_order"
        )));
    }

    if option_order.len() != payload.options.len() {
        return Err(GradeError::InvalidPresentation(
            "option_order length must match options length".to_string(),
        ));
    }

    let canonical_index = option_order[selected_position];
    if canonical_index >= payload.options.len() {
        return Err(GradeError::InvalidPresentation(
            "option_order contains index outside options".to_string(),
        ));
    }

    let correct = canonical_index == payload.correct_index;
    Ok(binary_outcome(
        correct,
        max_points,
        serde_json::json!({ "correct_index": payload.correct_index }),
    ))
}

fn grade_tf(
    payload: TfPayload,
    response: &AttemptResponse,
    max_points: i32,
) -> Result<GradeOutcome, GradeError> {
    let answer = match response {
        AttemptResponse::Tf { answer } => *answer,
        _ => return Err(GradeError::ResponseKindMismatch(QuestionKind::Tf)),
    };

    Ok(binary_outcome(
        answer == payload.correct,
        max_points,
        serde_json::json!({ "correct": payload.correct }),
    ))
}

fn grade_short(
    payload: ShortPayload,
    response: &AttemptResponse,
    max_points: i32,
) -> Result<GradeOutcome, GradeError> {
    let answer = match response {
        AttemptResponse::Short { answer } => answer,
        _ => return Err(GradeError::ResponseKindMismatch(QuestionKind::Short)),
    };

    if payload.judge != Judge::Exact {
        return Err(GradeError::InvalidPayload {
            kind: QuestionKind::Short,
            reason: "only exact judge is supported for short answers".to_string(),
        });
    }

    let normalized_answer = normalize(answer, payload.normalize);
    let correct = payload
        .accepted
        .iter()
        .any(|accepted| normalize(accepted, payload.normalize) == normalized_answer);

    Ok(binary_outcome(
        correct,
        max_points,
        serde_json::json!({ "accepted": payload.accepted }),
    ))
}

fn grade_essay(
    payload: EssayPayload,
    response: &AttemptResponse,
    max_points: i32,
) -> Result<GradeOutcome, GradeError> {
    let word_count = match response {
        AttemptResponse::Essay { word_count, .. } => *word_count,
        _ => return Err(GradeError::ResponseKindMismatch(QuestionKind::Essay)),
    };

    if payload.judge != Judge::Manual {
        return Err(GradeError::InvalidPayload {
            kind: QuestionKind::Essay,
            reason: "v1 only supports manual essay grading".to_string(),
        });
    }

    if let Some(min_words) = payload.min_words
        && word_count < min_words
    {
        return Ok(GradeOutcome {
            status: GradeStatus::Graded,
            correct: false,
            points_awarded: 0,
            max: max_points,
            note: Some(format!("minimum word count is {min_words}")),
            correct_answer: serde_json::json!({ "rubric": payload.rubric }),
        });
    }

    Ok(GradeOutcome {
        status: GradeStatus::PendingManual,
        correct: false,
        points_awarded: 0,
        max: max_points,
        note: Some("manual grading required".to_string()),
        correct_answer: serde_json::json!({ "rubric": payload.rubric }),
    })
}

fn grade_code(
    payload: CodePayload,
    response: &AttemptResponse,
    max_points: i32,
) -> Result<GradeOutcome, GradeError> {
    let source = match response {
        AttemptResponse::Code { source, language } if language == &payload.language => source,
        AttemptResponse::Code { .. } => {
            return Err(GradeError::InvalidResponse(
                "response language must match payload language".to_string(),
            ));
        }
        _ => return Err(GradeError::ResponseKindMismatch(QuestionKind::Code)),
    };

    let Some(exemplar) = payload.exemplar else {
        return Ok(GradeOutcome {
            status: GradeStatus::PendingManual,
            correct: false,
            points_awarded: 0,
            max: max_points,
            note: Some("code runner is not configured".to_string()),
            correct_answer: serde_json::json!({ "language": payload.language }),
        });
    };

    Ok(binary_outcome(
        source.trim() == exemplar.trim(),
        max_points,
        serde_json::json!({ "exemplar": exemplar }),
    ))
}

fn binary_outcome(
    correct: bool,
    max_points: i32,
    correct_answer: serde_json::Value,
) -> GradeOutcome {
    GradeOutcome {
        status: GradeStatus::Graded,
        correct,
        points_awarded: if correct { max_points } else { 0 },
        max: max_points,
        note: None,
        correct_answer,
    }
}

fn normalize(value: &str, rule: Normalize) -> String {
    match rule {
        Normalize::Exact => value.to_string(),
        Normalize::CaseInsensitiveStripAccents => value
            .chars()
            .flat_map(|c| strip_latin_accent(c).to_lowercase())
            .collect(),
    }
}

fn strip_latin_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å'
        | 'Ā' | 'Ă' | 'Ą' => 'a',
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' | 'Ç' | 'Ć' | 'Ĉ' | 'Ċ' | 'Č' => 'c',
        'ď' | 'đ' | 'Ď' | 'Đ' => 'd',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' | 'É' | 'È' | 'Ê' | 'Ë' | 'Ē' | 'Ĕ'
        | 'Ė' | 'Ę' | 'Ě' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'İ' | 'Í' | 'Ì' | 'Î' | 'Ï' | 'Ĩ' | 'Ī'
        | 'Ĭ' | 'Į' => 'i',
        'ñ' | 'ń' | 'ņ' | 'ň' | 'Ñ' | 'Ń' | 'Ņ' | 'Ň' => 'n',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' | 'Ø'
        | 'Ō' | 'Ŏ' | 'Ő' => 'o',
        'ŕ' | 'ŗ' | 'ř' | 'Ŕ' | 'Ŗ' | 'Ř' => 'r',
        'ś' | 'ŝ' | 'ş' | 'š' | 'Ś' | 'Ŝ' | 'Ş' | 'Š' => 's',
        'ť' | 'ţ' | 'ŧ' | 'Ť' | 'Ţ' | 'Ŧ' => 't',
        'ú' | 'ù' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' | 'Ú' | 'Ù' | 'Û' | 'Ü' | 'Ũ'
        | 'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' => 'u',
        'ý' | 'ÿ' | 'ŷ' | 'Ý' | 'Ÿ' | 'Ŷ' => 'y',
        'ź' | 'ż' | 'ž' | 'Ź' | 'Ż' | 'Ž' => 'z',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mc_payload(correct_index: usize) -> serde_json::Value {
        serde_json::json!({
            "options": ["red", "green", "blue"],
            "correct_index": correct_index
        })
    }

    #[test]
    fn mc_resolves_selected_position_through_server_option_order() {
        let outcome = grade_response(
            QuestionKind::Mc,
            &mc_payload(1),
            &AttemptResponse::Mc {
                selected_position: 0,
            },
            &AttemptPresentation {
                option_order: Some(vec![1, 2, 0]),
            },
            3,
        )
        .unwrap();

        assert_eq!(outcome.status, GradeStatus::Graded);
        assert!(outcome.correct);
        assert_eq!(outcome.points_awarded, 3);
        assert_eq!(outcome.max, 3);
    }

    #[test]
    fn mc_rejects_missing_option_order() {
        let err = grade_response(
            QuestionKind::Mc,
            &mc_payload(1),
            &AttemptResponse::Mc {
                selected_position: 0,
            },
            &AttemptPresentation::default(),
            1,
        )
        .unwrap_err();

        assert_eq!(err, GradeError::MissingPresentation("option_order"));
    }

    #[test]
    fn tf_scores_boolean_match() {
        let outcome = grade_response(
            QuestionKind::Tf,
            &serde_json::json!({ "correct": true }),
            &AttemptResponse::Tf { answer: true },
            &AttemptPresentation::default(),
            2,
        )
        .unwrap();

        assert!(outcome.correct);
        assert_eq!(outcome.points_awarded, 2);
    }

    #[test]
    fn short_normalizes_both_sides() {
        let payload = serde_json::json!({
            "accepted": ["café"],
            "normalize": "case_insensitive_strip_accents",
            "judge": "exact"
        });

        let outcome = grade_response(
            QuestionKind::Short,
            &payload,
            &AttemptResponse::Short {
                answer: "CAFE".to_string(),
            },
            &AttemptPresentation::default(),
            1,
        )
        .unwrap();

        assert!(outcome.correct);
        assert_eq!(outcome.points_awarded, 1);
    }

    #[test]
    fn exact_short_keeps_case_and_accents() {
        let payload = serde_json::json!({
            "accepted": ["café"],
            "normalize": "exact",
            "judge": "exact"
        });

        let outcome = grade_response(
            QuestionKind::Short,
            &payload,
            &AttemptResponse::Short {
                answer: "CAFE".to_string(),
            },
            &AttemptPresentation::default(),
            1,
        )
        .unwrap();

        assert!(!outcome.correct);
        assert_eq!(outcome.points_awarded, 0);
    }

    #[test]
    fn essay_returns_pending_manual_when_word_count_is_acceptable() {
        let outcome = grade_response(
            QuestionKind::Essay,
            &serde_json::json!({
                "min_words": 2,
                "rubric": "Clear reasoning",
                "judge": "manual"
            }),
            &AttemptResponse::Essay {
                body: "hello world".to_string(),
                word_count: 2,
            },
            &AttemptPresentation::default(),
            5,
        )
        .unwrap();

        assert_eq!(outcome.status, GradeStatus::PendingManual);
        assert_eq!(outcome.points_awarded, 0);
        assert_eq!(outcome.max, 5);
    }

    #[test]
    fn code_stub_compares_against_exemplar() {
        let outcome = grade_response(
            QuestionKind::Code,
            &serde_json::json!({
                "language": "rust",
                "starter": "fn main() {}",
                "tests": [],
                "exemplar": "fn main() { println!(\"ok\"); }"
            }),
            &AttemptResponse::Code {
                source: "fn main() { println!(\"ok\"); }".to_string(),
                language: "rust".to_string(),
            },
            &AttemptPresentation::default(),
            4,
        )
        .unwrap();

        assert_eq!(outcome.status, GradeStatus::Graded);
        assert!(outcome.correct);
        assert_eq!(outcome.points_awarded, 4);
    }
}
