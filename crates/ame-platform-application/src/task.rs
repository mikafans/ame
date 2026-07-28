use crate::domain::task::{
    InvalidTaskSubmissionTransition, TaskEvaluationMethod, TaskReviewStatus,
    TaskSubmissionEnvelope, TaskSubmissionStatus, validate_task_submission,
    validate_task_submission_transition,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct TaskSubmission {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub revision: u32,
    pub parent_submission_id: Option<Uuid>,
    pub reviewer_user_id: Option<Uuid>,
    pub rubric_scores: Option<Vec<crate::domain::task::RubricScore>>,
    pub review_provenance: Option<serde_json::Value>,
    pub rubric: Option<crate::domain::task::TaskRubric>,
    pub envelope: TaskSubmissionEnvelope,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartTaskSubmission {
    pub subject_user_id: Uuid,
    pub task_id: Uuid,
    pub content_version: u32,
    pub response: serde_json::Value,
    pub artifacts: Vec<crate::domain::task::SubmissionArtifact>,
    pub evaluation_method: TaskEvaluationMethod,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewTaskSubmission {
    pub reviewer_user_id: Uuid,
    pub submission_id: Uuid,
    pub outcome: TaskReviewOutcome,
    pub score: Option<f32>,
    pub feedback: Option<serde_json::Value>,
    pub rubric_scores: Option<Vec<crate::domain::task::RubricScore>>,
    pub review_provenance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskReviewOutcome {
    Reviewed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSubmissionError {
    NotFound,
    SubjectMismatch,
    StaleContentVersion,
    InvalidContract(String),
    InvalidTransition(InvalidTaskSubmissionTransition),
    Storage(String),
}

impl Display for TaskSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(formatter, "task or submission was not found"),
            Self::SubjectMismatch => write!(formatter, "task belongs to another subject"),
            Self::StaleContentVersion => write!(formatter, "task content version is stale"),
            Self::InvalidContract(message) => {
                write!(formatter, "invalid task submission: {message}")
            }
            Self::InvalidTransition(error) => error.fmt(formatter),
            Self::Storage(message) => {
                write!(formatter, "task submission storage failure: {message}")
            }
        }
    }
}

impl std::error::Error for TaskSubmissionError {}

#[async_trait]
pub trait TaskSubmissionRepository: Send + Sync {
    async fn start(
        &self,
        input: StartTaskSubmission,
    ) -> Result<TaskSubmission, TaskSubmissionError>;

    async fn submit(
        &self,
        subject_user_id: Uuid,
        submission_id: Uuid,
    ) -> Result<TaskSubmission, TaskSubmissionError>;

    async fn review(
        &self,
        input: ReviewTaskSubmission,
    ) -> Result<TaskSubmission, TaskSubmissionError>;

    async fn revise(
        &self,
        subject_user_id: Uuid,
        submission_id: Uuid,
        response: serde_json::Value,
        artifacts: Vec<crate::domain::task::SubmissionArtifact>,
    ) -> Result<TaskSubmission, TaskSubmissionError>;
}

#[derive(Clone, Default)]
pub struct InMemoryTaskSubmissionRepository {
    state: Arc<Mutex<TaskState>>,
}

#[derive(Default)]
struct TaskState {
    tasks: HashMap<Uuid, TaskFixture>,
    submissions: HashMap<Uuid, TaskSubmission>,
}

#[derive(Debug, Clone, Copy)]
struct TaskFixture {
    subject_user_id: Uuid,
    journey_id: Uuid,
    content_version: u32,
}

impl InMemoryTaskSubmissionRepository {
    pub fn register_task(
        &self,
        task_id: Uuid,
        subject_user_id: Uuid,
        journey_id: Uuid,
        content_version: u32,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.tasks.insert(
                task_id,
                TaskFixture {
                    subject_user_id,
                    journey_id,
                    content_version,
                },
            );
        }
    }
}

#[async_trait]
impl TaskSubmissionRepository for InMemoryTaskSubmissionRepository {
    async fn start(
        &self,
        input: StartTaskSubmission,
    ) -> Result<TaskSubmission, TaskSubmissionError> {
        if input.response.is_null() {
            return Err(TaskSubmissionError::InvalidContract(
                "response must be present".into(),
            ));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|error| TaskSubmissionError::Storage(error.to_string()))?;
        let task = state
            .tasks
            .get(&input.task_id)
            .ok_or(TaskSubmissionError::NotFound)?;
        if task.subject_user_id != input.subject_user_id {
            return Err(TaskSubmissionError::SubjectMismatch);
        }
        if task.content_version != input.content_version {
            return Err(TaskSubmissionError::StaleContentVersion);
        }
        let envelope = TaskSubmissionEnvelope {
            task_id: input.task_id,
            subject_user_id: input.subject_user_id,
            content_version: input.content_version,
            response: input.response,
            artifacts: input.artifacts,
            evaluation_method: input.evaluation_method,
            status: TaskSubmissionStatus::InProgress,
            review_status: TaskReviewStatus::NotRequired,
            score: None,
            feedback: None,
        };
        validate_task_submission(&envelope)
            .map_err(|error| TaskSubmissionError::InvalidContract(error.to_string()))?;
        let submission = TaskSubmission {
            id: Uuid::now_v7(),
            journey_id: task.journey_id,
            revision: 1,
            parent_submission_id: None,
            reviewer_user_id: None,
            rubric_scores: None,
            review_provenance: None,
            rubric: None,
            envelope,
        };
        state.submissions.insert(submission.id, submission.clone());
        Ok(submission)
    }

    async fn submit(
        &self,
        subject_user_id: Uuid,
        submission_id: Uuid,
    ) -> Result<TaskSubmission, TaskSubmissionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| TaskSubmissionError::Storage(error.to_string()))?;
        let submission = state
            .submissions
            .get_mut(&submission_id)
            .ok_or(TaskSubmissionError::NotFound)?;
        if submission.envelope.subject_user_id != subject_user_id {
            return Err(TaskSubmissionError::SubjectMismatch);
        }
        submission.envelope.status = validate_task_submission_transition(
            submission.envelope.status,
            TaskSubmissionStatus::Submitted,
        )
        .map(|_| TaskSubmissionStatus::Submitted)
        .map_err(TaskSubmissionError::InvalidTransition)?;
        if submission.envelope.evaluation_method == TaskEvaluationMethod::SelfReview {
            submission.envelope.review_status = TaskReviewStatus::Complete;
            submission.envelope.status = TaskSubmissionStatus::Reviewed;
        } else {
            submission.envelope.review_status = TaskReviewStatus::Pending;
        }
        Ok(submission.clone())
    }

    async fn review(
        &self,
        input: ReviewTaskSubmission,
    ) -> Result<TaskSubmission, TaskSubmissionError> {
        if input
            .score
            .is_some_and(|score| !(0.0..=1.0).contains(&score))
        {
            return Err(TaskSubmissionError::InvalidContract(
                "score must be between zero and one".into(),
            ));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|error| TaskSubmissionError::Storage(error.to_string()))?;
        let submission = state
            .submissions
            .get_mut(&input.submission_id)
            .ok_or(TaskSubmissionError::NotFound)?;
        let next_status = match input.outcome {
            TaskReviewOutcome::Reviewed => TaskSubmissionStatus::Reviewed,
            TaskReviewOutcome::Rejected => TaskSubmissionStatus::Rejected,
        };
        validate_task_submission_transition(submission.envelope.status, next_status)
            .map_err(TaskSubmissionError::InvalidTransition)?;
        submission.envelope.status = next_status;
        submission.envelope.review_status = TaskReviewStatus::Complete;
        submission.envelope.score = input.score;
        submission.envelope.feedback = input.feedback;
        submission.reviewer_user_id = Some(input.reviewer_user_id);
        submission.rubric_scores = input.rubric_scores;
        submission.review_provenance = input.review_provenance;
        Ok(submission.clone())
    }

    async fn revise(
        &self,
        subject_user_id: Uuid,
        submission_id: Uuid,
        response: serde_json::Value,
        artifacts: Vec<crate::domain::task::SubmissionArtifact>,
    ) -> Result<TaskSubmission, TaskSubmissionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| TaskSubmissionError::Storage(error.to_string()))?;
        let previous = state
            .submissions
            .get(&submission_id)
            .cloned()
            .ok_or(TaskSubmissionError::NotFound)?;
        if previous.envelope.subject_user_id != subject_user_id {
            return Err(TaskSubmissionError::SubjectMismatch);
        }
        validate_task_submission_transition(
            previous.envelope.status,
            TaskSubmissionStatus::InProgress,
        )
        .map_err(TaskSubmissionError::InvalidTransition)?;
        if let Some(existing) = state
            .submissions
            .values()
            .find(|candidate| candidate.parent_submission_id == Some(submission_id))
        {
            if existing.envelope.response == response && existing.envelope.artifacts == artifacts {
                return Ok(existing.clone());
            }
            return Err(TaskSubmissionError::InvalidContract(
                "revision payload conflicts with the existing successor".into(),
            ));
        }
        let mut envelope = previous.envelope.clone();
        envelope.response = response;
        envelope.artifacts = artifacts;
        envelope.status = TaskSubmissionStatus::InProgress;
        envelope.review_status = TaskReviewStatus::NotRequired;
        envelope.score = None;
        envelope.feedback = None;
        validate_task_submission(&envelope)
            .map_err(|error| TaskSubmissionError::InvalidContract(error.to_string()))?;
        let revision = TaskSubmission {
            id: Uuid::now_v7(),
            journey_id: previous.journey_id,
            revision: previous.revision + 1,
            parent_submission_id: Some(previous.id),
            reviewer_user_id: None,
            rubric_scores: None,
            review_provenance: None,
            rubric: previous.rubric,
            envelope,
        };
        state.submissions.insert(revision.id, revision.clone());
        Ok(revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scenario_submission_requires_current_owned_task_version() {
        let repository = InMemoryTaskSubmissionRepository::default();
        let owner = Uuid::now_v7();
        let journey = Uuid::now_v7();
        let task = Uuid::now_v7();
        repository.register_task(task, owner, journey, 2);
        let stale = repository
            .start(StartTaskSubmission {
                subject_user_id: owner,
                task_id: task,
                content_version: 1,
                response: serde_json::json!({"choice": "late-events"}),
                artifacts: vec![],
                evaluation_method: TaskEvaluationMethod::SelfReview,
            })
            .await;
        assert_eq!(stale, Err(TaskSubmissionError::StaleContentVersion));
        let foreign = repository
            .start(StartTaskSubmission {
                subject_user_id: Uuid::now_v7(),
                task_id: task,
                content_version: 2,
                response: serde_json::json!({"choice": "late-events"}),
                artifacts: vec![],
                evaluation_method: TaskEvaluationMethod::SelfReview,
            })
            .await;
        assert_eq!(foreign, Err(TaskSubmissionError::SubjectMismatch));
    }

    #[tokio::test]
    async fn self_review_submission_reaches_reviewed_state() {
        let repository = InMemoryTaskSubmissionRepository::default();
        let owner = Uuid::now_v7();
        let task = Uuid::now_v7();
        repository.register_task(task, owner, Uuid::now_v7(), 1);
        let submission = repository
            .start(StartTaskSubmission {
                subject_user_id: owner,
                task_id: task,
                content_version: 1,
                response: serde_json::json!({"answer": "inspect watermarks"}),
                artifacts: vec![],
                evaluation_method: TaskEvaluationMethod::SelfReview,
            })
            .await
            .expect("start submission");
        let submitted = repository
            .submit(owner, submission.id)
            .await
            .expect("submit task");
        assert_eq!(submitted.envelope.status, TaskSubmissionStatus::Reviewed);
        assert_eq!(submitted.envelope.review_status, TaskReviewStatus::Complete);
    }

    #[tokio::test]
    async fn agent_review_records_score_and_feedback() {
        let repository = InMemoryTaskSubmissionRepository::default();
        let owner = Uuid::now_v7();
        let task = Uuid::now_v7();
        repository.register_task(task, owner, Uuid::now_v7(), 1);
        let submission = repository
            .start(StartTaskSubmission {
                subject_user_id: owner,
                task_id: task,
                content_version: 1,
                response: serde_json::json!({"artifact": "watermark analysis"}),
                artifacts: vec![],
                evaluation_method: TaskEvaluationMethod::Agent,
            })
            .await
            .expect("start submission");
        let submitted = repository
            .submit(owner, submission.id)
            .await
            .expect("submit task");
        let reviewed = repository
            .review(ReviewTaskSubmission {
                reviewer_user_id: owner,
                submission_id: submitted.id,
                outcome: TaskReviewOutcome::Reviewed,
                score: Some(0.9),
                feedback: Some(serde_json::json!({"note": "Good diagnosis"})),
                rubric_scores: None,
                review_provenance: Some(serde_json::json!({"model": "reviewer-v1"})),
            })
            .await
            .expect("review task");
        assert_eq!(reviewed.envelope.status, TaskSubmissionStatus::Reviewed);
        assert_eq!(reviewed.envelope.score, Some(0.9));
    }

    #[tokio::test]
    async fn rejected_project_creates_an_idempotent_artifact_revision() {
        let repository = InMemoryTaskSubmissionRepository::default();
        let owner = Uuid::now_v7();
        let task = Uuid::now_v7();
        repository.register_task(task, owner, Uuid::now_v7(), 3);
        let first = repository
            .start(StartTaskSubmission {
                subject_user_id: owner,
                task_id: task,
                content_version: 3,
                response: serde_json::json!({"report": "first"}),
                artifacts: vec![],
                evaluation_method: TaskEvaluationMethod::Agent,
            })
            .await
            .unwrap();
        let submitted = repository.submit(owner, first.id).await.unwrap();
        repository
            .review(ReviewTaskSubmission {
                reviewer_user_id: Uuid::now_v7(),
                submission_id: submitted.id,
                outcome: TaskReviewOutcome::Rejected,
                score: Some(0.4),
                feedback: Some(serde_json::json!({"note": "add evidence"})),
                rubric_scores: None,
                review_provenance: Some(serde_json::json!({"method": "manual"})),
            })
            .await
            .unwrap();
        let artifact = crate::domain::task::SubmissionArtifact {
            kind: "report".into(),
            name: "revision.md".into(),
            media_type: "text/markdown".into(),
            content: "Revised evidence".into(),
        };
        let revised = repository
            .revise(
                owner,
                first.id,
                serde_json::json!({"report": "revised"}),
                vec![artifact.clone()],
            )
            .await
            .unwrap();
        let retry = repository
            .revise(
                owner,
                first.id,
                serde_json::json!({"report": "revised"}),
                vec![artifact],
            )
            .await
            .unwrap();
        assert_eq!(revised, retry);
        assert_eq!(revised.revision, 2);
        assert_eq!(revised.parent_submission_id, Some(first.id));
    }
}
