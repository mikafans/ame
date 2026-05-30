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

use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

fn default_limit() -> i64 {
    50
}

fn default_points() -> i32 {
    1
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QuestionRow {
    #[serde(flatten)]
    pub question: Question,
    pub tags: Vec<String>,
}

/// One page of the bank: the rows, the total for the filter, and a keyset
/// cursor pointing past the last row (when a further page may exist).
pub struct PagedQuestions {
    pub rows: Vec<QuestionRow>,
    pub total: i64,
    pub next_cursor: Option<String>,
}

/// Encode a keyset cursor from a row's sort keys: `<unix_nanos>_<uuid>`. The id
/// is unsuffixed by the nanos (digits only), so `split_once('_')` round-trips.
fn encode_cursor(created_at: OffsetDateTime, id: Uuid) -> String {
    format!("{}_{}", created_at.unix_timestamp_nanos(), id)
}

/// Decode a keyset cursor back into its `(created_at, id)` pair. Returns `None`
/// for any malformed cursor so the caller can surface a validation error.
pub fn decode_cursor(cursor: &str) -> Option<(OffsetDateTime, Uuid)> {
    let (nanos, id) = cursor.split_once('_')?;
    let ts = OffsetDateTime::from_unix_timestamp_nanos(nanos.parse().ok()?).ok()?;
    Some((ts, Uuid::parse_str(id).ok()?))
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

pub async fn list_questions_paged(
    pool: &PgPool,
    filter: &QuestionFilter,
    after: Option<(OffsetDateTime, Uuid)>,
) -> Result<PagedQuestions, ApiError> {
    let status_str = filter.status.map(|s| s.as_str().to_string());
    let limit = filter.limit.clamp(1, 200);
    let offset = filter.offset.max(0);

    // Filters shared by the page query and the count query. Params $1..$6 are
    // bound in the same order by both so the strings stay in lockstep.
    let where_clause = "WHERE ($1::text IS NULL OR EXISTS (SELECT 1 FROM tb_question_tags qt2 JOIN tb_tags t2 ON t2.id = qt2.tag_id WHERE qt2.question_id = q.id AND t2.name = $1)) \
           AND ($2::text IS NULL OR q.status = $2) \
           AND ($3::double precision IS NULL OR q.rating >= $3) \
           AND ($4::double precision IS NULL OR q.rating <= $4) \
           AND ($5::text IS NULL OR q.kind = $5) \
           AND ($6::text IS NULL OR q.prompt_tsv @@ plainto_tsquery('english', $6))";

    // Only pay for ts_rank when there's a search term. With no search the order
    // is plain created_at DESC, which idx_questions_created_at satisfies via an
    // index scan + LIMIT instead of sorting the whole table.
    let order_by = if filter.search.is_some() {
        "ORDER BY ts_rank(q.prompt_tsv, plainto_tsquery('english', $6)) DESC, q.created_at DESC, q.id DESC"
    } else {
        "ORDER BY q.created_at DESC, q.id DESC"
    };

    // Keyset seek: with the default created_at ordering a cursor turns deep
    // pagination from an O(offset) scan-and-discard into an index range scan.
    // A search reorders by ts_rank, so a (created_at,id) cursor can't apply.
    let use_keyset = filter.search.is_none() && after.is_some();

    let mut page_sql = format!(
        "SELECT q.id, q.kind, q.prompt, q.code_snippet, q.payload, q.explanation, q.status, \
                q.source, q.points, q.rating, q.attempts_count, q.version, q.created_by, q.created_at, q.updated_at \
         FROM tb_questions q {where_clause}"
    );
    if use_keyset {
        page_sql.push_str(" AND (q.created_at, q.id) < ($9, $10)");
    }
    page_sql.push_str(&format!(" {order_by} LIMIT $7 OFFSET $8"));

    let mut page_q = sqlx::query(&page_sql)
        .bind(filter.tag.as_deref())
        .bind(status_str.clone())
        .bind(filter.min_rating)
        .bind(filter.max_rating)
        .bind(filter.kind.as_deref())
        .bind(filter.search.as_deref())
        .bind(limit)
        .bind(offset);
    if use_keyset {
        let (ts, id) = after.expect("use_keyset implies after is Some");
        page_q = page_q.bind(ts).bind(id);
    }
    let rows = page_q.fetch_all(pool).await.map_err(internal)?;

    // Total for pagination. An exact COUNT re-scans the whole filtered set on
    // every page; for the unfiltered "browse the bank" case the planner's
    // reltuples estimate is accurate to a fraction of a percent and effectively
    // free, so use it there and reserve the exact count for filtered queries.
    let unfiltered = filter.tag.is_none()
        && filter.status.is_none()
        && filter.min_rating.is_none()
        && filter.max_rating.is_none()
        && filter.search.is_none()
        && filter.kind.is_none();
    let total: i64 = if unfiltered {
        let est: i64 = sqlx::query_scalar(
            "SELECT reltuples::bigint FROM pg_class WHERE relname = 'tb_questions'",
        )
        .fetch_one(pool)
        .await
        .map_err(internal)?;
        // reltuples is -1 until the table is first analysed; fall back to exact.
        if est >= 0 {
            est
        } else {
            count_filtered(pool, where_clause, filter, status_str.clone()).await?
        }
    } else {
        count_filtered(pool, where_clause, filter, status_str.clone()).await?
    };

    // Tags for just this page's rows, instead of joining + grouping the whole
    // bank. One round trip keyed by the returned ids.
    let ids: Vec<Uuid> = rows.iter().map(|r| r.get::<Uuid, _>("id")).collect();
    let mut tags_by_question: HashMap<Uuid, Vec<String>> = HashMap::new();
    if !ids.is_empty() {
        let tag_rows = sqlx::query(
            "SELECT qt.question_id, t.name FROM tb_question_tags qt \
             JOIN tb_tags t ON t.id = qt.tag_id \
             WHERE qt.question_id = ANY($1) \
             ORDER BY t.name",
        )
        .bind(&ids)
        .fetch_all(pool)
        .await
        .map_err(internal)?;
        for tr in &tag_rows {
            let qid: Uuid = tr.get("question_id");
            let name: String = tr.get("name");
            tags_by_question.entry(qid).or_default().push(name);
        }
    }

    let mut results = Vec::with_capacity(rows.len());
    for row in &rows {
        let question = row_to_question(row)?;
        let tags = tags_by_question.remove(&question.id).unwrap_or_default();
        results.push(QuestionRow { question, tags });
    }

    // A cursor to continue from past the last row, but only for the keyset-
    // compatible ordering and only when the page came back full (a short page
    // means there's nothing more to fetch).
    let next_cursor = if filter.search.is_none() && results.len() as i64 == limit {
        results
            .last()
            .map(|r| encode_cursor(r.question.created_at, r.question.id))
    } else {
        None
    };

    Ok(PagedQuestions {
        rows: results,
        total,
        next_cursor,
    })
}

/// Exact `COUNT(*)` over the active filter — bound identically to the page
/// query's `$1..$6` so the two stay in lockstep.
async fn count_filtered(
    pool: &PgPool,
    where_clause: &str,
    filter: &QuestionFilter,
    status_str: Option<String>,
) -> Result<i64, ApiError> {
    let count_sql = format!("SELECT count(*) FROM tb_questions q {where_clause}");
    sqlx::query_scalar(&count_sql)
        .bind(filter.tag.as_deref())
        .bind(status_str)
        .bind(filter.min_rating)
        .bind(filter.max_rating)
        .bind(filter.kind.as_deref())
        .bind(filter.search.as_deref())
        .fetch_one(pool)
        .await
        .map_err(internal)
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

    // Validate every row up front so one bad point value fails the whole batch
    // before we touch the DB (matches the prior per-row behaviour).
    for input in &batch {
        if input.points < 0 {
            return Err(ApiError::Validation(vec![FieldError {
                field: "points".into(),
                message: "points must be >= 0".into(),
            }]));
        }
    }

    // Client-side v7 ids let us map each returned row back to its input (and
    // its tags) without depending on RETURNING order, and keep the insert
    // time-ordered to match the created_at DESC, id DESC listing index.
    let ids: Vec<Uuid> = (0..batch.len()).map(|_| Uuid::now_v7()).collect();

    let mut kinds = Vec::with_capacity(batch.len());
    let mut prompts = Vec::with_capacity(batch.len());
    let mut code_snippets: Vec<Option<serde_json::Value>> = Vec::with_capacity(batch.len());
    let mut payloads = Vec::with_capacity(batch.len());
    let mut explanations: Vec<Option<String>> = Vec::with_capacity(batch.len());
    let mut sources: Vec<Option<String>> = Vec::with_capacity(batch.len());
    let mut points = Vec::with_capacity(batch.len());
    for input in &batch {
        kinds.push(input.kind.as_str().to_string());
        prompts.push(input.prompt.clone());
        code_snippets.push(
            input
                .code_snippet
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(internal)?,
        );
        payloads.push(input.payload.clone());
        explanations.push(input.explanation.clone());
        sources.push(input.source.clone());
        points.push(input.points);
    }

    let mut tx = pool.begin().await.map_err(internal)?;

    // One multi-row INSERT instead of N round-trips: unnest zips the column
    // arrays into rows; created_by is the same scalar for every row.
    let rows = sqlx::query(&format!(
        "INSERT INTO tb_questions \
             (id, kind, prompt, code_snippet, payload, explanation, source, points, created_by) \
         SELECT t.id, t.kind, t.prompt, t.code_snippet, t.payload, t.explanation, t.source, t.points, $9 \
         FROM unnest($1::uuid[], $2::text[], $3::text[], $4::jsonb[], $5::jsonb[], $6::text[], $7::text[], $8::int4[]) \
             AS t(id, kind, prompt, code_snippet, payload, explanation, source, points) \
         RETURNING {QUESTION_COLUMNS}"
    ))
    .bind(&ids)
    .bind(&kinds)
    .bind(&prompts)
    .bind(&code_snippets)
    .bind(&payloads)
    .bind(&explanations)
    .bind(&sources)
    .bind(&points)
    .bind(created_by)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;

    let mut by_id: HashMap<Uuid, Question> = HashMap::with_capacity(rows.len());
    for row in &rows {
        let question = row_to_question(row)?;
        by_id.insert(question.id, question);
    }

    // Bulk tag handling: upsert every distinct tag once, then link all
    // (question, tag) pairs in a single insert.
    let all_tag_names: Vec<String> = batch.iter().flat_map(|b| b.tags.clone()).collect();
    if !all_tag_names.is_empty() {
        let tag_records = get_or_create_tags_tx(&mut tx, &all_tag_names).await?;
        let tags_by_name: HashMap<String, Uuid> =
            tag_records.iter().map(|t| (t.name.clone(), t.id)).collect();

        let mut link_qids: Vec<Uuid> = Vec::new();
        let mut link_tids: Vec<Uuid> = Vec::new();
        for (input, &qid) in batch.iter().zip(ids.iter()) {
            let mut seen: Vec<Uuid> = Vec::new();
            for name in &input.tags {
                if let Some(&tid) = tags_by_name.get(&name.to_lowercase())
                    && !seen.contains(&tid)
                {
                    seen.push(tid);
                    link_qids.push(qid);
                    link_tids.push(tid);
                }
            }
        }

        if !link_qids.is_empty() {
            sqlx::query(
                "INSERT INTO tb_question_tags (question_id, tag_id) \
                 SELECT * FROM unnest($1::uuid[], $2::uuid[]) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(&link_qids)
            .bind(&link_tids)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        }
    }

    tx.commit().await.map_err(internal)?;

    // Return the questions in the original input order.
    let mut created = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(question) = by_id.remove(id) {
            created.push(question);
        }
    }
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
