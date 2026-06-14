//! Question, tag, version, and payload domain types.
//!
//! These mirror the storage shape in `db/migrations/20260519092355_init.sql`
//! and the wire shapes defined in
//! `docs/specs/2026-05-20-harus-platform-design.md`.
//!
//! Shared by `bank/` (storage) and `http/` (wire).

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    Mc,
    Tf,
    Short,
    Essay,
    Code,
}

impl QuestionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionKind::Mc => "mc",
            QuestionKind::Tf => "tf",
            QuestionKind::Short => "short",
            QuestionKind::Essay => "essay",
            QuestionKind::Code => "code",
        }
    }
}

impl std::fmt::Display for QuestionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for QuestionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mc" => Ok(QuestionKind::Mc),
            "tf" => Ok(QuestionKind::Tf),
            "short" => Ok(QuestionKind::Short),
            "essay" => Ok(QuestionKind::Essay),
            "code" => Ok(QuestionKind::Code),
            _ => Err(format!("unknown question kind: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Draft,
    Live,
    Archived,
}

impl QuestionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionStatus::Draft => "draft",
            QuestionStatus::Live => "live",
            QuestionStatus::Archived => "archived",
        }
    }
}

impl FromStr for QuestionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(QuestionStatus::Draft),
            "live" => Ok(QuestionStatus::Live),
            "archived" => Ok(QuestionStatus::Archived),
            _ => Err(format!("unknown question status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CodeSnippet {
    pub language: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: Uuid,
    pub kind: QuestionKind,
    pub prompt: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub version: i32,
    pub status: QuestionStatus,
    pub points: i32,
    pub code_snippet: Option<serde_json::Value>,
    pub payload: serde_json::Value,
    pub explanation: Option<String>,
    pub deep_dive: Option<String>,
    pub source: Option<String>,
    pub rating: f64,
    pub attempts_count: i32,
    pub created_by: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVersion {
    pub id: Uuid,
    pub question_id: Uuid,
    pub version: i32,
    pub prompt: String,
    pub code_snippet: Option<serde_json::Value>,
    pub payload: serde_json::Value,
    pub explanation: Option<String>,
    pub deep_dive: Option<String>,
    pub archived_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedQuestion {
    pub id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub archived_at: OffsetDateTime,
}

pub fn deserialize_mc_options<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
    let mut options = Vec::new();
    for v in values {
        match v {
            serde_json::Value::String(s) => options.push(s),
            serde_json::Value::Object(ref obj) => {
                if let Some(serde_json::Value::String(s)) = obj.get("text") {
                    options.push(s.clone());
                } else if let Some(serde_json::Value::String(s)) = obj.get("label") {
                    options.push(s.clone());
                } else {
                    options.push(v.to_string());
                }
            }
            _ => options.push(v.to_string()),
        }
    }
    Ok(options)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct McPayload {
    #[serde(deserialize_with = "deserialize_mc_options")]
    pub options: Vec<String>,
    pub correct_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TfPayload {
    pub correct: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Normalize {
    Exact,
    CaseInsensitiveStripAccents,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Judge {
    Exact,
    Aigc,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShortPayload {
    pub accepted: Vec<String>,
    pub normalize: Normalize,
    pub judge: Judge,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EssayPayload {
    pub min_words: Option<usize>,
    pub rubric: Option<String>,
    pub judge: Judge,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CodePayload {
    pub language: String,
    pub starter: String,
    pub tests: Vec<CodeTest>,
    pub exemplar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CodeTest {
    pub name: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_serialization() {
        let exact = serde_json::to_string(&Normalize::Exact).unwrap();
        let ci = serde_json::to_string(&Normalize::CaseInsensitiveStripAccents).unwrap();
        assert_eq!(exact, "\"exact\"");
        assert_eq!(ci, "\"case_insensitive_strip_accents\"");
    }
}
