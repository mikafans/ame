//! Assessment repository contract and aggregate grading.

use crate::{
    domain::{
        assessment::{
            Assessment, AssessmentRepositoryError, CreateAssessment, validate_assessment,
        },
        question::QuestionVersion,
    },
    question::{AnswerEvaluationStatus, GradedAnswer, grade_answer},
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait]
pub trait AssessmentRepository: Send + Sync {
    async fn create_assessment(
        &self,
        input: CreateAssessment,
        questions: &[QuestionVersion],
    ) -> Result<Assessment, AssessmentRepositoryError>;

    async fn get_assessment(
        &self,
        subject_user_id: Uuid,
        assessment_id: Uuid,
    ) -> Result<Assessment, AssessmentRepositoryError>;

    async fn get_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Option<Assessment>, AssessmentRepositoryError>;
}

#[derive(Clone, Default)]
pub struct InMemoryAssessmentRepository {
    assessments: Arc<Mutex<HashMap<Uuid, Assessment>>>,
}

#[async_trait]
impl AssessmentRepository for InMemoryAssessmentRepository {
    async fn create_assessment(
        &self,
        input: CreateAssessment,
        questions: &[QuestionVersion],
    ) -> Result<Assessment, AssessmentRepositoryError> {
        validate_assessment(&input)?;
        for item in &input.items {
            let question = questions
                .iter()
                .find(|question| question.id == item.question_version_id)
                .ok_or(AssessmentRepositoryError::Question(
                    crate::domain::question::QuestionRepositoryError::NotFound {
                        resource: "question version",
                    },
                ))?;
            if question.review_status != crate::domain::question::ContentReviewStatus::Approved {
                return Err(AssessmentRepositoryError::InvalidItem(
                    "only approved question versions can be published".to_string(),
                ));
            }
        }
        let mut assessments = self
            .assessments
            .lock()
            .map_err(|error| AssessmentRepositoryError::Storage(error.to_string()))?;
        if assessments
            .values()
            .any(|assessment| assessment.activity_id == input.activity_id)
        {
            return Err(AssessmentRepositoryError::ActivityAlreadyHasAssessment);
        }
        let assessment = Assessment {
            id: Uuid::now_v7(),
            subject_user_id: input.subject_user_id,
            source_actor_id: input.source_actor_id,
            activity_id: input.activity_id,
            version: 1,
            mode: input.mode,
            items: input.items,
            status: input.status,
            created_at: OffsetDateTime::now_utc(),
        };
        assessments.insert(assessment.id, assessment.clone());
        Ok(assessment)
    }

    async fn get_assessment(
        &self,
        subject_user_id: Uuid,
        assessment_id: Uuid,
    ) -> Result<Assessment, AssessmentRepositoryError> {
        let assessments = self
            .assessments
            .lock()
            .map_err(|error| AssessmentRepositoryError::Storage(error.to_string()))?;
        let assessment = assessments
            .get(&assessment_id)
            .ok_or(AssessmentRepositoryError::NotFound)?;
        if assessment.subject_user_id != subject_user_id {
            return Err(AssessmentRepositoryError::SubjectMismatch);
        }
        Ok(assessment.clone())
    }

    async fn get_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Option<Assessment>, AssessmentRepositoryError> {
        let assessments = self
            .assessments
            .lock()
            .map_err(|error| AssessmentRepositoryError::Storage(error.to_string()))?;
        Ok(assessments
            .values()
            .find(|assessment| {
                assessment.activity_id == activity_id
                    && assessment.subject_user_id == subject_user_id
                    && assessment.status == crate::domain::assessment::AssessmentStatus::Published
            })
            .cloned())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssessmentGrade {
    pub item_grades: Vec<GradedAnswer>,
    pub awarded_points: f32,
    pub max_points: f32,
    pub pending_manual_review: bool,
    pub score: Option<f32>,
}

pub fn grade_assessment(
    assessment: &Assessment,
    questions: &[QuestionVersion],
    responses: &HashMap<Uuid, serde_json::Value>,
) -> Result<AssessmentGrade, AssessmentRepositoryError> {
    let mut item_grades = Vec::with_capacity(assessment.items.len());
    let mut awarded_points = 0.0;
    let mut max_points = 0.0;
    let mut pending_manual_review = false;
    for item in &assessment.items {
        let question = questions
            .iter()
            .find(|question| question.id == item.question_version_id)
            .ok_or(AssessmentRepositoryError::Question(
                crate::domain::question::QuestionRepositoryError::NotFound {
                    resource: "question version",
                },
            ))?;
        let response = responses
            .get(&item.question_version_id)
            .cloned()
            .unwrap_or_default();
        let mut grade = grade_answer(question, &response);
        grade.awarded_points = grade.awarded_points.min(item.points as f32);
        if grade.status == AnswerEvaluationStatus::ManualReview {
            pending_manual_review = true;
        }
        awarded_points += grade.awarded_points;
        max_points += item.points as f32;
        item_grades.push(grade);
    }
    let score = (!pending_manual_review).then_some(awarded_points / max_points);
    Ok(AssessmentGrade {
        item_grades,
        awarded_points,
        max_points,
        pending_manual_review,
        score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        assessment::{AssessmentItemInput, AssessmentMode, AssessmentStatus, CreateAssessment},
        question::{ContentReviewStatus, QuestionKind, QuestionOption},
    };

    fn question(_subject: Uuid) -> QuestionVersion {
        QuestionVersion {
            id: Uuid::now_v7(),
            question_id: Uuid::now_v7(),
            generation_run_id: Uuid::now_v7(),
            version: 1,
            kind: QuestionKind::MultipleChoice,
            difficulty: None,
            prompt: "Choose".into(),
            options: vec![
                QuestionOption {
                    id: "a".into(),
                    text: "A".into(),
                    is_correct: true,
                },
                QuestionOption {
                    id: "b".into(),
                    text: "B".into(),
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
        }
    }

    #[tokio::test]
    async fn approved_questions_compose_and_grade_as_practice() {
        let subject = Uuid::now_v7();
        let version = question(subject);
        let repository = InMemoryAssessmentRepository::default();
        let assessment = repository
            .create_assessment(
                CreateAssessment {
                    subject_user_id: subject,
                    source_actor_id: subject,
                    activity_id: Uuid::now_v7(),
                    mode: AssessmentMode::Practice,
                    items: vec![AssessmentItemInput {
                        id: Uuid::now_v7(),
                        objective_id: Uuid::now_v7(),
                        question_version_id: version.id,
                        order_index: 0,
                        points: 1,
                    }],
                    status: AssessmentStatus::Published,
                },
                std::slice::from_ref(&version),
            )
            .await
            .expect("assessment creates");
        let mut responses = HashMap::new();
        responses.insert(version.id, serde_json::json!({"option_id": "a"}));
        let grade =
            grade_assessment(&assessment, &[version], &responses).expect("assessment grades");
        assert_eq!(grade.score, Some(1.0));
        assert!(!grade.pending_manual_review);
    }

    #[tokio::test]
    async fn unapproved_questions_and_cross_subject_reads_are_rejected() {
        let subject = Uuid::now_v7();
        let mut version = question(subject);
        version.review_status = ContentReviewStatus::Review;
        let result = InMemoryAssessmentRepository::default()
            .create_assessment(
                CreateAssessment {
                    subject_user_id: subject,
                    source_actor_id: subject,
                    activity_id: Uuid::now_v7(),
                    mode: AssessmentMode::Graded,
                    items: vec![AssessmentItemInput {
                        id: Uuid::now_v7(),
                        objective_id: Uuid::now_v7(),
                        question_version_id: version.id,
                        order_index: 0,
                        points: 1,
                    }],
                    status: AssessmentStatus::Published,
                },
                &[version],
            )
            .await;
        assert!(matches!(
            result,
            Err(AssessmentRepositoryError::InvalidItem(_))
        ));

        let repository = InMemoryAssessmentRepository::default();
        let version = question(subject);
        let assessment = repository
            .create_assessment(
                CreateAssessment {
                    subject_user_id: subject,
                    source_actor_id: subject,
                    activity_id: Uuid::now_v7(),
                    mode: AssessmentMode::Practice,
                    items: vec![AssessmentItemInput {
                        id: Uuid::now_v7(),
                        objective_id: Uuid::now_v7(),
                        question_version_id: version.id,
                        order_index: 0,
                        points: 1,
                    }],
                    status: AssessmentStatus::Published,
                },
                &[version],
            )
            .await
            .expect("assessment creates");
        assert_eq!(
            repository
                .get_assessment(Uuid::now_v7(), assessment.id)
                .await,
            Err(AssessmentRepositoryError::SubjectMismatch)
        );
    }

    #[test]
    fn manual_review_does_not_fabricate_a_score() {
        let mut question = question(Uuid::now_v7());
        question.kind = QuestionKind::Essay;
        question.options.clear();
        let assessment = Assessment {
            id: Uuid::now_v7(),
            subject_user_id: Uuid::now_v7(),
            source_actor_id: Uuid::now_v7(),
            activity_id: Uuid::now_v7(),
            version: 1,
            mode: AssessmentMode::Graded,
            items: vec![AssessmentItemInput {
                id: Uuid::now_v7(),
                objective_id: Uuid::now_v7(),
                question_version_id: question.id,
                order_index: 0,
                points: 2,
            }],
            status: AssessmentStatus::Published,
            created_at: OffsetDateTime::now_utc(),
        };
        let grade = grade_assessment(&assessment, &[question.clone()], &HashMap::new())
            .expect("essay grades");
        assert!(grade.pending_manual_review);
        assert_eq!(grade.score, None);
    }
}
