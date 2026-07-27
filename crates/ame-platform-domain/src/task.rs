//! Lifecycle contracts for application tasks and their learner submissions.

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Scenario,
    StructuredSubmission,
    Project,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Draft,
    Published,
    Retired,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskSubmissionStatus {
    InProgress,
    Submitted,
    InReview,
    Reviewed,
    Rejected,
    Abandoned,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskReviewStatus {
    NotRequired,
    Pending,
    Complete,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskEvaluationMethod {
    SelfReview,
    Automatic,
    Agent,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskRubricCriterion {
    pub id: String,
    pub objective_id: Uuid,
    pub description: String,
    pub max_points: u32,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskRubric {
    pub version: u32,
    pub criteria: Vec<TaskRubricCriterion>,
    pub passing_score: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskSubmissionEnvelope {
    pub task_id: Uuid,
    pub subject_user_id: Uuid,
    pub content_version: u32,
    pub response: serde_json::Value,
    pub evaluation_method: TaskEvaluationMethod,
    pub status: TaskSubmissionStatus,
    pub review_status: TaskReviewStatus,
    pub score: Option<f32>,
    pub feedback: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskContractError {
    EmptyRubric,
    InvalidRubric(String),
    InvalidSubmission(String),
}

impl Display for TaskContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRubric => {
                write!(formatter, "task rubric must contain at least one criterion")
            }
            Self::InvalidRubric(message) => write!(formatter, "invalid task rubric: {message}"),
            Self::InvalidSubmission(message) => {
                write!(formatter, "invalid task submission: {message}")
            }
        }
    }
}

impl std::error::Error for TaskContractError {}

pub fn validate_task_rubric(rubric: &TaskRubric) -> Result<(), TaskContractError> {
    if rubric.version == 0 {
        return Err(TaskContractError::InvalidRubric(
            "version must be greater than zero".to_string(),
        ));
    }
    if rubric.criteria.is_empty() {
        return Err(TaskContractError::EmptyRubric);
    }
    if rubric
        .passing_score
        .is_some_and(|score| !(0.0..=1.0).contains(&score))
    {
        return Err(TaskContractError::InvalidRubric(
            "passing score must be between zero and one".to_string(),
        ));
    }
    let mut criterion_ids = std::collections::HashSet::new();
    for criterion in &rubric.criteria {
        if criterion.id.trim().is_empty() {
            return Err(TaskContractError::InvalidRubric(
                "criterion id must not be empty".to_string(),
            ));
        }
        if !criterion_ids.insert(&criterion.id) {
            return Err(TaskContractError::InvalidRubric(
                "criterion ids must be unique".to_string(),
            ));
        }
        if criterion.description.trim().is_empty() {
            return Err(TaskContractError::InvalidRubric(
                "criterion description must not be empty".to_string(),
            ));
        }
        if criterion.max_points == 0 {
            return Err(TaskContractError::InvalidRubric(
                "criterion max points must be greater than zero".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_task_submission(
    submission: &TaskSubmissionEnvelope,
) -> Result<(), TaskContractError> {
    if submission.content_version == 0 {
        return Err(TaskContractError::InvalidSubmission(
            "content version must be greater than zero".to_string(),
        ));
    }
    if submission.response.is_null() {
        return Err(TaskContractError::InvalidSubmission(
            "response must be present".to_string(),
        ));
    }
    if submission.status == TaskSubmissionStatus::InProgress
        && submission.review_status != TaskReviewStatus::NotRequired
    {
        return Err(TaskContractError::InvalidSubmission(
            "an in-progress submission cannot be under review".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTaskSubmissionTransition {
    pub from: TaskSubmissionStatus,
    pub to: TaskSubmissionStatus,
}

impl Display for InvalidTaskSubmissionTransition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "cannot transition task submission from {:?} to {:?}",
            self.from, self.to
        )
    }
}

impl std::error::Error for InvalidTaskSubmissionTransition {}

pub fn validate_task_submission_transition(
    from: TaskSubmissionStatus,
    to: TaskSubmissionStatus,
) -> Result<(), InvalidTaskSubmissionTransition> {
    let valid = matches!(
        (from, to),
        (
            TaskSubmissionStatus::InProgress,
            TaskSubmissionStatus::Submitted
        ) | (
            TaskSubmissionStatus::InProgress,
            TaskSubmissionStatus::Abandoned
        ) | (
            TaskSubmissionStatus::Submitted,
            TaskSubmissionStatus::InReview
        ) | (
            TaskSubmissionStatus::Submitted,
            TaskSubmissionStatus::Reviewed
        ) | (
            TaskSubmissionStatus::Submitted,
            TaskSubmissionStatus::Rejected
        ) | (
            TaskSubmissionStatus::InReview,
            TaskSubmissionStatus::Reviewed
        ) | (
            TaskSubmissionStatus::InReview,
            TaskSubmissionStatus::Rejected
        ) | (
            TaskSubmissionStatus::Rejected,
            TaskSubmissionStatus::InProgress
        )
    );
    valid
        .then_some(())
        .ok_or(InvalidTaskSubmissionTransition { from, to })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_submission_follows_submit_review_and_revision_paths() {
        for (from, to) in [
            (
                TaskSubmissionStatus::InProgress,
                TaskSubmissionStatus::Submitted,
            ),
            (
                TaskSubmissionStatus::Submitted,
                TaskSubmissionStatus::InReview,
            ),
            (
                TaskSubmissionStatus::InReview,
                TaskSubmissionStatus::Reviewed,
            ),
            (
                TaskSubmissionStatus::Rejected,
                TaskSubmissionStatus::InProgress,
            ),
        ] {
            assert!(validate_task_submission_transition(from, to).is_ok());
        }
    }

    #[test]
    fn completed_or_abandoned_submissions_cannot_be_mutated() {
        for from in [
            TaskSubmissionStatus::Reviewed,
            TaskSubmissionStatus::Abandoned,
        ] {
            assert!(
                validate_task_submission_transition(from, TaskSubmissionStatus::InProgress)
                    .is_err()
            );
            assert!(
                validate_task_submission_transition(from, TaskSubmissionStatus::Submitted).is_err()
            );
        }
    }

    #[test]
    fn rubric_and_submission_capture_verifiable_work() {
        let objective_id = uuid::Uuid::now_v7();
        let rubric = TaskRubric {
            version: 1,
            criteria: vec![TaskRubricCriterion {
                id: "explain-tradeoff".to_string(),
                objective_id,
                description: "Explains the operational tradeoff".to_string(),
                max_points: 4,
                required: true,
            }],
            passing_score: Some(0.75),
        };
        assert!(validate_task_rubric(&rubric).is_ok());

        let submission = TaskSubmissionEnvelope {
            task_id: uuid::Uuid::now_v7(),
            subject_user_id: uuid::Uuid::now_v7(),
            content_version: 3,
            response: serde_json::json!({"answer": "Use event time."}),
            evaluation_method: TaskEvaluationMethod::Agent,
            status: TaskSubmissionStatus::Submitted,
            review_status: TaskReviewStatus::Pending,
            score: None,
            feedback: None,
        };
        assert!(validate_task_submission(&submission).is_ok());
    }

    #[test]
    fn rubric_rejects_duplicate_criteria_and_submission_rejects_null_response() {
        let objective_id = uuid::Uuid::now_v7();
        let criterion = TaskRubricCriterion {
            id: "same".to_string(),
            objective_id,
            description: "Criterion".to_string(),
            max_points: 1,
            required: false,
        };
        let rubric = TaskRubric {
            version: 1,
            criteria: vec![criterion.clone(), criterion],
            passing_score: None,
        };
        assert!(matches!(
            validate_task_rubric(&rubric),
            Err(TaskContractError::InvalidRubric(_))
        ));

        let submission = TaskSubmissionEnvelope {
            task_id: uuid::Uuid::now_v7(),
            subject_user_id: uuid::Uuid::now_v7(),
            content_version: 1,
            response: serde_json::Value::Null,
            evaluation_method: TaskEvaluationMethod::SelfReview,
            status: TaskSubmissionStatus::InProgress,
            review_status: TaskReviewStatus::NotRequired,
            score: None,
            feedback: None,
        };
        assert!(matches!(
            validate_task_submission(&submission),
            Err(TaskContractError::InvalidSubmission(_))
        ));
    }
}
