//! PostgreSQL implementation of the versioned question contract.

use crate::domain::generation::{GenerationRepository, validate_content_run};
use crate::domain::question::{
    ContentReviewStatus, CreateQuestion, Question, QuestionKind, QuestionRepositoryError,
    QuestionVersion, validate_question,
};
use crate::question::QuestionRepository;
use async_trait::async_trait;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgQuestionRepository {
    pool: PgPool,
}

impl PgQuestionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_version_by_id(
        &self,
        subject_user_id: Uuid,
        version_id: Uuid,
    ) -> Result<QuestionVersion, QuestionRepositoryError> {
        let row = sqlx::query(
            r#"SELECT q.subject_user_id, qv.generation_run_id, qv.id, qv.question_id, qv.version,
                      qv.kind, qv.prompt, qv.payload, qv.explanation, qv.rationale,
                      qv.points, qv.difficulty, qv.review_status, qv.source_references, qv.created_at
               FROM tb_questions q
               JOIN tb_question_versions qv ON qv.question_id = q.id
               WHERE qv.id = $1"#,
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(QuestionRepositoryError::NotFound {
            resource: "question version",
        })?;
        if row.get::<Uuid, _>("subject_user_id") != subject_user_id {
            return Err(QuestionRepositoryError::SubjectMismatch);
        }
        question_version_from_row(&row)
    }

    async fn ensure_generation_run(
        &self,
        subject_user_id: Uuid,
        generation_run_id: Uuid,
    ) -> Result<(), QuestionRepositoryError> {
        let run = crate::generation_postgres::PgGenerationRepository::new(self.pool.clone())
            .get(subject_user_id, generation_run_id)
            .await
            .map_err(QuestionRepositoryError::Generation)?;
        validate_content_run(&run, subject_user_id, "question.compose")
            .map_err(QuestionRepositoryError::Generation)
    }
}

#[async_trait]
impl QuestionRepository for PgQuestionRepository {
    async fn create_question(
        &self,
        input: CreateQuestion,
    ) -> Result<(Question, QuestionVersion), QuestionRepositoryError> {
        validate_question(&input)?;
        self.ensure_generation_run(input.subject_user_id, input.generation_run_id)
            .await?;
        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        let question_row = sqlx::query(
            r#"INSERT INTO tb_questions (
                    subject_user_id, source_actor_id, generation_run_id,
                    kind, status, current_version
                )
                VALUES ($1, $2, $3, $4, $5, 1)
                RETURNING id, subject_user_id, source_actor_id, kind, status,
                          current_version, generation_run_id, created_at"#,
        )
        .bind(input.subject_user_id)
        .bind(input.source_actor_id)
        .bind(input.generation_run_id)
        .bind(kind_value(input.kind))
        .bind(question_status_value(input.review_status))
        .fetch_one(&mut *transaction)
        .await
        .map_err(storage_error)?;
        let question = question_from_row(&question_row)?;
        let version = insert_version(&mut transaction, &question, 1, &input).await?;
        transaction.commit().await.map_err(storage_error)?;
        Ok((question, version))
    }

    async fn create_version(
        &self,
        subject_user_id: Uuid,
        question_id: Uuid,
        input: CreateQuestion,
    ) -> Result<QuestionVersion, QuestionRepositoryError> {
        validate_question(&input)?;
        self.ensure_generation_run(subject_user_id, input.generation_run_id)
            .await?;
        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        let row = sqlx::query(
            "SELECT id, subject_user_id, source_actor_id, kind, status, current_version, generation_run_id, created_at
             FROM tb_questions WHERE id = $1 FOR UPDATE",
        )
        .bind(question_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage_error)?
        .ok_or(QuestionRepositoryError::NotFound {
            resource: "question",
        })?;
        let question = question_from_row(&row)?;
        if question.subject_user_id != subject_user_id {
            return Err(QuestionRepositoryError::SubjectMismatch);
        }
        if question.kind != input.kind {
            return Err(QuestionRepositoryError::InvalidQuestion(
                "a question version cannot change kind".to_string(),
            ));
        }
        let version = insert_version(
            &mut transaction,
            &question,
            question.current_version + 1,
            &input,
        )
        .await?;
        sqlx::query("UPDATE tb_questions SET current_version = $2, status = $3, generation_run_id = $4, updated_at = now() WHERE id = $1")
            .bind(question_id)
            .bind(version.version as i32)
            .bind(question_status_value(input.review_status))
            .bind(input.generation_run_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage_error)?;
        transaction.commit().await.map_err(storage_error)?;
        Ok(version)
    }

    async fn get_version(
        &self,
        subject_user_id: Uuid,
        question_id: Uuid,
        version: u32,
    ) -> Result<QuestionVersion, QuestionRepositoryError> {
        let row = sqlx::query(
            r#"SELECT q.subject_user_id, qv.generation_run_id, qv.id, qv.question_id, qv.version,
                      qv.kind, qv.prompt, qv.payload, qv.explanation, qv.rationale,
                      qv.points, qv.difficulty, qv.review_status, qv.source_references, qv.created_at
               FROM tb_questions q
               JOIN tb_question_versions qv ON qv.question_id = q.id
               WHERE q.id = $1 AND qv.version = $2"#,
        )
        .bind(question_id)
        .bind(version as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(QuestionRepositoryError::NotFound {
            resource: "question version",
        })?;
        if row.get::<Uuid, _>("subject_user_id") != subject_user_id {
            return Err(QuestionRepositoryError::SubjectMismatch);
        }
        question_version_from_row(&row)
    }
}

async fn insert_version(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    question: &Question,
    version: u32,
    input: &CreateQuestion,
) -> Result<QuestionVersion, QuestionRepositoryError> {
    let source_references = json!(input.source_references);
    let payload = json!({
        "options": input.options,
        "accepted_answers": input.accepted_answers,
        "difficulty": input.difficulty
    });
    let row = sqlx::query(
        r#"INSERT INTO tb_question_versions (
                question_id, version, generation_run_id, kind, prompt, payload, explanation,
                rationale, points, difficulty, source_references, review_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, question_id, version, generation_run_id, kind, prompt, payload,
                      explanation, rationale, points, difficulty, review_status,
                      source_references, created_at"#,
    )
    .bind(question.id)
    .bind(version as i32)
    .bind(input.generation_run_id)
    .bind(kind_value(input.kind))
    .bind(&input.prompt)
    .bind(payload)
    .bind(&input.explanation)
    .bind(&input.rationale)
    .bind(input.points as i32)
    .bind(&input.difficulty)
    .bind(source_references)
    .bind(version_review_status(input.review_status))
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage_error)?;
    question_version_from_row(&row)
}

fn question_from_row(row: &sqlx::postgres::PgRow) -> Result<Question, QuestionRepositoryError> {
    Ok(Question {
        id: row.get("id"),
        subject_user_id: row.get("subject_user_id"),
        source_actor_id: row.get("source_actor_id"),
        generation_run_id: row.get("generation_run_id"),
        kind: parse_kind(row.get("kind"))?,
        current_version: row.get::<i32, _>("current_version") as u32,
        status: parse_question_status(row.get("status"))?,
        created_at: row.get("created_at"),
    })
}

fn question_version_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<QuestionVersion, QuestionRepositoryError> {
    let payload: serde_json::Value = row.get("payload");
    let options = serde_json::from_value(payload.get("options").cloned().unwrap_or_default())
        .map_err(|error| QuestionRepositoryError::Storage(error.to_string()))?;
    let accepted_answers =
        serde_json::from_value(payload.get("accepted_answers").cloned().unwrap_or_default())
            .map_err(|error| QuestionRepositoryError::Storage(error.to_string()))?;
    Ok(QuestionVersion {
        id: row.get("id"),
        question_id: row.get("question_id"),
        generation_run_id: row.get("generation_run_id"),
        version: row.get::<i32, _>("version") as u32,
        kind: parse_kind(row.get("kind"))?,
        prompt: row.get("prompt"),
        options,
        accepted_answers,
        explanation: row.get("explanation"),
        rationale: row.get("rationale"),
        points: row.get::<i32, _>("points") as u32,
        difficulty: row.get("difficulty"),
        review_status: parse_version_review_status(row.get("review_status"))?,
        source_references: serde_json::from_value(row.get("source_references"))
            .map_err(|error| QuestionRepositoryError::Storage(error.to_string()))?,
        created_at: row.get("created_at"),
    })
}

fn kind_value(kind: QuestionKind) -> &'static str {
    match kind {
        QuestionKind::MultipleChoice => "multiple_choice",
        QuestionKind::TrueFalse => "true_false",
        QuestionKind::ShortAnswer => "short_answer",
        QuestionKind::Essay => "essay",
        QuestionKind::Code => "code",
    }
}

fn parse_kind(value: &str) -> Result<QuestionKind, QuestionRepositoryError> {
    match value {
        "multiple_choice" => Ok(QuestionKind::MultipleChoice),
        "true_false" => Ok(QuestionKind::TrueFalse),
        "short_answer" => Ok(QuestionKind::ShortAnswer),
        "essay" => Ok(QuestionKind::Essay),
        "code" => Ok(QuestionKind::Code),
        _ => Err(QuestionRepositoryError::Storage(format!(
            "unknown question kind {value}"
        ))),
    }
}

fn question_status_value(status: ContentReviewStatus) -> &'static str {
    match status {
        ContentReviewStatus::Draft => "draft",
        ContentReviewStatus::Review => "review",
        ContentReviewStatus::Approved => "approved",
        ContentReviewStatus::Rejected | ContentReviewStatus::Retired => "retired",
    }
}

fn version_review_status(status: ContentReviewStatus) -> &'static str {
    match status {
        ContentReviewStatus::Draft => "draft",
        ContentReviewStatus::Review => "review",
        ContentReviewStatus::Approved => "approved",
        ContentReviewStatus::Rejected | ContentReviewStatus::Retired => "rejected",
    }
}

fn parse_question_status(value: &str) -> Result<ContentReviewStatus, QuestionRepositoryError> {
    match value {
        "draft" => Ok(ContentReviewStatus::Draft),
        "review" => Ok(ContentReviewStatus::Review),
        "approved" => Ok(ContentReviewStatus::Approved),
        "retired" => Ok(ContentReviewStatus::Retired),
        _ => Err(QuestionRepositoryError::Storage(format!(
            "unknown question status {value}"
        ))),
    }
}

fn parse_version_review_status(
    value: &str,
) -> Result<ContentReviewStatus, QuestionRepositoryError> {
    match value {
        "draft" => Ok(ContentReviewStatus::Draft),
        "review" => Ok(ContentReviewStatus::Review),
        "approved" => Ok(ContentReviewStatus::Approved),
        "rejected" => Ok(ContentReviewStatus::Rejected),
        _ => Err(QuestionRepositoryError::Storage(format!(
            "unknown question review status {value}"
        ))),
    }
}

fn storage_error(error: impl std::fmt::Display) -> QuestionRepositoryError {
    QuestionRepositoryError::Storage(error.to_string())
}
