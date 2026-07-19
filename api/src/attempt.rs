//! Resumable attempt repository and grading contract.

use crate::{
    assessment::grade_assessment,
    domain::{
        assessment::Assessment,
        attempt::{Attempt, AttemptAnswer, AttemptError, AttemptStatus, StartAttempt},
        question::QuestionVersion,
    },
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait]
pub trait AttemptRepository: Send + Sync {
    async fn start(
        &self,
        input: StartAttempt,
        assessment: &Assessment,
    ) -> Result<Attempt, AttemptError>;
    async fn get(&self, subject_user_id: Uuid, attempt_id: Uuid) -> Result<Attempt, AttemptError>;
    async fn list_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Vec<Attempt>, AttemptError>;
    async fn save_answer(
        &self,
        subject_user_id: Uuid,
        attempt_id: Uuid,
        item_id: Uuid,
        question_version_id: Uuid,
        response: serde_json::Value,
    ) -> Result<Attempt, AttemptError>;
    async fn finish(
        &self,
        subject_user_id: Uuid,
        attempt_id: Uuid,
        assessment: &Assessment,
        questions: &[QuestionVersion],
    ) -> Result<Attempt, AttemptError>;
}

#[derive(Clone, Default)]
pub struct InMemoryAttemptRepository {
    attempts: Arc<Mutex<HashMap<Uuid, Attempt>>>,
}

#[async_trait]
impl AttemptRepository for InMemoryAttemptRepository {
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
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|error| AttemptError::Storage(error.to_string()))?;
        if let Some(existing) = attempts
            .values()
            .find(|attempt| attempt.input == input && attempt.status == AttemptStatus::InProgress)
        {
            return Ok(existing.clone());
        }
        let attempt = Attempt {
            id: Uuid::now_v7(),
            input,
            status: AttemptStatus::InProgress,
            responses: HashMap::new(),
            item_question_versions: assessment
                .items
                .iter()
                .map(|item| (item.id, item.question_version_id))
                .collect(),
            answer_results: Vec::new(),
            stored_score: None,
            stored_max_points: None,
            grade: None,
            created_at: OffsetDateTime::now_utc(),
            submitted_at: None,
        };
        attempts.insert(attempt.id, attempt.clone());
        Ok(attempt)
    }

    async fn get(&self, subject_user_id: Uuid, attempt_id: Uuid) -> Result<Attempt, AttemptError> {
        let attempts = self
            .attempts
            .lock()
            .map_err(|error| AttemptError::Storage(error.to_string()))?;
        let attempt = attempts.get(&attempt_id).ok_or(AttemptError::NotFound)?;
        if attempt.input.subject_user_id != subject_user_id {
            return Err(AttemptError::SubjectMismatch);
        }
        Ok(attempt.clone())
    }

    async fn list_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Vec<Attempt>, AttemptError> {
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|error| AttemptError::Storage(error.to_string()))?
            .values()
            .filter(|attempt| {
                attempt.input.subject_user_id == subject_user_id
                    && attempt.input.activity_id == activity_id
            })
            .cloned()
            .collect::<Vec<_>>();
        attempts.sort_by_key(|attempt| attempt.created_at);
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
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|error| AttemptError::Storage(error.to_string()))?;
        let attempt = attempts
            .get_mut(&attempt_id)
            .ok_or(AttemptError::NotFound)?;
        if attempt.input.subject_user_id != subject_user_id {
            return Err(AttemptError::SubjectMismatch);
        }
        if attempt.status != AttemptStatus::InProgress {
            return Err(AttemptError::AlreadyFinished);
        }
        let expected_question_version = attempt
            .item_question_versions
            .get(&item_id)
            .ok_or(AttemptError::ItemNotInAssessment)?;
        if *expected_question_version != question_version_id {
            return Err(AttemptError::StaleQuestionVersion);
        }
        attempt
            .responses
            .insert(*expected_question_version, response);
        let answer_response = attempt
            .responses
            .get(expected_question_version)
            .cloned()
            .unwrap_or_default();
        if let Some(answer) = attempt
            .answer_results
            .iter_mut()
            .find(|answer| answer.assessment_item_id == item_id)
        {
            answer.response = answer_response;
            answer.evaluation_status = "pending".into();
            answer.correctness = None;
            answer.awarded_points = None;
        } else {
            attempt.answer_results.push(AttemptAnswer {
                assessment_item_id: item_id,
                question_version_id,
                response: answer_response,
                correctness: None,
                awarded_points: None,
                evaluation_status: "pending".into(),
            });
        }
        Ok(attempt.clone())
    }

    async fn finish(
        &self,
        subject_user_id: Uuid,
        attempt_id: Uuid,
        assessment: &Assessment,
        questions: &[QuestionVersion],
    ) -> Result<Attempt, AttemptError> {
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|error| AttemptError::Storage(error.to_string()))?;
        let attempt = attempts
            .get_mut(&attempt_id)
            .ok_or(AttemptError::NotFound)?;
        if attempt.input.subject_user_id != subject_user_id {
            return Err(AttemptError::SubjectMismatch);
        }
        if attempt.input.assessment_id != assessment.id
            || attempt.input.assessment_version != assessment.version
        {
            return Err(AttemptError::StaleQuestionVersion);
        }
        if attempt.status != AttemptStatus::InProgress {
            return attempt
                .grade
                .clone()
                .map(|_| attempt.clone())
                .ok_or(AttemptError::AlreadyFinished);
        }
        let grade = grade_assessment(assessment, questions, &attempt.responses)
            .map_err(|error| AttemptError::Storage(error.to_string()))?;
        attempt.answer_results = assessment
            .items
            .iter()
            .zip(&grade.item_grades)
            .map(|(item, item_grade)| AttemptAnswer {
                assessment_item_id: item.id,
                question_version_id: item.question_version_id,
                response: attempt
                    .responses
                    .get(&item.question_version_id)
                    .cloned()
                    .unwrap_or_default(),
                correctness: item_grade.correctness,
                awarded_points: Some(item_grade.awarded_points),
                evaluation_status: match item_grade.status {
                    crate::question::AnswerEvaluationStatus::Correct => "correct",
                    crate::question::AnswerEvaluationStatus::Incorrect => "incorrect",
                    crate::question::AnswerEvaluationStatus::Partial => "partial",
                    crate::question::AnswerEvaluationStatus::ManualReview => "manual_review",
                }
                .into(),
            })
            .collect();
        attempt.status = if grade.pending_manual_review {
            AttemptStatus::Submitted
        } else {
            AttemptStatus::Graded
        };
        attempt.grade = Some(grade);
        attempt.submitted_at = Some(OffsetDateTime::now_utc());
        Ok(attempt.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        assessment::{AssessmentItemInput, AssessmentMode, AssessmentStatus},
        question::{ContentReviewStatus, QuestionKind, QuestionOption},
    };

    fn assessment(question_id: Uuid) -> (Assessment, QuestionVersion) {
        let question = QuestionVersion {
            id: question_id,
            question_id: Uuid::now_v7(),
            generation_run_id: Uuid::now_v7(),
            version: 1,
            kind: QuestionKind::MultipleChoice,
            difficulty: None,
            prompt: "Choose".into(),
            options: vec![
                QuestionOption {
                    id: "right".into(),
                    text: "Right".into(),
                    is_correct: true,
                },
                QuestionOption {
                    id: "wrong".into(),
                    text: "Wrong".into(),
                    is_correct: false,
                },
            ],
            accepted_answers: vec![],
            explanation: None,
            rationale: None,
            points: 1,
            review_status: ContentReviewStatus::Approved,
            source_references: vec![],
            created_at: OffsetDateTime::now_utc(),
        };
        let assessment = Assessment {
            id: Uuid::now_v7(),
            subject_user_id: Uuid::now_v7(),
            source_actor_id: Uuid::now_v7(),
            activity_id: Uuid::now_v7(),
            version: 1,
            mode: AssessmentMode::Practice,
            items: vec![AssessmentItemInput {
                id: Uuid::now_v7(),
                objective_id: Uuid::now_v7(),
                question_version_id: question_id,
                order_index: 0,
                points: 1,
            }],
            status: AssessmentStatus::Published,
            created_at: OffsetDateTime::now_utc(),
        };
        (assessment, question)
    }

    #[tokio::test]
    async fn attempt_resumes_and_finishes_idempotently() {
        let repository = InMemoryAttemptRepository::default();
        let question_id = Uuid::now_v7();
        let (mut assessment, question) = assessment(question_id);
        let subject = assessment.subject_user_id;
        let input = StartAttempt {
            subject_user_id: subject,
            learning_session_id: Uuid::now_v7(),
            activity_id: assessment.activity_id,
            assessment_id: assessment.id,
            assessment_version: 1,
        };
        let first = repository
            .start(input.clone(), &assessment)
            .await
            .expect("attempt starts");
        assert_eq!(
            repository
                .start(input, &assessment)
                .await
                .expect("retry resumes")
                .id,
            first.id
        );
        let item_id = assessment.items[0].id;
        let saved = repository
            .save_answer(
                subject,
                first.id,
                item_id,
                question_id,
                serde_json::json!({"option_id": "right"}),
            )
            .await
            .expect("answer saves");
        let finished = repository
            .finish(subject, saved.id, &assessment, &[question])
            .await
            .expect("attempt finishes");
        assert_eq!(
            finished.grade.as_ref().and_then(|grade| grade.score),
            Some(1.0)
        );
        assert_eq!(
            repository
                .finish(subject, finished.id, &assessment, &[])
                .await
                .expect("finish retry")
                .id,
            finished.id
        );
        assessment.version = 2;
        assert_eq!(
            repository
                .finish(subject, finished.id, &assessment, &[])
                .await,
            Err(AttemptError::StaleQuestionVersion)
        );
    }

    #[tokio::test]
    async fn cross_subject_and_stale_answers_are_rejected() {
        let repository = InMemoryAttemptRepository::default();
        let question_id = Uuid::now_v7();
        let (assessment, _) = assessment(question_id);
        let attempt = repository
            .start(
                StartAttempt {
                    subject_user_id: assessment.subject_user_id,
                    learning_session_id: Uuid::now_v7(),
                    activity_id: assessment.activity_id,
                    assessment_id: assessment.id,
                    assessment_version: 1,
                },
                &assessment,
            )
            .await
            .expect("attempt starts");
        assert_eq!(
            repository.get(Uuid::now_v7(), attempt.id).await,
            Err(AttemptError::SubjectMismatch)
        );
        assert_eq!(
            repository
                .save_answer(
                    assessment.subject_user_id,
                    attempt.id,
                    Uuid::now_v7(),
                    question_id,
                    serde_json::json!({})
                )
                .await,
            Err(AttemptError::ItemNotInAssessment)
        );
        assert_eq!(
            repository
                .save_answer(
                    assessment.subject_user_id,
                    attempt.id,
                    assessment.items[0].id,
                    Uuid::now_v7(),
                    serde_json::json!({})
                )
                .await,
            Err(AttemptError::StaleQuestionVersion)
        );
    }
}
