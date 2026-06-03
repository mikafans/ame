//! Assessment domain types.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentMode {
    #[default]
    Practice,
    Graded,
}

impl std::fmt::Display for AssessmentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssessmentMode::Practice => write!(f, "practice"),
            AssessmentMode::Graded => write!(f, "graded"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Draft,
    Active,
    Archived,
}

impl std::fmt::Display for AssessmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssessmentStatus::Draft => write!(f, "draft"),
            AssessmentStatus::Active => write!(f, "active"),
            AssessmentStatus::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub mode: AssessmentMode,
    pub status: AssessmentStatus,
    pub objectives: Vec<String>,
    pub course: Option<String>,
    pub duration_min: Option<i32>,
    pub time_limit_seconds: Option<i32>,
    pub total_points: i32,
    pub passing_points: Option<i32>,
    pub show_results_during: bool,
    pub affects_rating: bool,
    pub method: String,
    pub composition_trace: Option<serde_json::Value>,
    pub created_by: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuestionImport {
    pub kind: crate::domain::question::QuestionKind,
    pub prompt: String,
    pub payload: serde_json::Value,
    pub explanation: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub points: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssessmentRequest {
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub mode: AssessmentMode,
    #[serde(default)]
    pub objectives: Vec<String>,
    pub course: Option<String>,
    pub duration_min: Option<i32>,
    pub time_limit_seconds: Option<i32>,
    pub passing_points: Option<i32>,
    #[serde(default)]
    pub show_results_during: bool,
    #[serde(default = "default_true")]
    pub affects_rating: bool,
    #[serde(default = "default_method")]
    pub method: String, // 'manual' or 'agent'
    #[serde(default)]
    pub questions: Vec<QuestionImport>,
    #[serde(default)]
    pub status: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_method() -> String {
    "manual".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssessmentRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<AssessmentStatus>,
    pub objectives: Option<Vec<String>>,
}
