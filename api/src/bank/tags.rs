//! Tag repository.
//!
//! All persistence for the `tags` table goes through this module. Tag names
//! are stored lowercased — the DB enforces `name = lower(name)` via CHECK,
//! and we mirror that here so a caller that forgets to lowercase still hits
//! the unique index on the canonical form.
//!
//! [`get_or_create_tags_tx`] is the workhorse used during question ingestion.
//! Pool-level entry points exist for the standalone tag endpoints; everything
//! that runs alongside question writes goes through the tx variant so the
//! whole batch commits (or rolls back) atomically.

use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::domain::error::ApiError;
use crate::domain::question::Tag;

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

fn row_to_tag(row: sqlx::postgres::PgRow) -> Tag {
    Tag {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        created_at: row.get("created_at"),
    }
}

/// Return tags owned by the caller, ordered by name. Used by `GET /tags`.
pub async fn list_tags(conn: &mut sqlx::PgConnection) -> Result<Vec<Tag>, ApiError> {
    let rows = sqlx::query(
        "SELECT DISTINCT t.id, t.name, t.description, t.created_at \
         FROM tb_tags t \
         JOIN tb_question_tags qt ON qt.tag_id = t.id \
         JOIN tb_questions q ON q.id = qt.question_id \
         ORDER BY t.name ASC",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(internal)?;
    Ok(rows.into_iter().map(row_to_tag).collect())
}

/// Create a single tag, returning the persisted row. Idempotent on `name`:
/// if a tag with the lowercased name already exists, its current row is
/// returned and `description` is left as-is.
pub async fn create_tag(
    conn: &mut sqlx::PgConnection,
    name: &str,
    description: Option<&str>,
) -> Result<Tag, ApiError> {
    let lower = name.to_lowercase();

    sqlx::query(
        "INSERT INTO tb_tags (name, description) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING",
    )
    .bind(&lower)
    .bind(description)
    .execute(&mut *conn)
    .await
    .map_err(internal)?;

    let row = sqlx::query("SELECT id, name, description, created_at FROM tb_tags WHERE name = $1")
        .bind(&lower)
        .fetch_one(&mut *conn)
        .await
        .map_err(internal)?;

    Ok(row_to_tag(row))
}

/// Lowercase + dedupe input names, ensure rows exist, and return one [`Tag`]
/// per unique input in first-seen order. Safe to run concurrently — the
/// `ON CONFLICT DO NOTHING` makes parallel inserts of the same name race
/// harmlessly.
pub async fn get_or_create_tags_tx(
    tx: &mut Transaction<'_, Postgres>,
    names: &[String],
) -> Result<Vec<Tag>, ApiError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }

    let mut unique: Vec<String> = Vec::with_capacity(names.len());
    for raw in names {
        let lower = raw.to_lowercase();
        if !unique.contains(&lower) {
            unique.push(lower);
        }
    }

    sqlx::query(
        "INSERT INTO tb_tags (name) SELECT unnest($1::text[]) ON CONFLICT (name) DO NOTHING",
    )
    .bind(&unique)
    .execute(&mut **tx)
    .await
    .map_err(internal)?;

    let rows =
        sqlx::query("SELECT id, name, description, created_at FROM tb_tags WHERE name = ANY($1)")
            .bind(&unique)
            .fetch_all(&mut **tx)
            .await
            .map_err(internal)?;

    let fetched: Vec<Tag> = rows.into_iter().map(row_to_tag).collect();

    let mut ordered = Vec::with_capacity(unique.len());
    for name in &unique {
        if let Some(tag) = fetched.iter().find(|t| &t.name == name) {
            ordered.push(tag.clone());
        }
    }
    Ok(ordered)
}

/// Replace the tag set on a question with `tag_ids`. Runs inside the
/// caller's transaction so question writes and tag links commit (or roll
/// back) atomically.
pub async fn set_question_tags(
    tx: &mut Transaction<'_, Postgres>,
    question_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM tb_question_tags WHERE question_id = $1")
        .bind(question_id)
        .execute(&mut **tx)
        .await
        .map_err(internal)?;

    for tag_id in tag_ids {
        sqlx::query(
            "INSERT INTO tb_question_tags (question_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(question_id)
        .bind(tag_id)
        .execute(&mut **tx)
        .await
        .map_err(internal)?;
    }

    Ok(())
}
