//! Repository logic for bank questions.

use std::str::FromStr;

use sqlx::{Connection, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{
    error::ApiError,
    question::{Question, QuestionKind, QuestionStatus, QuestionVersion},
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
    pub created_by: Option<Uuid>,
    pub assessment_id: Option<Uuid>,
}

pub struct PagedQuestions {
    pub rows: Vec<Question>,
    pub total: i64,
    pub next_cursor: Option<String>,
}

pub async fn list_questions(
    conn: &mut sqlx::PgConnection,
    filter: QuestionFilter,
    limit: i64,
    offset: i64,
) -> Result<Vec<Question>, ApiError> {
    let status_str = filter.status.map(|s| s.as_str().to_string());
    let kind_str = filter.kind.map(|k| k.as_str().to_string());

    let rows = sqlx::query(
        "SELECT id, kind, prompt, version, status, points, code_snippet, payload, explanation, deep_dive, source, rating, attempts_count, created_by, created_at, updated_at,
                COALESCE(ARRAY(SELECT t.name FROM tb_question_tags qt JOIN tb_tags t ON t.id = qt.tag_id WHERE qt.question_id = q.id ORDER BY t.name), '{}') AS tags
         FROM tb_questions q
         WHERE ($1::text IS NULL OR q.status = $1)
           AND ($2::text IS NULL OR q.kind = $2)
           AND ($3::text IS NULL OR q.prompt_tsv @@ websearch_to_tsquery('english', $3))
           AND ($4::double precision IS NULL OR q.rating >= $4)
           AND ($5::double precision IS NULL OR q.rating <= $5)
           AND ($6::text IS NULL OR EXISTS (
               SELECT 1 FROM tb_question_tags qt
               JOIN tb_tags t ON t.id = qt.tag_id
               WHERE qt.question_id = q.id AND t.name = $6
           ))
           AND ($9::uuid IS NULL OR q.created_by = $9 OR EXISTS (
               SELECT 1 FROM tb_agents ag WHERE ag.id = q.created_by AND ag.owner_user_id = $9
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
    .bind(filter.created_by)
    .fetch_all(conn)
    .await
    .map_err(internal)?;

    rows.into_iter().map(row_to_question).collect()
}

pub async fn list_questions_paged(
    conn: &mut sqlx::PgConnection,
    filter: &QuestionFilter,
    after: Option<(OffsetDateTime, Uuid)>,
) -> Result<PagedQuestions, ApiError> {
    let limit = filter.limit.unwrap_or(50);
    let query_limit = limit + 1;
    let offset = filter.offset.unwrap_or(0);
    let status_str = filter.status.map(|s| s.as_str().to_string());
    let kind_str = filter.kind.map(|k| k.as_str().to_string());

    let (after_ts, after_id) = match after {
        Some((ts, id)) => (Some(ts), Some(id)),
        None => (None, None),
    };

    let rows = sqlx::query(
        "SELECT id, kind, prompt, version, status, points, code_snippet, payload, explanation, deep_dive, source, rating, attempts_count, created_by, created_at, updated_at,
                COUNT(*) OVER() as total,
                COALESCE(ARRAY(SELECT t.name FROM tb_question_tags qt JOIN tb_tags t ON t.id = qt.tag_id WHERE qt.question_id = q.id ORDER BY t.name), '{}') AS tags
         FROM tb_questions q
         WHERE ($1::text IS NULL OR q.status = $1)
           AND ($2::text IS NULL OR q.kind = $2)
           AND ($3::text IS NULL OR q.prompt_tsv @@ websearch_to_tsquery('english', $3))
           AND ($4::double precision IS NULL OR q.rating >= $4)
           AND ($5::double precision IS NULL OR q.rating <= $5)
           AND ($6::text IS NULL OR EXISTS (
               SELECT 1 FROM tb_question_tags qt
               JOIN tb_tags t ON t.id = qt.tag_id
               WHERE qt.question_id = q.id AND t.name = $6
           ))
           AND ($9::timestamptz IS NULL OR (q.created_at, q.id) < ($9, $10))
           AND ($11::uuid IS NULL OR q.created_by = $11 OR EXISTS (
               SELECT 1 FROM tb_agents ag WHERE ag.id = q.created_by AND ag.owner_user_id = $11
           ))
           AND ($12::uuid IS NULL OR EXISTS (
               SELECT 1 FROM tb_assessment_items ai
               JOIN tb_assessment_sections asec ON ai.section_id = asec.id
               WHERE ai.question_id = q.id AND asec.assessment_id = $12
           ))
         ORDER BY q.created_at DESC, q.id DESC
         LIMIT $7 OFFSET $8"
    )
    .bind(status_str)
    .bind(kind_str)
    .bind(&filter.search)
    .bind(filter.min_rating)
    .bind(filter.max_rating)
    .bind(&filter.tag)
    .bind(query_limit)
    .bind(offset)
    .bind(after_ts)
    .bind(after_id)
    .bind(filter.created_by)
    .bind(filter.assessment_id)
    .fetch_all(conn)
    .await
    .map_err(internal)?;

    let total = rows.first().map(|r| r.get("total")).unwrap_or(0);
    let questions: Result<Vec<_>, _> = rows.into_iter().map(row_to_question).collect();
    let mut rows_vec = questions?;

    let has_more = rows_vec.len() as i64 > limit;
    if has_more {
        rows_vec.truncate(limit as usize);
    }

    let next_cursor = if has_more {
        rows_vec
            .last()
            .map(|q| format!("{}_{}", q.created_at.unix_timestamp_nanos(), q.id))
    } else {
        None
    };

    Ok(PagedQuestions {
        rows: rows_vec,
        total,
        next_cursor,
    })
}

pub async fn get_question(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
) -> Result<Option<Question>, ApiError> {
    let row = sqlx::query(
        "SELECT id, kind, prompt, version, status, points, code_snippet, payload, explanation, deep_dive, source, rating, attempts_count, created_by, created_at, updated_at,
                COALESCE(ARRAY(SELECT t.name FROM tb_question_tags qt JOIN tb_tags t ON t.id = qt.tag_id WHERE qt.question_id = q.id ORDER BY t.name), '{}') AS tags
         FROM tb_questions q WHERE q.id = $1"
    )
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(internal)?;

    row.map(row_to_question).transpose()
}

pub fn decode_cursor(cursor: &str) -> Option<(OffsetDateTime, Uuid)> {
    let parts: Vec<&str> = cursor.split('_').collect();
    if parts.len() != 2 {
        return None;
    }
    let ns = parts[0].parse::<i128>().ok()?;
    let ts = OffsetDateTime::from_unix_timestamp_nanos(ns).ok()?;
    let id = Uuid::parse_str(parts[1]).ok()?;
    Some((ts, id))
}

fn row_to_question(row: sqlx::postgres::PgRow) -> Result<Question, ApiError> {
    let kind_str: String = row.get("kind");
    let status_str: String = row.get("status");
    Ok(Question {
        id: row.get("id"),
        kind: QuestionKind::from_str(&kind_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        prompt: row.get("prompt"),
        tags: row.get("tags"),
        version: row.get("version"),
        status: QuestionStatus::from_str(&status_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        points: row.get("points"),
        code_snippet: row.get("code_snippet"),
        payload: row.get("payload"),
        explanation: row.get("explanation"),
        deep_dive: row.get("deep_dive"),
        source: row.get("source"),
        rating: row.get("rating"),
        attempts_count: row.get("attempts_count"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn get_question_versions(
    conn: &mut sqlx::PgConnection,
    question_id: Uuid,
) -> Result<Vec<QuestionVersion>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, question_id, version, prompt, code_snippet, payload, explanation, deep_dive, source, archived_at, created_at
         FROM tb_question_versions
         WHERE question_id = $1
         ORDER BY version DESC"
    )
    .bind(question_id)
    .fetch_all(conn)
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
                deep_dive: row.get("deep_dive"),
                source: row.get("source"),
                archived_at: row.get("archived_at"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

/// Returns true if the question with `question_id` is within `owner_id`'s
/// scope: either the question was created directly by `owner_id`, or it was
/// created by an agent whose `owner_user_id` is `owner_id`.
///
/// Mirrors the EXISTS predicate used in [`list_questions_paged`] so per-item
/// access checks are consistent with the list view.
pub async fn question_in_owner_scope(
    conn: &mut sqlx::PgConnection,
    question_id: Uuid,
    owner_id: Uuid,
) -> Result<bool, ApiError> {
    let in_scope: bool = sqlx::query_scalar(
        "SELECT EXISTS(
             SELECT 1 FROM tb_questions q
             WHERE q.id = $1
               AND (q.created_by = $2 OR EXISTS (
                   SELECT 1 FROM tb_agents ag
                   WHERE ag.id = q.created_by AND ag.owner_user_id = $2
               ))
         )",
    )
    .bind(question_id)
    .bind(owner_id)
    .fetch_one(conn)
    .await
    .map_err(internal)?;

    Ok(in_scope)
}

pub const MAX_BATCH: usize = 250;

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct QuestionInsert {
    pub kind: QuestionKind,
    pub prompt: String,
    pub payload: serde_json::Value,
    pub explanation: Option<String>,
    pub deep_dive: Option<String>,
    pub source: Option<String>,
    pub tags: Vec<String>,
    pub points: Option<i32>,
    pub status: Option<QuestionStatus>,
}

#[derive(Debug, Default, serde::Deserialize, utoipa::ToSchema)]
pub struct QuestionPatch {
    pub prompt: Option<String>,
    pub explanation: Option<String>,
    pub deep_dive: Option<String>,
    pub source: Option<String>,
    pub points: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub payload: Option<serde_json::Value>,
}

pub async fn create_questions(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    owner_id: Uuid,
    questions: Vec<QuestionInsert>,
) -> Result<Vec<Question>, ApiError> {
    if questions.len() > MAX_BATCH {
        return Err(ApiError::Validation(vec![
            crate::domain::error::FieldError {
                field: "questions".to_string(),
                message: format!("max {MAX_BATCH} questions per batch"),
            },
        ]));
    }
    let mut created = Vec::new();
    let mut tx = conn.begin().await.map_err(internal)?;

    for q in questions {
        if q.source
            .as_ref()
            .is_some_and(|s| !s.starts_with("http://") && !s.starts_with("https://"))
        {
            return Err(ApiError::Validation(vec![
                crate::domain::error::FieldError {
                    field: "source".to_string(),
                    message: "reference URL must use http or https scheme".to_string(),
                },
            ]));
        }

        let id = Uuid::now_v7();
        let status = q.status.unwrap_or(QuestionStatus::Draft);

        // Normalize tags upfront so we can use them in the final Question object
        let normalized_tags: Vec<String> = q
            .tags
            .iter()
            .map(|t| crate::bank::tags::normalize_tag(t))
            .collect();

        let q_row = sqlx::query(
            "INSERT INTO tb_questions (id, owner_id, kind, prompt, payload, explanation, deep_dive, source, status, points, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING id, kind, prompt, version, status, points, code_snippet, payload, explanation, deep_dive, source, rating, attempts_count, created_by, created_at, updated_at, '{}'::text[] AS tags"
        )
        .bind(id)
        .bind(owner_id)
        .bind(q.kind.as_str())
        .bind(&q.prompt)
        .bind(&q.payload)
        .bind(&q.explanation)
        .bind(&q.deep_dive)
        .bind(&q.source)
        .bind(status.as_str())
        .bind(q.points.unwrap_or(1))
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(internal)?;

        for tag_name in &normalized_tags {
            let tag_id: Uuid = sqlx::query_scalar(
                "INSERT INTO tb_tags (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id"
            )
            .bind(tag_name)
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

        // Build Question from the INSERT result. The RETURNING tags column is
        // empty (tag rows are inserted after the question), so attach the
        // in-memory tags, sorted to match the old ORDER BY t.name projection.
        let mut question = row_to_question(q_row)?;
        let mut tags = normalized_tags;
        tags.sort();
        question.tags = tags;
        created.push(question);
    }

    tx.commit().await.map_err(internal)?;
    Ok(created)
}

pub async fn update_question(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
    patch: QuestionPatch,
) -> Result<Question, ApiError> {
    let mut tx = conn.begin().await.map_err(internal)?;

    if patch
        .source
        .as_ref()
        .is_some_and(|s| !s.starts_with("http://") && !s.starts_with("https://"))
    {
        return Err(ApiError::Validation(vec![
            crate::domain::error::FieldError {
                field: "source".to_string(),
                message: "reference URL must use http or https scheme".to_string(),
            },
        ]));
    }

    // Fetch current state and check if editable.
    let current_row = sqlx::query(
        "SELECT version, status, prompt, code_snippet, payload, explanation, deep_dive, source FROM tb_questions WHERE id = $1 FOR UPDATE"
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound { resource: "question" })?;

    let current_status: String = current_row.get("status");
    if current_status == "archived" {
        return Err(ApiError::Validation(vec![
            crate::domain::error::FieldError {
                field: "status".to_string(),
                message: "archived questions cannot be edited".to_string(),
            },
        ]));
    }

    // If the question is live, snapshot the current version before mutating.
    if current_status == "live" {
        let current_version: i32 = current_row.get("version");
        let current_prompt: String = current_row.get("prompt");
        let current_code_snippet: Option<serde_json::Value> = current_row.get("code_snippet");
        let current_payload: serde_json::Value = current_row.get("payload");
        let current_explanation: Option<String> = current_row.get("explanation");
        let current_deep_dive: Option<String> = current_row.get("deep_dive");
        let current_source: Option<String> = current_row.get("source");

        sqlx::query(
            "INSERT INTO tb_question_versions (question_id, version, prompt, code_snippet, payload, explanation, deep_dive, source) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(id)
        .bind(current_version)
        .bind(current_prompt)
        .bind(current_code_snippet)
        .bind(current_payload)
        .bind(current_explanation)
        .bind(current_deep_dive)
        .bind(current_source)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;

        sqlx::query(
            "UPDATE tb_questions SET version = version + 1, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    }

    // Apply patch fields in a single UPDATE statement.
    sqlx::query(
        "UPDATE tb_questions \
         SET prompt = COALESCE($1, prompt), \
             explanation = COALESCE($2, explanation), \
             deep_dive = COALESCE($3, deep_dive), \
             points = COALESCE($4, points), \
             payload = COALESCE($5, payload), \
             source = COALESCE($6, source), \
             updated_at = now() \
         WHERE id = $7",
    )
    .bind(patch.prompt.as_deref())
    .bind(patch.explanation.as_deref())
    .bind(patch.deep_dive.as_deref())
    .bind(patch.points)
    .bind(patch.payload)
    .bind(patch.source.as_deref())
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    if let Some(tags) = &patch.tags {
        // Replace tags: delete existing, re-insert new ones.
        sqlx::query("DELETE FROM tb_question_tags WHERE question_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        for tag_name in tags {
            let normalized = crate::bank::tags::normalize_tag(tag_name);
            let tag_id: Uuid = sqlx::query_scalar(
                "INSERT INTO tb_tags (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id"
            )
            .bind(&normalized)
            .fetch_one(&mut *tx)
            .await
            .map_err(internal)?;
            sqlx::query("INSERT INTO tb_question_tags (question_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await
                .map_err(internal)?;
        }
    }

    tx.commit().await.map_err(internal)?;
    get_question(conn, id).await?.ok_or(ApiError::NotFound {
        resource: "question",
    })
}

pub async fn promote_question(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
) -> Result<Question, ApiError> {
    sqlx::query("UPDATE tb_questions SET status = 'live' WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(internal)?;
    get_question(conn, id).await?.ok_or(ApiError::NotFound {
        resource: "question",
    })
}

pub async fn archive_question(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
) -> Result<Question, ApiError> {
    sqlx::query("UPDATE tb_questions SET status = 'archived' WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(internal)?;
    get_question(conn, id).await?.ok_or(ApiError::NotFound {
        resource: "question",
    })
}

pub async fn get_related_questions(
    conn: &mut sqlx::PgConnection,
    question_id: Uuid,
    exclude: &[Uuid],
) -> Result<Vec<Question>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT q.id, q.kind, q.prompt, q.version, q.status, q.points, q.code_snippet, q.payload, q.explanation, q.deep_dive, q.source, q.rating, q.attempts_count, q.created_by, q.created_at, q.updated_at,
               COALESCE(ARRAY(SELECT t.name FROM tb_question_tags qt JOIN tb_tags t ON t.id = qt.tag_id WHERE qt.question_id = q.id ORDER BY t.name), '{}') AS tags,
               COUNT(qt_match.tag_id) AS shared_tag_count
        FROM tb_questions q
        JOIN tb_question_tags qt_self ON qt_self.question_id = q.id
        JOIN tb_question_tags qt_match ON qt_match.tag_id = qt_self.tag_id
        WHERE qt_match.question_id = $1
          AND q.id != $1
          AND q.status = 'live'
          AND NOT (q.id = ANY($2))
        GROUP BY q.id
        ORDER BY shared_tag_count DESC, q.rating DESC, q.created_at DESC
        LIMIT 5
        "#
    )
    .bind(question_id)
    .bind(exclude)
    .fetch_all(conn)
    .await
    .map_err(internal)?;

    let mut related = Vec::new();
    for row in rows {
        related.push(row_to_question(row)?);
    }
    Ok(related)
}
