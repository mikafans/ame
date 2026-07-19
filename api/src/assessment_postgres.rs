//! PostgreSQL implementation of the assessment repository contract.

use crate::{
    assessment::AssessmentRepository,
    domain::{
        assessment::{
            Assessment, AssessmentItemInput, AssessmentMode, AssessmentRepositoryError,
            AssessmentStatus, CreateAssessment,
        },
        question::{ContentReviewStatus, QuestionRepositoryError, QuestionVersion},
    },
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgAssessmentRepository {
    pool: PgPool,
}

impl PgAssessmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssessmentRepository for PgAssessmentRepository {
    async fn create_assessment(
        &self,
        input: CreateAssessment,
        questions: &[QuestionVersion],
    ) -> Result<Assessment, AssessmentRepositoryError> {
        crate::domain::assessment::validate_assessment(&input)?;
        for item in &input.items {
            let question = questions
                .iter()
                .find(|question| question.id == item.question_version_id)
                .ok_or(AssessmentRepositoryError::Question(
                    QuestionRepositoryError::NotFound {
                        resource: "question version",
                    },
                ))?;
            if question.review_status != ContentReviewStatus::Approved {
                return Err(AssessmentRepositoryError::InvalidItem(
                    "only approved question versions can be published".to_string(),
                ));
            }
        }
        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        let activity = sqlx::query("SELECT subject_user_id FROM tb_activities WHERE id = $1")
            .bind(input.activity_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage_error)?
            .ok_or(AssessmentRepositoryError::NotFound)?;
        if activity.get::<Uuid, _>("subject_user_id") != input.subject_user_id {
            return Err(AssessmentRepositoryError::SubjectMismatch);
        }
        let row = sqlx::query(
            r#"INSERT INTO tb_assessments (
                    activity_id, subject_user_id, source_actor_id, version,
                    mode, status
                )
                VALUES ($1, $2, $3, 1, $4, $5)
                RETURNING id, subject_user_id, source_actor_id, activity_id,
                          version, mode, status, created_at"#,
        )
        .bind(input.activity_id)
        .bind(input.subject_user_id)
        .bind(input.source_actor_id)
        .bind(mode_value(input.mode))
        .bind(status_value(input.status))
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| {
            if let sqlx::Error::Database(database) = &error
                && database.constraint() == Some("tb_assessments_activity_id_key")
            {
                return AssessmentRepositoryError::ActivityAlreadyHasAssessment;
            }
            storage_error(error)
        })?;
        let assessment = assessment_from_row(&row, input.items.clone())?;
        for item in &input.items {
            sqlx::query(
                "INSERT INTO tb_assessment_items (id, assessment_id, question_version_id, order_index, points_override) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(item.id)
            .bind(assessment.id)
            .bind(item.question_version_id)
            .bind(item.order_index)
            .bind(item.points as i32)
            .execute(&mut *transaction)
            .await
            .map_err(storage_error)?;
        }
        transaction.commit().await.map_err(storage_error)?;
        Ok(assessment)
    }

    async fn get_assessment(
        &self,
        subject_user_id: Uuid,
        assessment_id: Uuid,
    ) -> Result<Assessment, AssessmentRepositoryError> {
        let row = sqlx::query(
            "SELECT id, subject_user_id, source_actor_id, activity_id, version, mode, status, created_at FROM tb_assessments WHERE id = $1",
        )
        .bind(assessment_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(AssessmentRepositoryError::NotFound)?;
        if row.get::<Uuid, _>("subject_user_id") != subject_user_id {
            return Err(AssessmentRepositoryError::SubjectMismatch);
        }
        let items = sqlx::query(
            "SELECT id, question_version_id, order_index, points_override FROM tb_assessment_items WHERE assessment_id = $1 ORDER BY order_index",
        )
        .bind(assessment_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(|item| AssessmentItemInput {
            id: item.get("id"),
            question_version_id: item.get("question_version_id"),
            order_index: item.get("order_index"),
            points: item.get::<i32, _>("points_override") as u32,
        })
        .collect();
        assessment_from_row(&row, items)
    }

    async fn get_for_activity(
        &self,
        subject_user_id: Uuid,
        activity_id: Uuid,
    ) -> Result<Option<Assessment>, AssessmentRepositoryError> {
        let row = sqlx::query(
            "SELECT id, subject_user_id, source_actor_id, activity_id, version, mode, status, created_at FROM tb_assessments WHERE activity_id = $1 AND subject_user_id = $2 AND status = 'published'",
        )
        .bind(activity_id)
        .bind(subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let assessment_id: Uuid = row.get("id");
        let items = sqlx::query(
            "SELECT id, question_version_id, order_index, points_override FROM tb_assessment_items WHERE assessment_id = $1 ORDER BY order_index",
        )
        .bind(assessment_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage_error)?
        .into_iter()
        .map(|item| AssessmentItemInput {
            id: item.get("id"),
            question_version_id: item.get("question_version_id"),
            order_index: item.get("order_index"),
            points: item.get::<i32, _>("points_override") as u32,
        })
        .collect();
        Ok(Some(assessment_from_row(&row, items)?))
    }
}

fn assessment_from_row(
    row: &sqlx::postgres::PgRow,
    items: Vec<AssessmentItemInput>,
) -> Result<Assessment, AssessmentRepositoryError> {
    Ok(Assessment {
        id: row.get("id"),
        subject_user_id: row.get("subject_user_id"),
        source_actor_id: row.get("source_actor_id"),
        activity_id: row.get("activity_id"),
        version: row.get::<i32, _>("version") as u32,
        mode: parse_mode(row.get("mode"))?,
        items,
        status: parse_status(row.get("status"))?,
        created_at: row.get("created_at"),
    })
}

fn mode_value(mode: AssessmentMode) -> &'static str {
    match mode {
        AssessmentMode::Practice => "practice",
        AssessmentMode::Graded => "graded",
    }
}

fn status_value(status: AssessmentStatus) -> &'static str {
    match status {
        AssessmentStatus::Draft => "draft",
        AssessmentStatus::Published => "published",
        AssessmentStatus::Retired => "retired",
    }
}

fn parse_mode(value: &str) -> Result<AssessmentMode, AssessmentRepositoryError> {
    match value {
        "practice" => Ok(AssessmentMode::Practice),
        "graded" => Ok(AssessmentMode::Graded),
        _ => Err(AssessmentRepositoryError::Storage(format!(
            "unknown assessment mode {value}"
        ))),
    }
}

fn parse_status(value: &str) -> Result<AssessmentStatus, AssessmentRepositoryError> {
    match value {
        "draft" => Ok(AssessmentStatus::Draft),
        "published" => Ok(AssessmentStatus::Published),
        "retired" => Ok(AssessmentStatus::Retired),
        _ => Err(AssessmentRepositoryError::Storage(format!(
            "unknown assessment status {value}"
        ))),
    }
}

fn storage_error(error: impl std::fmt::Display) -> AssessmentRepositoryError {
    AssessmentRepositoryError::Storage(error.to_string())
}
