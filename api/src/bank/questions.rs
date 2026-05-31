//! Repository logic for bank questions.

use std::str::FromStr;

use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    domain::{
        error::ApiError,
        question::{Question, QuestionKind, QuestionStatus, QuestionVersion},
    },
};

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

#[derive(Debug, Default)]
pub struct QuestionFilter {
    pub tag: Option<String>,
    pub status: Option<QuestionStatus>,
    pub search: Option<String>,
    pub kind: Option<QuestionKind>,
    pub min_rating: Option<f64>,
    pub max_rating: Option<f64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub struct PagedQuestions {
    pub rows: Vec<Question>,
    pub total: i64,
    pub next_cursor: Option<String>,
}

pub async fn list_questions(
    pool: &PgPool,
    filter: QuestionFilter,
    limit: i64,
    offset: i64,
) -> Result<Vec<Question>, ApiError> {
    let status_str = filter.status.map(|s| s.as_str().to_string());
    let kind_str = filter.kind.map(|k| k.as_str().to_string());
    
    let rows = sqlx::query(
        "SELECT id, kind, prompt, version, status, points, code_snippet, payload, explanation, source, rating, attempts_count, created_by, created_at, updated_at
         FROM tb_questions q
         WHERE ($1::text IS NULL OR q.status = $1)
           AND ($2::text IS NULL OR q.kind = $2)
           AND ($3::text IS NULL OR q.prompt ILIKE '%' || $3 || '%')
           AND ($4::double precision IS NULL OR q.rating >= $4)
           AND ($5::double precision IS NULL OR q.rating <= $5)
           AND ($6::text IS NULL OR EXISTS (
               SELECT 1 FROM tb_question_tags qt
               JOIN tb_tags t ON t.id = qt.tag_id
               WHERE qt.question_id = q.id AND t.name = $6
           ))
         ORDER BY q.created_at DESC
         LIMIT $7 OFFSET $8"
    )
    .bind(status_str)
    .bind(kind_str)
    .bind(filter.search)
    .bind(filter.min_rating)
    .bind(filter.max_rating)
    .bind(filter.tag)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    rows.into_iter().map(row_to_question).collect()
}

pub async fn list_questions_paged(
    pool: &PgPool,
    filter: &QuestionFilter,
    after: Option<(OffsetDateTime, Uuid)>,
) -> Result<PagedQuestions, ApiError> {
    let limit = filter.limit.unwrap_or(50);
    let offset = filter.offset.unwrap_or(0);
    let status_str = filter.status.map(|s| s.as_str().to_string());
    let kind_str = filter.kind.map(|k| k.as_str().to_string());

    let (after_ts, after_id) = match after {
        Some((ts, id)) => (Some(ts), Some(id)),
        None => (None, None),
    };

    let rows = sqlx::query(
        "SELECT id, kind, prompt, version, status, points, code_snippet, payload, explanation, source, rating, attempts_count, created_by, created_at, updated_at,
                COUNT(*) OVER() as total
         FROM tb_questions q
         WHERE ($1::text IS NULL OR q.status = $1)
           AND ($2::text IS NULL OR q.kind = $2)
           AND ($3::text IS NULL OR q.prompt ILIKE '%' || $3 || '%')
           AND ($4::double precision IS NULL OR q.rating >= $4)
           AND ($5::double precision IS NULL OR q.rating <= $5)
           AND ($6::text IS NULL OR EXISTS (
               SELECT 1 FROM tb_question_tags qt
               JOIN tb_tags t ON t.id = qt.tag_id
               WHERE qt.question_id = q.id AND t.name = $6
           ))
           AND ($9::timestamptz IS NULL OR (q.created_at, q.id) < ($9, $10))
         ORDER BY q.created_at DESC, q.id DESC
         LIMIT $7 OFFSET $8"
    )
    .bind(status_str)
    .bind(kind_str)
    .bind(&filter.search)
    .bind(filter.min_rating)
    .bind(filter.max_rating)
    .bind(&filter.tag)
    .bind(limit)
    .bind(offset)
    .bind(after_ts)
    .bind(after_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    let total = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let questions: Result<Vec<_>, _> = rows.into_iter().map(row_to_question).collect();
    let rows_vec = questions?;
    
    let next_cursor = if rows_vec.len() as i64 == limit {
        rows_vec.last().map(|q| format!("{}_{}", q.created_at.unix_timestamp(), q.id))
    } else {
        None
    };

    Ok(PagedQuestions {
        rows: rows_vec,
        total,
        next_cursor,
    })
}

pub async fn get_question(pool: &PgPool, id: Uuid) -> Result<Option<Question>, ApiError> {
    let row = sqlx::query(
        "SELECT id, kind, prompt, version, status, points, code_snippet, payload, explanation, source, rating, attempts_count, created_by, created_at, updated_at
         FROM tb_questions WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;

    row.map(row_to_question).transpose()
}

pub fn decode_cursor(cursor: &str) -> Option<(OffsetDateTime, Uuid)> {
    let parts: Vec<&str> = cursor.split('_').collect();
    if parts.len() != 2 {
        return None;
    }
    let ts = OffsetDateTime::from_unix_timestamp(parts[0].parse().ok()?).ok()?;
    let id = Uuid::parse_str(parts[1]).ok()?;
    Some((ts, id))
}

fn row_to_question(row: sqlx::postgres::PgRow) -> Result<Question, ApiError> {
    let kind_str: String = row.get("kind");
    let status_str: String = row.get("status");
    Ok(Question {
        id: row.get("id"),
        kind: QuestionKind::from_str(&kind_str).map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        prompt: row.get("prompt"),
        version: row.get("version"),
        status: QuestionStatus::from_str(&status_str).map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        points: row.get("points"),
        code_snippet: row.get("code_snippet"),
        payload: row.get("payload"),
        explanation: row.get("explanation"),
        source: row.get("source"),
        rating: row.get("rating"),
        attempts_count: row.get("attempts_count"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn get_question_versions(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Vec<QuestionVersion>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, question_id, version, prompt, code_snippet, payload, explanation, archived_at, created_at
         FROM tb_question_versions
         WHERE question_id = $1
         ORDER BY version DESC"
    )
    .bind(question_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    rows.into_iter()
        .map(|row| {
            Ok(QuestionVersion {
                id: row.get("id"),
                question_id: row.get("question_id"),
                version: row.get("version"),
                prompt: row.get("prompt"),
                code_snippet: row.get("code_snippet"),
                payload: row.get("payload"),
                explanation: row.get("explanation"),
                archived_at: row.get("archived_at"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct QuestionInsert {
    pub kind: QuestionKind,
    pub prompt: String,
    pub payload: serde_json::Value,
    pub explanation: Option<String>,
    pub tags: Vec<String>,
    pub points: Option<i32>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct QuestionPatch {
    pub prompt: Option<String>,
    pub explanation: Option<String>,
    pub points: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub payload: Option<serde_json::Value>,
}

pub async fn create_questions(
    pool: &PgPool,
    user_id: Uuid,
    questions: Vec<QuestionInsert>,
) -> Result<Vec<Question>, ApiError> {
    let mut created = Vec::new();
    let mut tx = pool.begin().await.map_err(internal)?;

    for q in questions {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tb_questions (id, kind, prompt, payload, explanation, status, points, created_by)
             VALUES ($1, $2, $3, $4, $5, 'draft', $6, $7)"
        )
        .bind(id)
        .bind(q.kind.as_str())
        .bind(&q.prompt)
        .bind(&q.payload)
        .bind(&q.explanation)
        .bind(q.points.unwrap_or(1))
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;

        for tag_name in q.tags {
            let tag_id: Uuid = sqlx::query_scalar(
                "INSERT INTO tb_tags (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id"
            )
            .bind(&tag_name)
            .fetch_one(&mut *tx)
            .await
            .map_err(internal)?;

            sqlx::query("INSERT INTO tb_question_tags (question_id, tag_id) VALUES ($1, $2)")
                .bind(id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await
                .map_err(internal)?;
        }
        
        let q_row = sqlx::query("SELECT id, kind, prompt, version, status, points, code_snippet, payload, explanation, source, rating, attempts_count, created_by, created_at, updated_at FROM tb_questions WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(internal)?;
        created.push(row_to_question(q_row)?);
    }

    tx.commit().await.map_err(internal)?;
    Ok(created)
}

pub async fn update_question(
    pool: &PgPool,
    id: Uuid,
    patch: QuestionPatch,
) -> Result<Question, ApiError> {
    let mut tx = pool.begin().await.map_err(internal)?;

    if let Some(prompt) = patch.prompt {
        sqlx::query("UPDATE tb_questions SET prompt = $1 WHERE id = $2")
            .bind(prompt)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
    }
    // ... other fields ...
    
    tx.commit().await.map_err(internal)?;
    get_question(pool, id).await?.ok_or(ApiError::NotFound { resource: "question" })
}

pub async fn promote_question(pool: &PgPool, id: Uuid) -> Result<Question, ApiError> {
    sqlx::query("UPDATE tb_questions SET status = 'live' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(internal)?;
    get_question(pool, id).await?.ok_or(ApiError::NotFound { resource: "question" })
}

pub async fn archive_question(pool: &PgPool, id: Uuid) -> Result<Question, ApiError> {
    sqlx::query("UPDATE tb_questions SET status = 'archived' WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(internal)?;
    get_question(pool, id).await?.ok_or(ApiError::NotFound { resource: "question" })
}
