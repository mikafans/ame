//! Quiz/exam session domain types.
//!
//! These mirror the `sessions` table and the `question_plan` JSON shape from
//! `docs/specs/2026-05-20-harus-platform-design.md`.

use std::{collections::BTreeMap, str::FromStr};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Quiz,
    Exam,
    Practice,
}

impl SessionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            SessionKind::Quiz => "quiz",
            SessionKind::Exam => "exam",
            SessionKind::Practice => "practice",
        }
    }
}

impl std::fmt::Display for SessionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown session kind: {0}")]
pub struct UnknownSessionKind(pub String);

impl FromStr for SessionKind {
    type Err = UnknownSessionKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "quiz" => Ok(SessionKind::Quiz),
            "exam" => Ok(SessionKind::Exam),
            "practice" => Ok(SessionKind::Practice),
            other => Err(UnknownSessionKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    InProgress,
    Finished,
    Abandoned,
}

impl SessionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            SessionStatus::InProgress => "in_progress",
            SessionStatus::Finished => "finished",
            SessionStatus::Abandoned => "abandoned",
        }
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown session status: {0}")]
pub struct UnknownSessionStatus(pub String);

impl FromStr for SessionStatus {
    type Err = UnknownSessionStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in_progress" => Ok(SessionStatus::InProgress),
            "finished" => Ok(SessionStatus::Finished),
            "abandoned" => Ok(SessionStatus::Abandoned),
            other => Err(UnknownSessionStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct QuestionPlan {
    pub items: Vec<PlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PlanItem {
    pub question_id: Uuid,
    pub version: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_order: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: SessionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiz_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exam_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub filter: Option<serde_json::Value>,
    pub question_plan: QuestionPlan,
    pub status: SessionStatus,
    pub affects_rating: bool,
    #[schema(value_type = Object)]
    pub rating_snapshot: BTreeMap<Uuid, f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub deadline_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub started_at: OffsetDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub finished_at: Option<OffsetDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiz_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub course_title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_kind_string_roundtrip() {
        for kind in [SessionKind::Quiz, SessionKind::Exam, SessionKind::Practice] {
            let parsed: SessionKind = kind.as_str().parse().expect("known kind parses");
            assert_eq!(parsed, kind);
            assert_eq!(kind.to_string(), kind.as_str());
        }
    }

    #[test]
    fn session_kind_rejects_unknown() {
        let err = "drill".parse::<SessionKind>().unwrap_err();
        assert_eq!(err.0, "drill");
    }

    #[test]
    fn session_status_string_roundtrip() {
        for status in [
            SessionStatus::InProgress,
            SessionStatus::Finished,
            SessionStatus::Abandoned,
        ] {
            let parsed: SessionStatus = status.as_str().parse().expect("known status parses");
            assert_eq!(parsed, status);
            assert_eq!(status.to_string(), status.as_str());
        }
    }

    #[test]
    fn question_plan_matches_spec_shape() {
        let question_id = Uuid::now_v7();
        let plan = QuestionPlan {
            items: vec![PlanItem {
                question_id,
                version: 3,
                section: Some("rust".to_string()),
                option_order: Some(vec![2, 0, 1]),
            }],
        };

        assert_eq!(
            serde_json::to_value(plan).unwrap(),
            serde_json::json!({
                "items": [{
                    "question_id": question_id,
                    "version": 3,
                    "section": "rust",
                    "option_order": [2, 0, 1]
                }]
            })
        );
    }
}
