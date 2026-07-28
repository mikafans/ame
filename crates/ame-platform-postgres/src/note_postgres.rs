//! PostgreSQL learner-note adapter.

use crate::{
    domain::note::{CreateNote, MAX_NOTE_BYTES, NoteError, validate_note},
    note::{Note, NoteRepository},
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgNoteRepository {
    pool: PgPool,
}

impl PgNoteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NoteRepository for PgNoteRepository {
    async fn create(&self, input: CreateNote) -> Result<Note, NoteError> {
        validate_note(&input)?;
        if let Some(row) = sqlx::query(
            "SELECT * FROM tb_learner_notes
             WHERE subject_user_id = $1 AND retry_key = $2",
        )
        .bind(input.subject_user_id)
        .bind(&input.retry_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        {
            let note = map_note(&row);
            if note.journey_id != input.journey_id
                || note.activity_id != input.activity_id
                || note.content_version != input.content_version
                || note.body != input.body
            {
                return Err(NoteError::RetryConflict);
            }
            return Ok(note);
        }
        let anchor_valid = match input.activity_id {
            Some(activity_id) => sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (
                        SELECT 1 FROM tb_activities a
                        JOIN tb_learning_journeys j ON j.id = a.journey_id
                        WHERE a.id = $1 AND a.journey_id = $2
                          AND j.subject_user_id = $3
                          AND a.content_version >= $4
                    )",
            )
            .bind(activity_id)
            .bind(input.journey_id)
            .bind(input.subject_user_id)
            .bind(input.content_version)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?,
            None => sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (
                        SELECT 1 FROM tb_learning_journeys
                        WHERE id = $1 AND subject_user_id = $2
                    )",
            )
            .bind(input.journey_id)
            .bind(input.subject_user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?,
        };
        if !anchor_valid {
            return Err(NoteError::InvalidAnchor);
        }
        let row = sqlx::query(
            "INSERT INTO tb_learner_notes
                (id, subject_user_id, journey_id, activity_id, content_version,
                 body, retry_key)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(Uuid::now_v7())
        .bind(input.subject_user_id)
        .bind(input.journey_id)
        .bind(input.activity_id)
        .bind(input.content_version)
        .bind(input.body)
        .bind(input.retry_key)
        .fetch_one(&self.pool)
        .await
        .map_err(storage)?;
        Ok(map_note(&row))
    }

    async fn list(
        &self,
        subject_user_id: Uuid,
        journey_id: Uuid,
        activity_id: Option<Uuid>,
    ) -> Result<Vec<Note>, NoteError> {
        sqlx::query(
            "SELECT * FROM tb_learner_notes
             WHERE subject_user_id = $1 AND journey_id = $2
               AND ($3::uuid IS NULL OR activity_id = $3)
             ORDER BY updated_at DESC",
        )
        .bind(subject_user_id)
        .bind(journey_id)
        .bind(activity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)
        .map(|rows| rows.iter().map(map_note).collect())
    }

    async fn update(
        &self,
        subject_user_id: Uuid,
        id: Uuid,
        expected_revision: i32,
        body: String,
    ) -> Result<Note, NoteError> {
        if body.trim().is_empty() {
            return Err(NoteError::EmptyField { field: "body" });
        }
        if body.len() > MAX_NOTE_BYTES {
            return Err(NoteError::TooLarge);
        }
        sqlx::query(
            "UPDATE tb_learner_notes
             SET body = $1, revision = revision + 1, updated_at = now()
             WHERE id = $2 AND subject_user_id = $3 AND revision = $4
             RETURNING *",
        )
        .bind(body)
        .bind(id)
        .bind(subject_user_id)
        .bind(expected_revision)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .map(|row| map_note(&row))
        .ok_or(NoteError::NotFound)
    }

    async fn delete(&self, subject_user_id: Uuid, id: Uuid) -> Result<(), NoteError> {
        let result =
            sqlx::query("DELETE FROM tb_learner_notes WHERE id = $1 AND subject_user_id = $2")
                .bind(id)
                .bind(subject_user_id)
                .execute(&self.pool)
                .await
                .map_err(storage)?;
        if result.rows_affected() == 0 {
            return Err(NoteError::NotFound);
        }
        Ok(())
    }
}

fn map_note(row: &sqlx::postgres::PgRow) -> Note {
    Note {
        id: row.get("id"),
        subject_user_id: row.get("subject_user_id"),
        journey_id: row.get("journey_id"),
        activity_id: row.get("activity_id"),
        content_version: row.get("content_version"),
        body: row.get("body"),
        revision: row.get("revision"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn storage(error: impl std::fmt::Display) -> NoteError {
    NoteError::Storage(error.to_string())
}
