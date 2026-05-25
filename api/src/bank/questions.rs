//! Question repository: CRUD + versioning.
//!
//! ## Versioning rule (spec §"Versioning rule")
//!
//! Editing a `live` question copies the previous `prompt` / `code_snippet` /
//! `payload` / `explanation` into `question_versions` and bumps
//! `questions.version`. Editing a `draft` question does not version — drafts
//! are not yet in flight, so attempts can't reference them. Archived
//! questions reject edits outright.
//!
//! ## Batch limit
//!
//! [`create_questions`] enforces `MAX_BATCH = 50` per spec, mirroring the
//! limit advertised at the `POST /questions` boundary so callers fail fast
//! rather than discovering the cap after writing half a batch.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::bank::tags::{get_or_create_tags_tx, set_question_tags};
use crate::domain::error::{ApiError, FieldError};
use crate::domain::question::{
    CodeSnippet, Question, QuestionKind, QuestionStatus, QuestionVersion,
};

pub const MAX_BATCH: usize = 50;

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuestionInsert {
    pub kind: QuestionKind,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<CodeSnippet>,
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default = "default_points")]
    pub points: i32,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Partial update body for `PATCH /questions/:id`.
///
/// Field semantics: `None` means "leave unchanged", `Some(v)` means "set to v".
/// Clearing a nullable field (e.g. removing `explanation`) is not supported in
/// MVP — rewrite via a new question if you need to.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct QuestionPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<CodeSnippet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub payload: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct QuestionFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<QuestionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_rating: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rating: Option<f64>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

fn default_points() -> i32 {
    1
}

fn row_to_question(row: &sqlx::postgres::PgRow) -> Result<Question, ApiError> {
    let kind_str: String = row.get("kind");
    let status_str: String = row.get("status");
    let code_snippet_val: Option<serde_json::Value> = row.get("code_snippet");
    let code_snippet = match code_snippet_val {
        Some(v) => Some(serde_json::from_value(v).map_err(internal)?),
        None => None,
    };

    Ok(Question {
        id: row.get("id"),
        kind: QuestionKind::from_str(&kind_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid kind in DB: {e}")))?,
        prompt: row.get("prompt"),
        code_snippet,
        payload: row.get("payload"),
        explanation: row.get("explanation"),
        status: QuestionStatus::from_str(&status_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid status in DB: {e}")))?,
        source: row.get("source"),
        points: row.get("points"),
        rating: row.get("rating"),
        attempts_count: row.get("attempts_count"),
        version: row.get("version"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

const QUESTION_COLUMNS: &str = "id, kind, prompt, code_snippet, payload, explanation, status, \
     source, points, rating, attempts_count, version, created_by, created_at, updated_at";

pub async fn get_question(pool: &PgPool, id: Uuid) -> Result<Option<Question>, ApiError> {
    let row = sqlx::query(&format!(
        "SELECT {QUESTION_COLUMNS} FROM tb_questions WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;

    row.as_ref().map(row_to_question).transpose()
}

/// List question versions for an id, newest first (highest version first).
pub async fn list_question_versions(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Vec<QuestionVersion>, ApiError> {
    let rows = sqlx::query(
        "SELECT question_id, version, prompt, code_snippet, payload, explanation, archived_at \
         FROM tb_question_versions \
         WHERE question_id = $1 \
         ORDER BY version DESC",
    )
    .bind(question_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    rows.into_iter()
        .map(|row| {
            let code_snippet_val: Option<serde_json::Value> = row.get("code_snippet");
            let code_snippet = match code_snippet_val {
                Some(v) => Some(serde_json::from_value(v).map_err(internal)?),
                None => None,
            };
            Ok(QuestionVersion {
                question_id: row.get("question_id"),
                version: row.get("version"),
                prompt: row.get("prompt"),
                code_snippet,
                payload: row.get("payload"),
                explanation: row.get("explanation"),
                archived_at: row.get("archived_at"),
            })
        })
        .collect()
}

pub async fn list_questions(
    pool: &PgPool,
    filter: &QuestionFilter,
) -> Result<Vec<Question>, ApiError> {
    let status_str = filter.status.map(|s| s.as_str().to_string());
    let limit = filter.limit.clamp(1, 200);
    let offset = filter.offset.max(0);

    let rows = sqlx::query(
        "SELECT DISTINCT q.id, q.kind, q.prompt, q.code_snippet, q.payload, q.explanation, q.status, \
              q.source, q.points, q.rating, q.attempts_count, q.version, q.created_by, q.created_at, q.updated_at \
         FROM tb_questions q \
         LEFT JOIN tb_question_tags qt ON qt.question_id = q.id \
         LEFT JOIN tb_tags t ON t.id = qt.tag_id \
         WHERE ($1::text IS NULL OR t.name = $1) \
           AND ($2::text IS NULL OR q.status = $2) \
           AND ($3::double precision IS NULL OR q.rating >= $3) \
           AND ($4::double precision IS NULL OR q.rating <= $4) \
         ORDER BY q.created_at DESC \
         LIMIT $5 OFFSET $6",
    )
    .bind(filter.tag.as_deref())
    .bind(status_str)
    .bind(filter.min_rating)
    .bind(filter.max_rating)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    rows.iter().map(row_to_question).collect()
}

pub async fn create_questions(
    pool: &PgPool,
    created_by: Uuid,
    batch: Vec<QuestionInsert>,
) -> Result<Vec<Question>, ApiError> {
    if batch.is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "questions".into(),
            message: "batch must contain at least one question".into(),
        }]));
    }
    if batch.len() > MAX_BATCH {
        return Err(ApiError::Validation(vec![FieldError {
            field: "questions".into(),
            message: format!("batch exceeds max of {MAX_BATCH} (got {})", batch.len()),
        }]));
    }

    let mut tx = pool.begin().await.map_err(internal)?;
    let mut created = Vec::with_capacity(batch.len());

    for input in batch {
        if input.points < 0 {
            return Err(ApiError::Validation(vec![FieldError {
                field: "points".into(),
                message: "points must be >= 0".into(),
            }]));
        }

        let code_snippet_value = input
            .code_snippet
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(internal)?;

        let row = sqlx::query(&format!(
            "INSERT INTO tb_questions (kind, prompt, code_snippet, payload, explanation, source, points, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             RETURNING {QUESTION_COLUMNS}"
        ))
        .bind(input.kind.as_str())
        .bind(&input.prompt)
        .bind(code_snippet_value)
        .bind(&input.payload)
        .bind(input.explanation.as_deref())
        .bind(input.source.as_deref())
        .bind(input.points)
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await
        .map_err(internal)?;

        let question = row_to_question(&row)?;

        if !input.tags.is_empty() {
            let tag_records = get_or_create_tags_tx(&mut tx, &input.tags).await?;
            let tag_ids: Vec<Uuid> = tag_records.iter().map(|t| t.id).collect();
            set_question_tags(&mut tx, question.id, &tag_ids).await?;
        }

        created.push(question);
    }

    tx.commit().await.map_err(internal)?;
    Ok(created)
}

/// Apply a patch, honouring the versioning rule. See module rustdoc.
pub async fn update_question(
    pool: &PgPool,
    id: Uuid,
    patch: QuestionPatch,
) -> Result<Question, ApiError> {
    let mut tx = pool.begin().await.map_err(internal)?;

    let current_row = sqlx::query(
        "SELECT prompt, code_snippet, payload, explanation, status, version \
         FROM tb_questions WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound {
        resource: "question",
    })?;

    let current_status_str: String = current_row.get("status");
    let current_status = QuestionStatus::from_str(&current_status_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid status in DB: {e}")))?;

    if current_status == QuestionStatus::Archived {
        return Err(ApiError::Validation(vec![FieldError {
            field: "status".into(),
            message: "cannot edit an archived question".into(),
        }]));
    }
    if matches!(patch.points, Some(points) if points < 0) {
        return Err(ApiError::Validation(vec![FieldError {
            field: "points".into(),
            message: "points must be >= 0".into(),
        }]));
    }

    if current_status == QuestionStatus::Live {
        let prev_version: i32 = current_row.get("version");
        let prev_prompt: String = current_row.get("prompt");
        let prev_code_snippet: Option<serde_json::Value> = current_row.get("code_snippet");
        let prev_payload: serde_json::Value = current_row.get("payload");
        let prev_explanation: Option<String> = current_row.get("explanation");

        sqlx::query(
            "INSERT INTO tb_question_versions \
                 (question_id, version, prompt, code_snippet, payload, explanation) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(prev_version)
        .bind(&prev_prompt)
        .bind(&prev_code_snippet)
        .bind(&prev_payload)
        .bind(&prev_explanation)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    }

    let bump = matches!(current_status, QuestionStatus::Live);
    let code_snippet_value = patch
        .code_snippet
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(internal)?;

    let sql = if bump {
        "UPDATE tb_questions SET \
            prompt = COALESCE($2, prompt), \
            code_snippet = COALESCE($3, code_snippet), \
            payload = COALESCE($4, payload), \
            explanation = COALESCE($5, explanation), \
            points = COALESCE($6, points), \
            version = version + 1, \
            updated_at = now() \
         WHERE id = $1"
    } else {
        "UPDATE tb_questions SET \
            prompt = COALESCE($2, prompt), \
            code_snippet = COALESCE($3, code_snippet), \
            payload = COALESCE($4, payload), \
            explanation = COALESCE($5, explanation), \
            points = COALESCE($6, points), \
            updated_at = now() \
         WHERE id = $1"
    };

    sqlx::query(sql)
        .bind(id)
        .bind(patch.prompt.as_deref())
        .bind(code_snippet_value)
        .bind(patch.payload.as_ref())
        .bind(patch.explanation.as_deref())
        .bind(patch.points)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;

    if let Some(tag_names) = patch.tags.as_ref() {
        let tag_records = get_or_create_tags_tx(&mut tx, tag_names).await?;
        let tag_ids: Vec<Uuid> = tag_records.iter().map(|t| t.id).collect();
        set_question_tags(&mut tx, id, &tag_ids).await?;
    }

    let row = sqlx::query(&format!(
        "SELECT {QUESTION_COLUMNS} FROM tb_questions WHERE id = $1"
    ))
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal)?;
    let updated = row_to_question(&row)?;

    tx.commit().await.map_err(internal)?;
    Ok(updated)
}

async fn set_status(pool: &PgPool, id: Uuid, target: QuestionStatus) -> Result<Question, ApiError> {
    let row = sqlx::query(&format!(
        "UPDATE tb_questions SET status = $2, updated_at = now() WHERE id = $1 RETURNING {QUESTION_COLUMNS}"
    ))
    .bind(id)
    .bind(target.as_str())
    .fetch_optional(pool)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound {
        resource: "question",
    })?;
    row_to_question(&row)
}

pub async fn promote_question(pool: &PgPool, id: Uuid) -> Result<Question, ApiError> {
    set_status(pool, id, QuestionStatus::Live).await
}

pub async fn archive_question(pool: &PgPool, id: Uuid) -> Result<Question, ApiError> {
    set_status(pool, id, QuestionStatus::Archived).await
}
