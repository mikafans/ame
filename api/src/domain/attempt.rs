//! Attempt domain types.
//!
//! Attempts persist the submitted response, server-owned presentation state,
//! grading result, and rating deltas for a single question attempt.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(untagged)]
pub enum AttemptResponse {
    Mc { selected_position: usize },
    Tf { answer: bool },
    Short { answer: String },
    Essay { body: String, word_count: i32 },
    Code { source: String, language: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct AttemptPresentation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_order: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Attempt {
    pub id: Uuid,
    pub user_id: Uuid,
    pub question_id: Uuid,
    pub question_version: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,
    pub response: AttemptResponse,
    pub presentation: AttemptPresentation,
    pub is_correct: bool,
    pub score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_to_answer_ms: Option<i32>,
    pub rating_before_user_avg: f64,
    pub rating_before_question: f64,
    pub user_tag_deltas: BTreeMap<String, f64>,
    pub question_delta: f64,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mc_response_matches_api_example() {
        let response = AttemptResponse::Mc {
            selected_position: 0,
        };

        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({ "selected_position": 0 })
        );
    }

    #[test]
    fn tf_response_roundtrips() {
        let response: AttemptResponse =
            serde_json::from_value(serde_json::json!({ "answer": true })).unwrap();
        assert_eq!(response, AttemptResponse::Tf { answer: true });
    }

    #[test]
    fn short_response_roundtrips() {
        let response: AttemptResponse =
            serde_json::from_value(serde_json::json!({ "answer": "cafe" })).unwrap();
        assert_eq!(
            response,
            AttemptResponse::Short {
                answer: "cafe".to_string()
            }
        );
    }

    #[test]
    fn essay_response_roundtrips() {
        let response: AttemptResponse =
            serde_json::from_value(serde_json::json!({ "body": "hello world", "word_count": 2 }))
                .unwrap();
        assert_eq!(
            response,
            AttemptResponse::Essay {
                body: "hello world".to_string(),
                word_count: 2
            }
        );
    }

    #[test]
    fn code_response_roundtrips() {
        let response: AttemptResponse = serde_json::from_value(
            serde_json::json!({ "source": "fn main() {}", "language": "rust" }),
        )
        .unwrap();
        assert_eq!(
            response,
            AttemptResponse::Code {
                source: "fn main() {}".to_string(),
                language: "rust".to_string()
            }
        );
    }

    #[test]
    fn attempt_presentation_matches_storage_shape() {
        let presentation = AttemptPresentation {
            option_order: Some(vec![2, 0, 3, 1]),
        };

        assert_eq!(
            serde_json::to_value(presentation).unwrap(),
            serde_json::json!({ "option_order": [2, 0, 3, 1] })
        );
    }
}
