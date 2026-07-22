//! PostgreSQL implementation of the generation-run lifecycle.

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Row, postgres::PgRow};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::generation::{
    GenerationError, GenerationRepository, GenerationRun, GenerationStatus, StartGenerationRun,
    transition_status, validate_start,
};

#[derive(Clone)]
pub struct PgGenerationRepository {
    pool: PgPool,
}

impl PgGenerationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GenerationRepository for PgGenerationRepository {
    async fn start(&self, input: StartGenerationRun) -> Result<GenerationRun, GenerationError> {
        validate_start(&input)?;
        if let Some(retry_key) = input.retry_key.as_deref()
            && let Some(row) = sqlx::query(
                "SELECT id, subject_user_id, source_actor_id, operation, provider, retry_key, content_version, status, error, created_at, updated_at FROM tb_generation_runs WHERE subject_user_id = $1 AND operation = $2 AND retry_key = $3",
            )
            .bind(input.subject_user_id)
            .bind(&input.operation)
            .bind(retry_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage_error)?
        {
            return generation_from_row(&row);
        }

        let row = sqlx::query(
            "INSERT INTO tb_generation_runs (subject_user_id, source_actor_id, operation, provider, retry_key, content_version, status) VALUES ($1, $2, $3, $4, $5, $6, 'requested') ON CONFLICT DO NOTHING RETURNING id, subject_user_id, source_actor_id, operation, provider, retry_key, content_version, status, error, created_at, updated_at",
        )
        .bind(input.subject_user_id)
        .bind(input.source_actor_id)
        .bind(&input.operation)
        .bind(&input.provider)
        .bind(&input.retry_key)
        .bind(input.content_version as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?;

        if let Some(row) = row {
            return generation_from_row(&row);
        }
        let retry_key = input.retry_key.ok_or_else(|| {
            GenerationError::Storage("generation run insert was not returned".into())
        })?;
        sqlx::query(
            "SELECT id, subject_user_id, source_actor_id, operation, provider, retry_key, content_version, status, error, created_at, updated_at FROM tb_generation_runs WHERE subject_user_id = $1 AND operation = $2 AND retry_key = $3",
        )
        .bind(input.subject_user_id)
        .bind(input.operation)
        .bind(retry_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| GenerationError::Storage("generation run conflict was not readable".into()))
        .and_then(|row| generation_from_row(&row))
    }

    async fn transition(
        &self,
        subject_user_id: Uuid,
        id: Uuid,
        status: GenerationStatus,
        error: Option<Value>,
    ) -> Result<GenerationRun, GenerationError> {
        let mut transaction = self.pool.begin().await.map_err(storage_error)?;
        let row = sqlx::query(
            "SELECT id, subject_user_id, source_actor_id, operation, provider, retry_key, content_version, status, error, created_at, updated_at FROM tb_generation_runs WHERE id = $1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage_error)?
        .ok_or(GenerationError::NotFound)?;
        let current = generation_from_row(&row)?;
        if current.subject_user_id != subject_user_id {
            return Err(GenerationError::SubjectMismatch);
        }
        transition_status(current.status, status)?;
        if status == GenerationStatus::Failed && error.is_none() {
            return Err(GenerationError::EmptyField { field: "error" });
        }
        if status != GenerationStatus::Failed && error.is_some() {
            return Err(GenerationError::InvalidTransition {
                from: current.status,
                to: status,
            });
        }
        let updated = sqlx::query(
            "UPDATE tb_generation_runs SET status = $2, error = $3, updated_at = now() WHERE id = $1 RETURNING id, subject_user_id, source_actor_id, operation, provider, retry_key, content_version, status, error, created_at, updated_at",
        )
        .bind(id)
        .bind(status_value(status))
        .bind(error)
        .fetch_one(&mut *transaction)
        .await
        .map_err(storage_error)?;
        transaction.commit().await.map_err(storage_error)?;
        generation_from_row(&updated)
    }

    async fn get(&self, subject_user_id: Uuid, id: Uuid) -> Result<GenerationRun, GenerationError> {
        let row = sqlx::query(
            "SELECT id, subject_user_id, source_actor_id, operation, provider, retry_key, content_version, status, error, created_at, updated_at FROM tb_generation_runs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage_error)?
        .ok_or(GenerationError::NotFound)?;
        let run = generation_from_row(&row)?;
        if run.subject_user_id != subject_user_id {
            return Err(GenerationError::SubjectMismatch);
        }
        Ok(run)
    }
}

fn status_value(status: GenerationStatus) -> &'static str {
    match status {
        GenerationStatus::Requested => "requested",
        GenerationStatus::Running => "running",
        GenerationStatus::ReviewRequired => "review_required",
        GenerationStatus::Published => "published",
        GenerationStatus::Failed => "failed",
    }
}

fn parse_status(value: &str) -> Result<GenerationStatus, GenerationError> {
    match value {
        "requested" => Ok(GenerationStatus::Requested),
        "running" => Ok(GenerationStatus::Running),
        "review_required" => Ok(GenerationStatus::ReviewRequired),
        "published" => Ok(GenerationStatus::Published),
        "failed" => Ok(GenerationStatus::Failed),
        other => Err(GenerationError::Storage(format!(
            "unknown generation status {other}"
        ))),
    }
}

fn generation_from_row(row: &PgRow) -> Result<GenerationRun, GenerationError> {
    Ok(GenerationRun {
        id: row.get("id"),
        subject_user_id: row.get("subject_user_id"),
        source_actor_id: row.get("source_actor_id"),
        operation: row.get("operation"),
        provider: row.get("provider"),
        retry_key: row.get("retry_key"),
        content_version: row.get::<i32, _>("content_version") as u32,
        status: parse_status(row.get("status"))?,
        error: row.get("error"),
        created_at: row.get::<OffsetDateTime, _>("created_at"),
        updated_at: row.get::<OffsetDateTime, _>("updated_at"),
    })
}

fn storage_error(error: impl std::fmt::Display) -> GenerationError {
    GenerationError::Storage(error.to_string())
}
