//! Question, tag, version, and payload domain types.
//!
//! These mirror the storage shape in `db/migrations/20260519092355_init.sql`
//! and the wire shapes defined in
//! `docs/specs/2026-05-19-question-exam-platform-design.md`.
//!
//! The `kind` / `status` enums are the single point of translation between the
//! Postgres `text` columns (constrained by CHECK clauses) and Rust. If the DB
//! constraint changes, update the `FromStr` impls and the `as_str` arms in
//! lockstep — see the same pattern in [`crate::domain::user::Scope`].
//!
//! Payload shapes are kept as separate per-kind structs. `Question::payload`
//! stays as `serde_json::Value` to match the jsonb column 1:1; typed decoding
//! into [`McqPayload`] / [`FreeTextPayload`] / [`ClozePayload`] happens at the
//! repository / HTTP boundary.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    Mcq,
    FreeText,
    Cloze,
}

impl QuestionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            QuestionKind::Mcq => "mcq",
            QuestionKind::FreeText => "free_text",
            QuestionKind::Cloze => "cloze",
        }
    }
}

impl std::fmt::Display for QuestionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown question kind: {0}")]
pub struct UnknownQuestionKind(pub String);

impl FromStr for QuestionKind {
    type Err = UnknownQuestionKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mcq" => Ok(QuestionKind::Mcq),
            "free_text" => Ok(QuestionKind::FreeText),
            "cloze" => Ok(QuestionKind::Cloze),
            other => Err(UnknownQuestionKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Draft,
    Live,
    Archived,
}

impl QuestionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            QuestionStatus::Draft => "draft",
            QuestionStatus::Live => "live",
            QuestionStatus::Archived => "archived",
        }
    }
}

impl std::fmt::Display for QuestionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown question status: {0}")]
pub struct UnknownQuestionStatus(pub String);

impl FromStr for QuestionStatus {
    type Err = UnknownQuestionStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(QuestionStatus::Draft),
            "live" => Ok(QuestionStatus::Live),
            "archived" => Ok(QuestionStatus::Archived),
            other => Err(UnknownQuestionStatus(other.to_string())),
        }
    }
}

/// Normalization rule applied to *both* the stored accepted answers and the
/// user's response at grade time. See spec §"Normalize semantics".
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Normalize {
    Exact,
    CaseInsensitiveStripAccents,
}

/// Free-text judge. Only `exact` is supported in MVP; `llm` is reserved for
/// the deferred LLM-as-judge feature.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Judge {
    Exact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct CodeSnippet {
    pub language: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct McqPayload {
    pub options: Vec<String>,
    pub correct_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct FreeTextPayload {
    pub accepted: Vec<String>,
    pub normalize: Normalize,
    pub judge: Judge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ClozePayload {
    pub template: String,
    pub blanks: Vec<Vec<String>>,
    pub normalize: Normalize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Question {
    pub id: Uuid,
    pub kind: QuestionKind,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<CodeSnippet>,
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    pub status: QuestionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub rating: f64,
    pub attempts_count: i32,
    pub version: i32,
    pub created_by: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

/// Historical snapshot of a question's editable fields, written when a `live`
/// question is edited (see spec §"Versioning rule").
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuestionVersion {
    pub question_id: Uuid,
    pub version: i32,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<CodeSnippet>,
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub archived_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn question_kind_string_roundtrip() {
        for kind in [
            QuestionKind::Mcq,
            QuestionKind::FreeText,
            QuestionKind::Cloze,
        ] {
            let parsed: QuestionKind = kind.as_str().parse().expect("known kind should parse");
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn question_kind_rejects_unknown() {
        let err = "code".parse::<QuestionKind>().unwrap_err();
        assert_eq!(err.0, "code");
    }

    #[test]
    fn question_kind_serde_matches_db_strings() {
        assert_eq!(
            serde_json::to_string(&QuestionKind::FreeText).unwrap(),
            "\"free_text\""
        );
        let parsed: QuestionKind = serde_json::from_str("\"mcq\"").unwrap();
        assert_eq!(parsed, QuestionKind::Mcq);
    }

    #[test]
    fn question_status_string_roundtrip() {
        for status in [
            QuestionStatus::Draft,
            QuestionStatus::Live,
            QuestionStatus::Archived,
        ] {
            let parsed: QuestionStatus = status.as_str().parse().expect("known status parses");
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn question_status_rejects_unknown() {
        let err = "retired".parse::<QuestionStatus>().unwrap_err();
        assert_eq!(err.0, "retired");
    }

    #[test]
    fn mcq_payload_serde() {
        let p = McqPayload {
            options: vec!["a".into(), "b".into(), "c".into()],
            correct_index: 1,
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["correct_index"], 1);
        let back: McqPayload = serde_json::from_value(json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn free_text_payload_normalize_variants_match_spec() {
        let exact = serde_json::to_string(&Normalize::Exact).unwrap();
        let ci = serde_json::to_string(&Normalize::CaseInsensitiveStripAccents).unwrap();
        assert_eq!(exact, "\"exact\"");
        assert_eq!(ci, "\"case_insensitive_strip_accents\"");
    }
}
