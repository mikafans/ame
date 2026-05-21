//! Exam and ExamSection domain types.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExamMethod {
    Manual,
    Agent,
}

impl ExamMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExamMethod::Manual => "manual",
            ExamMethod::Agent => "agent",
        }
    }
}

impl std::str::FromStr for ExamMethod {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manual" => Ok(ExamMethod::Manual),
            "agent" => Ok(ExamMethod::Agent),
            other => Err(format!("unknown exam method: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExamStatus {
    Draft,
    Published,
    Archived,
}

impl ExamStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExamStatus::Draft => "draft",
            ExamStatus::Published => "published",
            ExamStatus::Archived => "archived",
        }
    }
}

impl std::str::FromStr for ExamStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(ExamStatus::Draft),
            "published" => Ok(ExamStatus::Published),
            "archived" => Ok(ExamStatus::Archived),
            other => Err(format!("unknown exam status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Exam {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub method: ExamMethod,
    pub status: ExamStatus,
    pub blueprint: serde_json::Value,
    pub duration_min: Option<i32>,
    pub total_points: i32,
    pub passing_points: Option<i32>,
    pub objectives: Vec<String>,
    pub affects_rating: bool,
    pub show_results_during: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composition_trace: Option<serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// A resolved section of an exam.
///
/// Exactly one of `question_ids` (static) or `mix` (dynamic) is non-null.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExamSection {
    pub id: Uuid,
    pub exam_id: Uuid,
    pub title: String,
    pub order_index: i32,
    pub weight: f64,
    pub question_ids: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mix: Option<serde_json::Value>,
    pub items_count: i32,
}
