use ame_platform_application::task::{
    ReviewTaskSubmission, StartTaskSubmission, TaskReviewOutcome, TaskSubmission,
    TaskSubmissionError, TaskSubmissionRepository,
};
use ame_platform_domain::task::{
    TaskEvaluationMethod, TaskReviewStatus, TaskSubmissionEnvelope, TaskSubmissionStatus,
    validate_task_submission_transition,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgTaskSubmissionRepository {
    pool: PgPool,
}

impl PgTaskSubmissionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TaskSubmissionRepository for PgTaskSubmissionRepository {
    async fn start(
        &self,
        input: StartTaskSubmission,
    ) -> Result<TaskSubmission, TaskSubmissionError> {
        if input.response.is_null() {
            return Err(TaskSubmissionError::InvalidContract(
                "response must be present".into(),
            ));
        }
        let task = sqlx::query(
            "SELECT a.journey_id, a.subject_user_id, a.content_version
             FROM tb_activities a
             JOIN tb_learning_journeys j ON j.id = a.journey_id AND j.subject_user_id = a.subject_user_id
             WHERE a.id = $1 AND a.subject_user_id = $2",
        )
        .bind(input.task_id)
        .bind(input.subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(TaskSubmissionError::SubjectMismatch)?;
        if task.get::<i32, _>("content_version") != input.content_version as i32 {
            return Err(TaskSubmissionError::StaleContentVersion);
        }
        let row = sqlx::query(
            "INSERT INTO tb_task_submissions
                 (task_id, subject_user_id, journey_id, content_version, response, evaluation_method)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, journey_id, task_id, subject_user_id, content_version, response,
                       evaluation_method, status, review_status, score, feedback",
        )
        .bind(input.task_id)
        .bind(input.subject_user_id)
        .bind(task.get::<Uuid, _>("journey_id"))
        .bind(input.content_version as i32)
        .bind(input.response)
        .bind(evaluation_method_name(input.evaluation_method))
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        decode_submission(row)
    }

    async fn submit(
        &self,
        subject_user_id: Uuid,
        submission_id: Uuid,
    ) -> Result<TaskSubmission, TaskSubmissionError> {
        let current = sqlx::query(
            "SELECT id, journey_id, task_id, subject_user_id, content_version, response,
                    evaluation_method, status, review_status, score, feedback
             FROM tb_task_submissions WHERE id = $1",
        )
        .bind(submission_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(TaskSubmissionError::NotFound)?;
        if current.get::<Uuid, _>("subject_user_id") != subject_user_id {
            return Err(TaskSubmissionError::SubjectMismatch);
        }
        let status = task_submission_status(current.get("status"))?;
        validate_task_submission_transition(status, TaskSubmissionStatus::Submitted)
            .map_err(TaskSubmissionError::InvalidTransition)?;
        let method = task_evaluation_method(current.get("evaluation_method"))?;
        let (next_status, review_status) = if method == TaskEvaluationMethod::SelfReview {
            (TaskSubmissionStatus::Reviewed, TaskReviewStatus::Complete)
        } else {
            (TaskSubmissionStatus::Submitted, TaskReviewStatus::Pending)
        };
        let row = sqlx::query(
            "UPDATE tb_task_submissions
                SET status = $2, review_status = $3,
                    submitted_at = COALESCE(submitted_at, now()),
                    reviewed_at = CASE WHEN $2 = 'reviewed' THEN now() ELSE reviewed_at END,
                    updated_at = now()
              WHERE id = $1
              RETURNING id, journey_id, task_id, subject_user_id, content_version, response,
                        evaluation_method, status, review_status, score, feedback",
        )
        .bind(submission_id)
        .bind(task_submission_status_name(next_status))
        .bind(task_review_status_name(review_status))
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        decode_submission(row)
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
        let current =
            sqlx::query("SELECT status, subject_user_id FROM tb_task_submissions WHERE id = $1")
                .bind(input.submission_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage_error)?
                .ok_or(TaskSubmissionError::NotFound)?;
        let current_status = task_submission_status(current.get("status"))?;
        let next_status = match input.outcome {
            TaskReviewOutcome::Reviewed => TaskSubmissionStatus::Reviewed,
            TaskReviewOutcome::Rejected => TaskSubmissionStatus::Rejected,
        };
        validate_task_submission_transition(current_status, next_status)
            .map_err(TaskSubmissionError::InvalidTransition)?;
        let row = sqlx::query(
            "UPDATE tb_task_submissions
                SET status = $2, review_status = 'complete', score = $3,
                    feedback = $4, reviewed_at = now(), updated_at = now()
              WHERE id = $1
              RETURNING id, journey_id, task_id, subject_user_id, content_version, response,
                        evaluation_method, status, review_status, score, feedback",
        )
        .bind(input.submission_id)
        .bind(task_submission_status_name(next_status))
        .bind(input.score)
        .bind(input.feedback)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        decode_submission(row)
    }
}

fn decode_submission(row: sqlx::postgres::PgRow) -> Result<TaskSubmission, TaskSubmissionError> {
    Ok(TaskSubmission {
        id: row.get("id"),
        journey_id: row.get("journey_id"),
        envelope: TaskSubmissionEnvelope {
            task_id: row.get("task_id"),
            subject_user_id: row.get("subject_user_id"),
            content_version: row.get::<i32, _>("content_version") as u32,
            response: row.get("response"),
            evaluation_method: task_evaluation_method(row.get("evaluation_method"))?,
            status: task_submission_status(row.get("status"))?,
            review_status: task_review_status(row.get("review_status"))?,
            score: row.get("score"),
            feedback: row.get("feedback"),
        },
    })
}

fn evaluation_method_name(method: TaskEvaluationMethod) -> &'static str {
    match method {
        TaskEvaluationMethod::SelfReview => "self_review",
        TaskEvaluationMethod::Automatic => "automatic",
        TaskEvaluationMethod::Agent => "agent",
        TaskEvaluationMethod::Manual => "manual",
    }
}
fn task_evaluation_method(value: String) -> Result<TaskEvaluationMethod, TaskSubmissionError> {
    match value.as_str() {
        "self_review" => Ok(TaskEvaluationMethod::SelfReview),
        "automatic" => Ok(TaskEvaluationMethod::Automatic),
        "agent" => Ok(TaskEvaluationMethod::Agent),
        "manual" => Ok(TaskEvaluationMethod::Manual),
        _ => Err(TaskSubmissionError::Storage(
            "invalid evaluation method".into(),
        )),
    }
}
fn task_submission_status_name(status: TaskSubmissionStatus) -> &'static str {
    match status {
        TaskSubmissionStatus::InProgress => "in_progress",
        TaskSubmissionStatus::Submitted => "submitted",
        TaskSubmissionStatus::InReview => "in_review",
        TaskSubmissionStatus::Reviewed => "reviewed",
        TaskSubmissionStatus::Rejected => "rejected",
        TaskSubmissionStatus::Abandoned => "abandoned",
    }
}
fn task_submission_status(value: String) -> Result<TaskSubmissionStatus, TaskSubmissionError> {
    match value.as_str() {
        "in_progress" => Ok(TaskSubmissionStatus::InProgress),
        "submitted" => Ok(TaskSubmissionStatus::Submitted),
        "in_review" => Ok(TaskSubmissionStatus::InReview),
        "reviewed" => Ok(TaskSubmissionStatus::Reviewed),
        "rejected" => Ok(TaskSubmissionStatus::Rejected),
        "abandoned" => Ok(TaskSubmissionStatus::Abandoned),
        _ => Err(TaskSubmissionError::Storage(
            "invalid submission status".into(),
        )),
    }
}
fn task_review_status_name(status: TaskReviewStatus) -> &'static str {
    match status {
        TaskReviewStatus::NotRequired => "not_required",
        TaskReviewStatus::Pending => "pending",
        TaskReviewStatus::Complete => "complete",
    }
}
fn task_review_status(value: String) -> Result<TaskReviewStatus, TaskSubmissionError> {
    match value.as_str() {
        "not_required" => Ok(TaskReviewStatus::NotRequired),
        "pending" => Ok(TaskReviewStatus::Pending),
        "complete" => Ok(TaskReviewStatus::Complete),
        _ => Err(TaskSubmissionError::Storage("invalid review status".into())),
    }
}
fn storage_error(error: sqlx::Error) -> TaskSubmissionError {
    TaskSubmissionError::Storage(error.to_string())
}
