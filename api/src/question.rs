//! Question repository contract and deterministic grading primitives.

use crate::domain::generation::{GenerationError, GenerationRun, validate_content_run};
use crate::domain::question::{
    CreateQuestion, Question, QuestionKind, QuestionRepositoryError, QuestionVersion,
    validate_question,
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait]
pub trait QuestionRepository: Send + Sync {
    async fn create_question(
        &self,
        input: CreateQuestion,
    ) -> Result<(Question, QuestionVersion), QuestionRepositoryError>;

    async fn create_version(
        &self,
        subject_user_id: Uuid,
        question_id: Uuid,
        input: CreateQuestion,
    ) -> Result<QuestionVersion, QuestionRepositoryError>;

    async fn get_version(
        &self,
        subject_user_id: Uuid,
        question_id: Uuid,
        version: u32,
    ) -> Result<QuestionVersion, QuestionRepositoryError>;
}

#[derive(Clone, Default)]
pub struct InMemoryQuestionRepository {
    state: Arc<Mutex<QuestionState>>,
}

#[derive(Default)]
struct QuestionState {
    questions: HashMap<Uuid, Question>,
    versions: HashMap<(Uuid, u32), QuestionVersion>,
    generation_runs: HashMap<Uuid, GenerationRun>,
}

#[async_trait]
impl QuestionRepository for InMemoryQuestionRepository {
    async fn create_question(
        &self,
        input: CreateQuestion,
    ) -> Result<(Question, QuestionVersion), QuestionRepositoryError> {
        validate_question(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(|error| QuestionRepositoryError::Storage(error.to_string()))?;
        validate_generation_run(&state, input.subject_user_id, input.generation_run_id)?;
        let now = OffsetDateTime::now_utc();
        let question = Question {
            id: Uuid::now_v7(),
            subject_user_id: input.subject_user_id,
            source_actor_id: input.source_actor_id,
            generation_run_id: input.generation_run_id,
            kind: input.kind,
            current_version: 1,
            status: input.review_status,
            created_at: now,
        };
        let version = question_version(&question, 1, &input, now);
        state.questions.insert(question.id, question.clone());
        state.versions.insert((question.id, 1), version.clone());
        Ok((question, version))
    }

    async fn create_version(
        &self,
        subject_user_id: Uuid,
        question_id: Uuid,
        input: CreateQuestion,
    ) -> Result<QuestionVersion, QuestionRepositoryError> {
        validate_question(&input)?;
        let mut state = self
            .state
            .lock()
            .map_err(|error| QuestionRepositoryError::Storage(error.to_string()))?;
        validate_generation_run(&state, input.subject_user_id, input.generation_run_id)?;
        let question =
            state
                .questions
                .get_mut(&question_id)
                .ok_or(QuestionRepositoryError::NotFound {
                    resource: "question",
                })?;
        if question.subject_user_id != subject_user_id {
            return Err(QuestionRepositoryError::SubjectMismatch);
        }
        if question.kind != input.kind {
            return Err(QuestionRepositoryError::InvalidQuestion(
                "a question version cannot change kind".to_string(),
            ));
        }
        let version_number = question.current_version + 1;
        let now = OffsetDateTime::now_utc();
        let version = question_version(question, version_number, &input, now);
        question.current_version = version_number;
        question.generation_run_id = input.generation_run_id;
        question.status = input.review_status;
        state
            .versions
            .insert((question_id, version_number), version.clone());
        Ok(version)
    }

    async fn get_version(
        &self,
        subject_user_id: Uuid,
        question_id: Uuid,
        version: u32,
    ) -> Result<QuestionVersion, QuestionRepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|error| QuestionRepositoryError::Storage(error.to_string()))?;
        let question =
            state
                .questions
                .get(&question_id)
                .ok_or(QuestionRepositoryError::NotFound {
                    resource: "question",
                })?;
        if question.subject_user_id != subject_user_id {
            return Err(QuestionRepositoryError::SubjectMismatch);
        }
        state.versions.get(&(question_id, version)).cloned().ok_or(
            QuestionRepositoryError::NotFound {
                resource: "question version",
            },
        )
    }
}

impl InMemoryQuestionRepository {
    pub fn register_generation_run(
        &self,
        run: GenerationRun,
    ) -> Result<(), QuestionRepositoryError> {
        self.state
            .lock()
            .map_err(|error| QuestionRepositoryError::Storage(error.to_string()))?
            .generation_runs
            .insert(run.id, run);
        Ok(())
    }
}

fn validate_generation_run(
    state: &QuestionState,
    subject_user_id: Uuid,
    generation_run_id: Uuid,
) -> Result<(), QuestionRepositoryError> {
    let run = state.generation_runs.get(&generation_run_id).ok_or(
        QuestionRepositoryError::Generation(GenerationError::NotFound),
    )?;
    validate_content_run(run, subject_user_id, "question.compose")
        .map_err(QuestionRepositoryError::Generation)
}

fn question_version(
    question: &Question,
    version: u32,
    input: &CreateQuestion,
    created_at: OffsetDateTime,
) -> QuestionVersion {
    QuestionVersion {
        id: Uuid::now_v7(),
        question_id: question.id,
        generation_run_id: input.generation_run_id,
        version,
        kind: question.kind,
        prompt: input.prompt.clone(),
        options: input.options.clone(),
        accepted_answers: input.accepted_answers.clone(),
        explanation: input.explanation.clone(),
        rationale: input.rationale.clone(),
        difficulty: input.difficulty.clone(),
        points: input.points,
        review_status: input.review_status,
        source_references: input.source_references.clone(),
        created_at,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradedAnswer {
    pub correctness: Option<f32>,
    pub awarded_points: f32,
    pub status: AnswerEvaluationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerEvaluationStatus {
    Correct,
    Incorrect,
    Partial,
    ManualReview,
}

pub fn grade_answer(question: &QuestionVersion, response: &serde_json::Value) -> GradedAnswer {
    match question.kind {
        QuestionKind::MultipleChoice => {
            let selected = response
                .get("option_id")
                .and_then(serde_json::Value::as_str);
            let correct = selected.is_some_and(|selected| {
                question
                    .options
                    .iter()
                    .any(|option| option.id == selected && option.is_correct)
            });
            GradedAnswer {
                correctness: Some(if correct { 1.0 } else { 0.0 }),
                awarded_points: if correct { question.points as f32 } else { 0.0 },
                status: if correct {
                    AnswerEvaluationStatus::Correct
                } else {
                    AnswerEvaluationStatus::Incorrect
                },
            }
        }
        QuestionKind::TrueFalse => {
            let selected = response.get("value").and_then(serde_json::Value::as_bool);
            let correct = selected.is_some_and(|value| {
                question
                    .options
                    .iter()
                    .any(|option| option.is_correct && option.id == value.to_string())
            });
            GradedAnswer {
                correctness: Some(if correct { 1.0 } else { 0.0 }),
                awarded_points: if correct { question.points as f32 } else { 0.0 },
                status: if correct {
                    AnswerEvaluationStatus::Correct
                } else {
                    AnswerEvaluationStatus::Incorrect
                },
            }
        }
        QuestionKind::ShortAnswer => {
            let answer = response
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let normalized = answer.trim().to_lowercase();
            let correct = question
                .accepted_answers
                .iter()
                .any(|accepted| accepted.trim().to_lowercase() == normalized);
            GradedAnswer {
                correctness: Some(if correct { 1.0 } else { 0.0 }),
                awarded_points: if correct { question.points as f32 } else { 0.0 },
                status: if correct {
                    AnswerEvaluationStatus::Correct
                } else {
                    AnswerEvaluationStatus::Incorrect
                },
            }
        }
        QuestionKind::Essay | QuestionKind::Code => GradedAnswer {
            correctness: None,
            awarded_points: 0.0,
            status: AnswerEvaluationStatus::ManualReview,
        },
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::domain::{
        generation::{GenerationRun, GenerationStatus},
        question::{ContentReviewStatus, QuestionOption},
    };

    fn multiple_choice() -> CreateQuestion {
        CreateQuestion {
            subject_user_id: Uuid::now_v7(),
            source_actor_id: Uuid::now_v7(),
            generation_run_id: Uuid::now_v7(),
            kind: QuestionKind::MultipleChoice,
            prompt: "Which option is correct?".into(),
            options: vec![
                QuestionOption {
                    id: "wrong".into(),
                    text: "Wrong".into(),
                    is_correct: false,
                },
                QuestionOption {
                    id: "right".into(),
                    text: "Right".into(),
                    is_correct: true,
                },
            ],
            accepted_answers: vec![],
            explanation: Some("Because it is correct".into()),
            rationale: Some("Tests the objective".into()),
            difficulty: Some("introductory".into()),
            points: 2,
            review_status: ContentReviewStatus::Approved,
            source_references: vec!["https://example.test/source".into()],
        }
    }

    #[tokio::test]
    async fn versioned_question_keeps_stable_option_identity_and_history() {
        let repository = InMemoryQuestionRepository::default();
        let input = multiple_choice();
        let subject = input.subject_user_id;
        repository
            .register_generation_run(published_run(
                input.generation_run_id,
                subject,
                "question.compose",
            ))
            .expect("generation run registers");
        let (question, first) = repository
            .create_question(input.clone())
            .await
            .expect("question creates");
        assert_eq!(first.version, 1);
        let second = repository
            .create_version(subject, question.id, input)
            .await
            .expect("version creates");
        assert_eq!(second.version, 2);
        assert_eq!(
            repository
                .get_version(subject, question.id, 1)
                .await
                .expect("old version reads")
                .version,
            1
        );
    }

    fn published_run(id: Uuid, subject_user_id: Uuid, operation: &str) -> GenerationRun {
        let now = OffsetDateTime::now_utc();
        GenerationRun {
            id,
            subject_user_id,
            source_actor_id: subject_user_id,
            operation: operation.into(),
            provider: Some("test-provider".into()),
            retry_key: None,
            content_version: 1,
            status: GenerationStatus::Published,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn shuffled_multiple_choice_is_graded_by_option_id_not_position() {
        let input = multiple_choice();
        let version = QuestionVersion {
            id: Uuid::now_v7(),
            question_id: Uuid::now_v7(),
            generation_run_id: Uuid::now_v7(),
            version: 1,
            kind: input.kind,
            prompt: input.prompt,
            options: input.options,
            accepted_answers: input.accepted_answers,
            explanation: input.explanation,
            rationale: input.rationale,
            difficulty: input.difficulty,
            points: input.points,
            review_status: input.review_status,
            source_references: input.source_references,
            created_at: OffsetDateTime::now_utc(),
        };
        let result = grade_answer(&version, &serde_json::json!({"option_id": "right"}));
        assert_eq!(result.correctness, Some(1.0));
        assert_eq!(result.awarded_points, 2.0);
        assert_eq!(result.status, AnswerEvaluationStatus::Correct);
    }

    #[test]
    fn invalid_question_and_manual_review_are_explicit() {
        let mut input = multiple_choice();
        input.options[1].is_correct = false;
        assert!(matches!(
            validate_question(&input),
            Err(QuestionRepositoryError::InvalidQuestion(_))
        ));
        input.options[1].is_correct = true;
        input.kind = QuestionKind::Essay;
        input.options.clear();
        let version = QuestionVersion {
            id: Uuid::now_v7(),
            question_id: Uuid::now_v7(),
            generation_run_id: Uuid::now_v7(),
            version: 1,
            kind: input.kind,
            prompt: input.prompt,
            options: input.options,
            accepted_answers: input.accepted_answers,
            explanation: input.explanation,
            rationale: input.rationale,
            difficulty: input.difficulty,
            points: input.points,
            review_status: input.review_status,
            source_references: input.source_references,
            created_at: OffsetDateTime::now_utc(),
        };
        assert_eq!(
            grade_answer(&version, &serde_json::json!({"value": "answer"})).status,
            AnswerEvaluationStatus::ManualReview
        );
    }
}
