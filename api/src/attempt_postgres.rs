//! PostgreSQL implementation of resumable assessment attempts.

use crate::{
    assessment::grade_assessment,
    attempt::AttemptRepository,
    domain::{
        assessment::Assessment,
        attempt::{Attempt, AttemptAnswer, AttemptError, AttemptStatus, StartAttempt},
        question::QuestionVersion,
    },
    question::AnswerEvaluationStatus,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgAttemptRepository {
    pool: PgPool,
}

impl PgAttemptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AttemptRepository for PgAttemptRepository {
    async fn start(
        &self,
        input: StartAttempt,
        assessment: &Assessment,
    ) -> Result<Attempt, AttemptError> {
        if input.subject_user_id != assessment.subject_user_id
            || input.activity_id != assessment.activity_id
            || input.assessment_id != assessment.id
            || input.assessment_version != assessment.version
        {
            return Err(AttemptError::StaleQuestionVersion);
        }
        let existing = sqlx::query(
            "SELECT id, status, created_at, submitted_at FROM tb_attempts WHERE subject_user_id = $1 AND assessment_id = $2 AND assessment_version = $3 AND status = 'in_progress' ORDER BY created_at DESC LIMIT 1",
        )
        .bind(input.subject_user_id)
        .bind(input.assessment_id)
        .bind(input.assessment_version as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;
        if let Some(row) = existing {
            return self.load_attempt(row.get("id"), input).await;
        }
        let id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO tb_attempts (learning_session_id, activity_id, subject_user_id, assessment_id, assessment_version) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(input.learning_session_id)
        .bind(input.activity_id)
        .bind(input.subject_user_id)
        .bind(input.assessment_id)
        .bind(input.assessment_version as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(storage_error)?;
        self.load_attempt(id, input).await
    }

    async fn get(&self, subject_user_id: Uuid, attempt_id: Uuid) -> Result<Attempt, AttemptError> {
        let row = sqlx::query("SELECT id, learning_session_id, activity_id, subject_user_id, assessment_id, assessment_version, status, created_at, submitted_at FROM tb_attempts WHERE id = $1")
            .bind(attempt_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage_error)?
            .ok_or(AttemptError::NotFound)?;
        if row.get::<Uuid, _>("subject_user_id") != subject_user_id {
            return Err(AttemptError::SubjectMismatch);
        }
        let input = StartAttempt {
            subject_user_id,
            learning_session_id: row.get("learning_session_id"),
            activity_id: row.get("activity_id"),
            assessment_id: row.get("assessment_id"),
            assessment_version: row.get::<i32, _>("assessment_version") as u32,
        };
        self.load_attempt(attempt_id, input).await
    }

    async fn list_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Vec<Attempt>, AttemptError> {
        let rows = sqlx::query(
            "SELECT id, learning_session_id, activity_id, subject_user_id, assessment_id, assessment_version FROM tb_attempts WHERE subject_user_id = $1 AND activity_id = $2 ORDER BY created_at",
        )
        .bind(subject_user_id)
        .bind(activity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?;
        let mut attempts = Vec::with_capacity(rows.len());
        for row in rows {
            let input = StartAttempt {
                subject_user_id: row.get("subject_user_id"),
                learning_session_id: row.get("learning_session_id"),
                activity_id: row.get("activity_id"),
                assessment_id: row.get("assessment_id"),
                assessment_version: row.get::<i32, _>("assessment_version") as u32,
            };
            attempts.push(self.load_attempt(row.get("id"), input).await?);
        }
        Ok(attempts)
    }

    async fn save_answer(
        &self,
        subject_user_id: Uuid,
        attempt_id: Uuid,
        item_id: Uuid,
        question_version_id: Uuid,
        response: serde_json::Value,
    ) -> Result<Attempt, AttemptError> {
        let attempt = self.get(subject_user_id, attempt_id).await?;
        if attempt.status != AttemptStatus::InProgress {
            return Err(AttemptError::AlreadyFinished);
        }
        if attempt.item_question_versions.get(&item_id) != Some(&question_version_id) {
            return if attempt.item_question_versions.contains_key(&item_id) {
                Err(AttemptError::StaleQuestionVersion)
            } else {
                Err(AttemptError::ItemNotInAssessment)
            };
        }
        sqlx::query(
            "INSERT INTO tb_attempt_answers (attempt_id, assessment_item_id, question_version_id, response) VALUES ($1, $2, $3, $4) ON CONFLICT (attempt_id, assessment_item_id) DO UPDATE SET question_version_id = EXCLUDED.question_version_id, response = EXCLUDED.response, updated_at = now()",
        )
        .bind(attempt_id)
        .bind(item_id)
        .bind(question_version_id)
        .bind(response)
        .execute(&self.pool)
        .await
        .map_err(storage_error)?;
        self.get(subject_user_id, attempt_id).await
    }

    async fn finish(
        &self,
        subject_user_id: Uuid,
        attempt_id: Uuid,
        assessment: &Assessment,
        questions: &[QuestionVersion],
    ) -> Result<Attempt, AttemptError> {
        let attempt = self.get(subject_user_id, attempt_id).await?;
        if attempt.input.assessment_id != assessment.id
            || attempt.input.assessment_version != assessment.version
        {
            return Err(AttemptError::StaleQuestionVersion);
        }
        if attempt.status != AttemptStatus::InProgress {
            return if attempt.grade.is_some() {
                Ok(attempt)
            } else {
                Err(AttemptError::AlreadyFinished)
            };
        }
        let grade = grade_assessment(assessment, questions, &attempt.responses)
            .map_err(|error| AttemptError::Storage(error.to_string()))?;
        let status = if grade.pending_manual_review {
            "submitted"
        } else {
            "graded"
        };
        let review_status = if grade.pending_manual_review {
            "pending"
        } else {
            "complete"
        };
        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        sqlx::query("UPDATE tb_attempts SET status = $1, score = $2, max_score = $3, review_status = $4, submitted_at = now(), graded_at = CASE WHEN $1 = 'graded' THEN now() ELSE NULL END, updated_at = now() WHERE id = $5 AND status = 'in_progress'")
            .bind(status).bind(grade.score).bind(grade.max_points).bind(review_status).bind(attempt_id)
            .execute(&mut *transaction).await.map_err(storage_error)?;
        for (item, item_grade) in assessment.items.iter().zip(&grade.item_grades) {
            sqlx::query("UPDATE tb_attempt_answers SET correctness = $1, awarded_points = $2, evaluation_status = $3, updated_at = now() WHERE attempt_id = $4 AND assessment_item_id = $5")
                .bind(item_grade.correctness).bind(item_grade.awarded_points).bind(evaluation_status(item_grade.status)).bind(attempt_id).bind(item.id)
                .execute(&mut *transaction).await.map_err(storage_error)?;
        }
        transaction.commit().await.map_err(storage_error)?;
        let mut finished = self.get(subject_user_id, attempt_id).await?;
        finished.status = if grade.pending_manual_review {
            AttemptStatus::Submitted
        } else {
            AttemptStatus::Graded
        };
        finished.grade = Some(grade);
        finished.submitted_at = Some(OffsetDateTime::now_utc());
        Ok(finished)
    }
}

impl PgAttemptRepository {
    async fn load_attempt(
        &self,
        attempt_id: Uuid,
        input: StartAttempt,
    ) -> Result<Attempt, AttemptError> {
        let row =
            sqlx::query("SELECT status, created_at, submitted_at, score::float4 AS score, max_score::float4 AS max_score FROM tb_attempts WHERE id = $1")
                .bind(attempt_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage_error)?
                .ok_or(AttemptError::NotFound)?;
        let items = sqlx::query("SELECT id, question_version_id FROM tb_assessment_items WHERE assessment_id = $1 ORDER BY order_index")
            .bind(input.assessment_id).fetch_all(&self.pool).await.map_err(storage_error)?;
        let answers = sqlx::query("SELECT assessment_item_id, question_version_id, response, correctness::float4 AS correctness, awarded_points::float4 AS awarded_points, evaluation_status FROM tb_attempt_answers WHERE attempt_id = $1")
            .bind(attempt_id).fetch_all(&self.pool).await.map_err(storage_error)?;
        let mut responses = HashMap::new();
        let mut item_question_versions = HashMap::new();
        let mut answer_results = Vec::new();
        for item in items {
            item_question_versions.insert(item.get("id"), item.get("question_version_id"));
        }
        for answer in answers {
            let item_id: Uuid = answer.get("assessment_item_id");
            let version_id: Uuid = answer.get("question_version_id");
            item_question_versions.insert(item_id, version_id);
            let response: serde_json::Value = answer.get("response");
            responses.insert(version_id, response.clone());
            answer_results.push(AttemptAnswer {
                assessment_item_id: item_id,
                question_version_id: version_id,
                response,
                correctness: answer.get("correctness"),
                awarded_points: answer.get("awarded_points"),
                evaluation_status: answer.get("evaluation_status"),
            });
        }
        Ok(Attempt {
            id: attempt_id,
            input,
            status: parse_status(row.get("status"))?,
            responses,
            item_question_versions,
            answer_results,
            stored_score: row.get("score"),
            stored_max_points: row.get("max_score"),
            grade: None,
            created_at: row.get("created_at"),
            submitted_at: row.get("submitted_at"),
        })
    }
}

fn parse_status(value: &str) -> Result<AttemptStatus, AttemptError> {
    match value {
        "in_progress" => Ok(AttemptStatus::InProgress),
        "submitted" => Ok(AttemptStatus::Submitted),
        "graded" => Ok(AttemptStatus::Graded),
        "abandoned" => Ok(AttemptStatus::Abandoned),
        other => Err(AttemptError::Storage(format!(
            "unknown attempt status {other}"
        ))),
    }
}

fn evaluation_status(status: AnswerEvaluationStatus) -> &'static str {
    match status {
        AnswerEvaluationStatus::Correct => "correct",
        AnswerEvaluationStatus::Incorrect => "incorrect",
        AnswerEvaluationStatus::Partial => "partial",
        AnswerEvaluationStatus::ManualReview => "manual_review",
    }
}

fn storage_error(error: impl std::fmt::Display) -> AttemptError {
    AttemptError::Storage(error.to_string())
}
