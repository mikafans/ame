//! Lifecycle contracts for application tasks and their learner submissions.

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

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
}
