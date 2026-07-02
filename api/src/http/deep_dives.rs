//! Deep Dive request routes.

use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::io::Write;
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireAnyScope, ScopeOneOf},
    },
    domain::{
        error::{ApiError, FieldError},
        user::Scope,
    },
    http::{AppState, db::DbConn},
};

pub struct DeepDiveReadScopes;
impl ScopeOneOf for DeepDiveReadScopes {
    const SCOPES: &'static [Scope] = &[Scope::AssessmentRead, Scope::AttemptRead, Scope::Admin];
}

pub struct DeepDiveWriteScopes;
impl ScopeOneOf for DeepDiveWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::AttemptWrite, Scope::Admin];
}

pub struct DeepDivePublishScopes;
impl ScopeOneOf for DeepDivePublishScopes {
    const SCOPES: &'static [Scope] = &[Scope::AssessmentWrite, Scope::Admin];
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeepDive {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub question_id: Uuid,
    pub source_session_id: Option<Uuid>,
    pub source_attempt_id: Option<Uuid>,
    pub status: String,
    pub reason: Option<String>,
    pub body_markdown: Option<String>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub published_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub archived_at: Option<OffsetDateTime>,
    pub category: Option<String>,
    pub user_note: Option<String>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub note_updated_at: Option<OffsetDateTime>,
    pub question_prompt: String,
    pub question_kind: String,
    pub question_tags: Vec<String>,
    pub assessment_title: Option<String>,
    pub course: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeepDiveBody {
    pub question_id: Uuid,
    #[serde(default)]
    pub source_session_id: Option<Uuid>,
    #[serde(default)]
    pub source_attempt_id: Option<Uuid>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDeepDivesQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListDeepDivesResponse {
    pub deep_dives: Vec<DeepDive>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchDeepDiveBody {
    pub status: Option<String>,
    pub user_note: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublishDeepDiveBody {
    pub body_markdown: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

#[utoipa::path(
    post,
    path = "/v1/deep-dives",
    request_body = CreateDeepDiveBody,
    responses(
        (status = 201, description = "Deep dive created", body = DeepDive),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Question, session, or attempt not found"),
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn create_deep_dive(
    State(_state): State<AppState>,
    mut db: DbConn,
    auth: RequireAnyScope<DeepDiveWriteScopes>,
    Json(body): Json<CreateDeepDiveBody>,
) -> Result<(StatusCode, Json<DeepDive>), ApiError> {
    let auth = auth.0;
    let conn = &mut *db;
    validate_reason(&body.reason)?;
    ensure_question_access(conn, &auth, body.question_id).await?;
    validate_source(
        conn,
        auth.owner_id,
        body.question_id,
        body.source_session_id,
        body.source_attempt_id,
    )
    .await?;

    if let Some(existing) = find_active_duplicate(
        conn,
        auth.owner_id,
        body.question_id,
        body.source_session_id,
        body.source_attempt_id,
    )
    .await?
    {
        return Ok((StatusCode::OK, Json(existing)));
    }

    let row = sqlx::query(
        r#"
        INSERT INTO tb_deep_dives (
            owner_id, question_id, source_session_id, source_attempt_id, reason, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(auth.owner_id)
    .bind(body.question_id)
    .bind(body.source_session_id)
    .bind(body.source_attempt_id)
    .bind(body.reason.and_then(trim_to_none))
    .bind(auth.user.id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let id: Uuid = row.get("id");
    let dive = fetch_owned_deep_dive(conn, id, auth.owner_id).await?;
    Ok((StatusCode::CREATED, Json(dive)))
}

#[utoipa::path(
    get,
    path = "/v1/deep-dives",
    params(ListDeepDivesQuery),
    responses(
        (status = 200, description = "List of deep dives", body = ListDeepDivesResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn list_deep_dives(
    State(_state): State<AppState>,
    mut db: DbConn,
    auth: RequireAnyScope<DeepDiveReadScopes>,
    Query(query): Query<ListDeepDivesQuery>,
) -> Result<Json<ListDeepDivesResponse>, ApiError> {
    let auth = auth.0;
    let status = query.status.and_then(trim_to_none);
    if let Some(status) = status.as_deref() {
        validate_status(status, true)?;
    }

    let category = query.category.and_then(trim_to_none);
    let search = query.search.and_then(trim_to_none);

    let mut qb = base_select();
    qb.push(" WHERE d.owner_id = ");
    qb.push_bind(auth.owner_id);

    if let Some(status) = status {
        qb.push(" AND d.status = ");
        qb.push_bind(status);
    } else {
        qb.push(" AND d.status <> 'archived' ");
    }

    if let Some(category) = category {
        qb.push(" AND d.category = ");
        qb.push_bind(category);
    }

    if let Some(search) = search {
        let search_pat = format!("%{}%", search);
        qb.push(" AND (q.prompt ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR d.body_markdown ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR d.user_note ILIKE ");
        qb.push_bind(search_pat);
        qb.push(") ");
    }

    qb.push(" GROUP BY d.id, q.prompt, q.kind, ass.title, ass.course ORDER BY d.updated_at DESC");

    let rows = qb
        .build()
        .fetch_all(&mut *db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let dives = rows.into_iter().map(row_to_deep_dive).collect();

    Ok(Json(ListDeepDivesResponse { deep_dives: dives }))
}

#[utoipa::path(
    get,
    path = "/v1/deep-dives/{id}",
    params(
        ("id" = Uuid, Path, description = "Deep dive ID")
    ),
    responses(
        (status = 200, description = "Deep dive details", body = DeepDive),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Deep dive not found"),
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn get_deep_dive(
    State(_state): State<AppState>,
    mut db: DbConn,
    auth: RequireAnyScope<DeepDiveReadScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeepDive>, ApiError> {
    let dive = fetch_owned_deep_dive(&mut db, id, auth.0.owner_id).await?;
    Ok(Json(dive))
}

#[utoipa::path(
    patch,
    path = "/v1/deep-dives/{id}",
    params(
        ("id" = Uuid, Path, description = "Deep dive ID")
    ),
    request_body = PatchDeepDiveBody,
    responses(
        (status = 200, description = "Deep dive updated", body = DeepDive),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Deep dive not found"),
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn patch_deep_dive(
    State(state): State<AppState>,
    mut db: DbConn,
    auth: RequireAnyScope<DeepDiveWriteScopes>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchDeepDiveBody>,
) -> Result<Json<DeepDive>, ApiError> {
    let auth = auth.0;
    if body.status.is_none() && body.user_note.is_none() {
        let dive = fetch_owned_deep_dive(&mut db, id, auth.owner_id).await?;
        return Ok(Json(dive));
    }

    if let Some(ref status) = body.status {
        validate_patch_status(status)?;
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let is_admin = auth.user.role == crate::domain::user::Role::Admin;
    crate::http::db::set_rls_guc(&mut tx, auth.owner_id, is_admin).await?;

    let _existing = fetch_owned_deep_dive(&mut tx, id, auth.owner_id).await?;

    if let Some(ref status) = body.status {
        sqlx::query(
            r#"
            UPDATE tb_deep_dives
            SET status = $1,
                archived_at = CASE WHEN $1 = 'archived' THEN now() ELSE NULL END
            WHERE id = $2 AND owner_id = $3
            "#,
        )
        .bind(status)
        .bind(id)
        .bind(auth.owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    if let Some(ref user_note) = body.user_note {
        let trimmed_note = trim_to_none(user_note.clone());
        sqlx::query(
            r#"
            UPDATE tb_deep_dives
            SET user_note = $1,
                note_updated_at = now()
            WHERE id = $2 AND owner_id = $3
            "#,
        )
        .bind(&trimmed_note)
        .bind(id)
        .bind(auth.owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        let current = sqlx::query(
            "SELECT body_markdown, category FROM tb_deep_dives WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(auth.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        let current_body_markdown: Option<String> = current.get("body_markdown");
        let current_category: Option<String> = current.get("category");

        sqlx::query(
            r#"
            INSERT INTO tb_deep_dive_revisions (deep_dive_id, revision, body_markdown, category, user_note, created_by)
            VALUES (
                $1,
                COALESCE((SELECT MAX(revision) FROM tb_deep_dive_revisions WHERE deep_dive_id = $1), 0) + 1,
                $2,
                $3,
                $4,
                $5
            )
            "#,
        )
        .bind(id)
        .bind(current_body_markdown.as_ref())
        .bind(current_category.as_ref())
        .bind(trimmed_note.as_ref())
        .bind(auth.user.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let updated = fetch_owned_deep_dive(&mut db, id, auth.owner_id).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    patch,
    path = "/v1/deep-dives/{id}/publish",
    params(
        ("id" = Uuid, Path, description = "Deep dive ID")
    ),
    request_body = PublishDeepDiveBody,
    responses(
        (status = 200, description = "Deep dive published", body = DeepDive),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Deep dive not found"),
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn publish_deep_dive(
    State(state): State<AppState>,
    mut db: DbConn,
    auth: RequireAnyScope<DeepDivePublishScopes>,
    Path(id): Path<Uuid>,
    Json(body): Json<PublishDeepDiveBody>,
) -> Result<Json<DeepDive>, ApiError> {
    let auth = auth.0;
    let target_status = body.status.unwrap_or_else(|| "published".to_string());
    validate_status(&target_status, false)?;
    if target_status == "archived" {
        return Err(ApiError::Validation(vec![FieldError {
            field: "status".into(),
            message: "publish status cannot be archived".into(),
        }]));
    }
    let body_markdown =
        trim_to_none(body.body_markdown).ok_or(ApiError::Validation(vec![FieldError {
            field: "bodyMarkdown".into(),
            message: "must not be empty".into(),
        }]))?;

    let category = body.category.and_then(trim_to_none);

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let is_admin = auth.user.role == crate::domain::user::Role::Admin;
    crate::http::db::set_rls_guc(&mut tx, auth.owner_id, is_admin).await?;

    let existing = fetch_deep_dive_for_publish(&mut tx, id, &auth).await?;

    sqlx::query(
        r#"
        UPDATE tb_deep_dives
        SET body_markdown = $1,
            status = $2,
            category = $3,
            created_by = $4,
            published_at = CASE WHEN $2 = 'published' THEN now() ELSE published_at END,
            archived_at = NULL
        WHERE id = $5
        "#,
    )
    .bind(&body_markdown)
    .bind(&target_status)
    .bind(category.as_ref())
    .bind(auth.user.id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    sqlx::query(
        r#"
        INSERT INTO tb_deep_dive_revisions (deep_dive_id, revision, body_markdown, category, user_note, created_by)
        VALUES (
            $1,
            COALESCE((SELECT MAX(revision) FROM tb_deep_dive_revisions WHERE deep_dive_id = $1), 0) + 1,
            $2,
            $3,
            $4,
            $5
        )
        "#,
    )
    .bind(id)
    .bind(&body_markdown)
    .bind(category.as_ref())
    .bind(existing.user_note.as_ref())
    .bind(auth.user.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let updated = fetch_owned_deep_dive(&mut db, existing.id, existing.owner_id).await?;
    Ok(Json(updated))
}

fn base_select() -> sqlx::QueryBuilder<'static, sqlx::Postgres> {
    sqlx::QueryBuilder::new(
        r#"
        SELECT
            d.id,
            d.owner_id,
            d.question_id,
            d.source_session_id,
            d.source_attempt_id,
            d.status,
            d.reason,
            d.body_markdown,
            d.created_by,
            d.created_at,
            d.updated_at,
            d.published_at,
            d.archived_at,
            d.category,
            d.user_note,
            d.note_updated_at,
            q.prompt AS question_prompt,
            q.kind AS question_kind,
            COALESCE(ARRAY_REMOVE(ARRAY_AGG(DISTINCT t.name), NULL), ARRAY[]::text[]) AS question_tags,
            ass.title AS assessment_title,
            ass.course AS course
        FROM tb_deep_dives d
        JOIN tb_questions q ON q.id = d.question_id
        LEFT JOIN tb_question_tags qt ON qt.question_id = q.id
        LEFT JOIN tb_tags t ON t.id = qt.tag_id
        LEFT JOIN tb_sessions s ON s.id = d.source_session_id
        LEFT JOIN tb_assessments ass ON ass.id = s.assessment_id
        "#,
    )
}

fn row_to_deep_dive(row: sqlx::postgres::PgRow) -> DeepDive {
    DeepDive {
        id: row.get("id"),
        owner_id: row.get("owner_id"),
        question_id: row.get("question_id"),
        source_session_id: row.get("source_session_id"),
        source_attempt_id: row.get("source_attempt_id"),
        status: row.get("status"),
        reason: row.get("reason"),
        body_markdown: row.get("body_markdown"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        published_at: row.get("published_at"),
        archived_at: row.get("archived_at"),
        category: row.get("category"),
        user_note: row.get("user_note"),
        note_updated_at: row.get("note_updated_at"),
        question_prompt: row.get("question_prompt"),
        question_kind: row.get("question_kind"),
        question_tags: row.get("question_tags"),
        assessment_title: row.get("assessment_title"),
        course: row.get("course"),
    }
}

async fn fetch_owned_deep_dive(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
    owner_id: Uuid,
) -> Result<DeepDive, ApiError> {
    let row = base_select()
        .push(" WHERE d.id = ")
        .push_bind(id)
        .push(" AND d.owner_id = ")
        .push_bind(owner_id)
        .push(" GROUP BY d.id, q.prompt, q.kind, ass.title, ass.course")
        .build()
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::NotFound {
            resource: "deep_dive",
        })?;
    Ok(row_to_deep_dive(row))
}

async fn fetch_deep_dive_for_publish(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
    auth: &AuthenticatedUser,
) -> Result<DeepDive, ApiError> {
    let row = base_select()
        .push(" WHERE d.id = ")
        .push_bind(id)
        .push(" AND d.owner_id = ")
        .push_bind(auth.owner_id)
        .push(" GROUP BY d.id, q.prompt, q.kind, ass.title, ass.course")
        .build()
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    row.map(row_to_deep_dive).ok_or(ApiError::NotFound {
        resource: "deep_dive",
    })
}

async fn find_active_duplicate(
    conn: &mut sqlx::PgConnection,
    owner_id: Uuid,
    question_id: Uuid,
    source_session_id: Option<Uuid>,
    source_attempt_id: Option<Uuid>,
) -> Result<Option<DeepDive>, ApiError> {
    let row = base_select()
        .push(" WHERE d.owner_id = ")
        .push_bind(owner_id)
        .push(" AND d.question_id = ")
        .push_bind(question_id)
        .push(" AND d.source_session_id IS NOT DISTINCT FROM ")
        .push_bind(source_session_id)
        .push(" AND d.source_attempt_id IS NOT DISTINCT FROM ")
        .push_bind(source_attempt_id)
        .push(" AND d.archived_at IS NULL GROUP BY d.id, q.prompt, q.kind, ass.title, ass.course")
        .build()
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(row.map(row_to_deep_dive))
}

async fn ensure_question_access(
    conn: &mut sqlx::PgConnection,
    auth: &AuthenticatedUser,
    question_id: Uuid,
) -> Result<(), ApiError> {
    if crate::bank::questions::question_in_owner_scope(conn, question_id, auth.owner_id).await? {
        Ok(())
    } else {
        Err(ApiError::NotFound {
            resource: "question",
        })
    }
}

async fn validate_source(
    conn: &mut sqlx::PgConnection,
    owner_id: Uuid,
    question_id: Uuid,
    source_session_id: Option<Uuid>,
    source_attempt_id: Option<Uuid>,
) -> Result<(), ApiError> {
    if let Some(session_id) = source_session_id {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM tb_sessions
                WHERE id = $1
                  AND owner_id = $2
                  AND question_plan->'items' @> jsonb_build_array(jsonb_build_object('question_id', $3))
            )
            "#,
        )
        .bind(session_id)
        .bind(owner_id)
        .bind(question_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
        if !exists {
            return Err(ApiError::NotFound {
                resource: "session",
            });
        }
    }

    if let Some(attempt_id) = source_attempt_id {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM tb_attempts
                WHERE id = $1
                  AND owner_id = $2
                  AND question_id = $3
                  AND ($4::uuid IS NULL OR session_id = $4)
            )
            "#,
        )
        .bind(attempt_id)
        .bind(owner_id)
        .bind(question_id)
        .bind(source_session_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
        if !exists {
            return Err(ApiError::NotFound {
                resource: "attempt",
            });
        }
    }

    Ok(())
}

fn validate_reason(reason: &Option<String>) -> Result<(), ApiError> {
    if reason.as_ref().is_some_and(|s| s.len() > 500) {
        return Err(ApiError::Validation(vec![FieldError {
            field: "reason".into(),
            message: "must be 500 characters or fewer".into(),
        }]));
    }
    Ok(())
}

fn validate_patch_status(status: &str) -> Result<(), ApiError> {
    if matches!(status, "requested" | "needs_revision" | "archived") {
        return Ok(());
    }
    Err(ApiError::Validation(vec![FieldError {
        field: "status".into(),
        message: "must be requested, needs_revision, or archived".into(),
    }]))
}

fn validate_status(status: &str, allow_archived: bool) -> Result<(), ApiError> {
    let ok = matches!(
        status,
        "requested" | "drafting" | "published" | "needs_revision" | "archived"
    ) && (allow_archived || status != "archived");
    if ok {
        return Ok(());
    }
    Err(ApiError::Validation(vec![FieldError {
        field: "status".into(),
        message: "is not a supported deep dive status".into(),
    }]))
}

fn trim_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn sanitize_filename(name: &str) -> String {
    let mut sanitized = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') {
            sanitized.push(c);
        } else {
            sanitized.push('_');
        }
    }
    let mut truncated = sanitized.trim().to_string();
    if truncated.len() > 100 {
        truncated.truncate(100);
        truncated = truncated.trim_end_matches('_').to_string();
    }
    if truncated.is_empty() {
        truncated = "deep_dive".to_string();
    }
    truncated
}

#[utoipa::path(
    get,
    path = "/v1/deep-dives/export",
    responses(
        (status = 200, description = "ZIP archive of deep dives", body = Vec<u8>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn export_deep_dives(
    State(_state): State<AppState>,
    mut db: DbConn,
    auth: RequireAnyScope<DeepDiveReadScopes>,
) -> Result<impl IntoResponse, ApiError> {
    let auth = auth.0;

    let rows = base_select()
        .push(" WHERE d.owner_id = ")
        .push_bind(auth.owner_id)
        .push(" AND d.status = 'published' GROUP BY d.id, q.prompt, q.kind, ass.title, ass.course ORDER BY d.updated_at DESC")
        .build()
        .fetch_all(&mut *db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let dives: Vec<DeepDive> = rows.into_iter().map(row_to_deep_dive).collect();

    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut used_names = std::collections::HashSet::new();

        for dive in &dives {
            let category_dir =
                sanitize_filename(dive.category.as_deref().unwrap_or("Uncategorized"));
            let base_name = sanitize_filename(&dive.question_prompt);
            let mut relative_path = format!("{}/{}.md", category_dir, base_name);
            let mut counter = 1;
            while used_names.contains(&relative_path) {
                relative_path = format!("{}/{}_{}.md", category_dir, base_name, counter);
                counter += 1;
            }
            used_names.insert(relative_path.clone());

            let tags_str = if dive.question_tags.is_empty() {
                "[]".to_string()
            } else {
                let quoted: Vec<String> = dive
                    .question_tags
                    .iter()
                    .map(|t| format!("\"{}\"", t))
                    .collect();
                format!("[{}]", quoted.join(", "))
            };

            let category_str = dive.category.as_deref().unwrap_or("Uncategorized");

            let published_at_str = dive
                .published_at
                .map(|t| {
                    t.format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            let mut content = format!(
                r#"---
id: {}
category: {}
tags: {}
published_at: {}
---
# {}

{}
"#,
                dive.id,
                category_str,
                tags_str,
                published_at_str,
                dive.question_prompt,
                dive.body_markdown.as_deref().unwrap_or("")
            );

            if let Some(ref note) = dive.user_note {
                content.push_str("\n## Personal Notes\n\n");
                content.push_str(note);
                content.push('\n');
            }

            zip.start_file(&relative_path, options)
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("Zip error: {e}")))?;
            zip.write_all(content.as_bytes())
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("Zip error: {e}")))?;
        }

        zip.finish()
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("Zip error: {e}")))?;
    }

    let response = Response::builder()
        .header(CONTENT_TYPE, "application/zip")
        .header(
            CONTENT_DISPOSITION,
            "attachment; filename=\"deep-dives-export.zip\"",
        )
        .body(axum::body::Body::from(buf))
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Response builder error: {e}")))?;

    Ok(response)
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeepDiveRevision {
    pub id: Uuid,
    pub deep_dive_id: Uuid,
    pub revision: i32,
    pub body_markdown: Option<String>,
    pub category: Option<String>,
    pub user_note: Option<String>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeepDiveRevisionsResponse {
    pub revisions: Vec<DeepDiveRevision>,
}

#[utoipa::path(
    get,
    path = "/v1/deep-dives/{id}/revisions",
    params(
        ("id" = Uuid, Path, description = "Deep dive ID")
    ),
    responses(
        (status = 200, description = "List of revisions for the deep dive", body = DeepDiveRevisionsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Deep dive not found"),
    ),
    security(("bearer" = [])),
    tag = "deep-dives"
)]
pub async fn get_deep_dive_revisions(
    State(_state): State<AppState>,
    mut db: DbConn,
    auth: RequireAnyScope<DeepDiveReadScopes>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeepDiveRevisionsResponse>, ApiError> {
    let auth = auth.0;
    // Check ownership of the deep dive
    let _dive = fetch_owned_deep_dive(&mut db, id, auth.owner_id).await?;

    let rows = sqlx::query(
        r#"
        SELECT id, deep_dive_id, revision, body_markdown, category, user_note, created_by, created_at
        FROM tb_deep_dive_revisions
        WHERE deep_dive_id = $1
        ORDER BY revision DESC
        "#,
    )
    .bind(id)
    .fetch_all(&mut *db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let revisions = rows
        .into_iter()
        .map(|row| DeepDiveRevision {
            id: row.get("id"),
            deep_dive_id: row.get("deep_dive_id"),
            revision: row.get("revision"),
            body_markdown: row.get("body_markdown"),
            category: row.get("category"),
            user_note: row.get("user_note"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(DeepDiveRevisionsResponse { revisions }))
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/v1/deep-dives",
            post(create_deep_dive).get(list_deep_dives),
        )
        .route("/v1/deep-dives/export", get(export_deep_dives))
        .route(
            "/v1/deep-dives/{id}",
            get(get_deep_dive).patch(patch_deep_dive),
        )
        .route("/v1/deep-dives/{id}/publish", patch(publish_deep_dive))
        .route(
            "/v1/deep-dives/{id}/revisions",
            get(get_deep_dive_revisions),
        )
        .with_state(state)
}
