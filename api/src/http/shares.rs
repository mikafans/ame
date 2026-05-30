//! Share API: create, get, revoke share links and serve embed views.
//!
//! Privacy rules enforced at resolution time (§4.16):
//!   1. Never reveal other learners' attempts or scores.
//!   2. include_score is opt-in (default false).
//!   3. visibility='cohort' requires authenticated cohort membership (deferred; cohorts MVP).
//!
//! Router declaration order: `/quizzes/{id}/embed` MUST be declared before `/quizzes/{id}`.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::scope::{RequireAnyScope, ScopeOneOf},
    domain::{
        error::{ApiError, FieldError},
        user::{Role, Scope},
    },
    http::AppState,
};

// ── scope guards ──────────────────────────────────────────────────────────────

pub struct ShareScopes;
impl ScopeOneOf for ShareScopes {
    const SCOPES: &'static [Scope] = &[Scope::QuizRead, Scope::Admin];
}

// ── shapes ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateShareBody {
    pub kind: String,
    pub id: Uuid,
    pub visibility: Option<String>,
    pub include_explanation: Option<bool>,
    pub include_score: Option<bool>,
    pub include_attribution: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareCreated {
    pub share_id: Uuid,
    pub url: String,
    pub embed_url: String,
    pub og: OgMeta,
}

#[derive(Debug, Serialize)]
pub struct OgMeta {
    pub image: Option<String>,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareView {
    pub id: Uuid,
    pub kind: String,
    pub target_id: Uuid,
    pub visibility: String,
    pub include_score: bool,
    pub include_explanation: bool,
    pub include_attribution: bool,
    pub og_image_url: Option<String>,
    pub created_at: OffsetDateTime,
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn kind_to_plural(kind: &str) -> &'static str {
    match kind {
        "quiz" => "quizzes",
        "exam" => "exams",
        "item" => "items",
        _ => "items",
    }
}

// ── handlers ──────────────────────────────────────────────────────────────────

/// POST /v1/shares — create a share link (requires quiz.read).
pub async fn create_share(
    State(state): State<AppState>,
    user: RequireAnyScope<ShareScopes>,
    Json(body): Json<CreateShareBody>,
) -> Result<impl IntoResponse, ApiError> {
    if !["quiz", "exam", "item"].contains(&body.kind.as_str()) {
        return Err(ApiError::Validation(vec![FieldError {
            field: "kind".into(),
            message: "must be quiz, exam, or item".into(),
        }]));
    }
    let visibility = body.visibility.as_deref().unwrap_or("public").to_string();
    if !["public", "unlisted", "cohort"].contains(&visibility.as_str()) {
        return Err(ApiError::Validation(vec![FieldError {
            field: "visibility".into(),
            message: "must be public, unlisted, or cohort".into(),
        }]));
    }
    let include_explanation = body.include_explanation.unwrap_or(false);
    let include_score = body.include_score.unwrap_or(false);
    let include_attribution = body.include_attribution.unwrap_or(true);

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Auto-promote private quiz to unlisted if creating a share
    if body.kind == "quiz" {
        sqlx::query(
            "UPDATE tb_quizzes SET visibility = 'unlisted' WHERE id = $1 AND visibility = 'private'",
        )
        .bind(body.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    // Natural-tuple dedup
    let existing = sqlx::query(
        "SELECT id FROM tb_share_links
         WHERE created_by_user_id = $1
           AND kind = $2
           AND target_id = $3
           AND visibility = $4
           AND include_explanation = $5
           AND include_score = $6
           AND include_attribution = $7
           AND revoked_at IS NULL
         LIMIT 1",
    )
    .bind(user.0.user.id)
    .bind(&body.kind)
    .bind(body.id)
    .bind(&visibility)
    .bind(include_explanation)
    .bind(include_score)
    .bind(include_attribution)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let share_id: Uuid = if let Some(row) = existing {
        row.get("id")
    } else {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO tb_share_links
             (id, kind, target_id, created_by_user_id, visibility,
              include_explanation, include_score, include_attribution)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(&body.kind)
        .bind(body.id)
        .bind(user.0.user.id)
        .bind(&visibility)
        .bind(include_explanation)
        .bind(include_score)
        .bind(include_attribution)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
        id
    };

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let plural = kind_to_plural(&body.kind);
    Ok((
        StatusCode::CREATED,
        Json(ShareCreated {
            share_id,
            url: format!("/v1/shares/{share_id}"),
            embed_url: format!("/v1/{plural}/{}/embed", body.id),
            og: OgMeta {
                image: None,
                title: format!("Shared {}", body.kind),
                description: "View on Harus".to_string(),
            },
        }),
    ))
}

/// GET /v1/shares/{id} — public resolution.
pub async fn get_share(
    State(state): State<AppState>,
    Path(share_id): Path<Uuid>,
) -> Result<Json<ShareView>, ApiError> {
    let row = sqlx::query(
        "SELECT id, kind, target_id, created_by_user_id, visibility,
                include_explanation, include_score, include_attribution,
                og_image_url, created_at, revoked_at
         FROM tb_share_links WHERE id = $1",
    )
    .bind(share_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound { resource: "share" })?;

    let revoked_at: Option<OffsetDateTime> = row.get("revoked_at");
    if revoked_at.is_some() {
        return Err(ApiError::NotFound { resource: "share" });
    }

    Ok(Json(ShareView {
        id: row.get("id"),
        kind: row.get("kind"),
        target_id: row.get("target_id"),
        visibility: row.get("visibility"),
        include_score: row.get("include_score"),
        include_explanation: row.get("include_explanation"),
        include_attribution: row.get("include_attribution"),
        og_image_url: row.get("og_image_url"),
        created_at: row.get("created_at"),
    }))
}

/// DELETE /v1/shares/{id} — revoke (caller must be owner or admin).
pub async fn revoke_share(
    State(state): State<AppState>,
    user: RequireAnyScope<ShareScopes>,
    Path(share_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let row =
        sqlx::query("SELECT created_by_user_id, revoked_at FROM tb_share_links WHERE id = $1")
            .bind(share_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
            .ok_or(ApiError::NotFound { resource: "share" })?;

    let revoked_at: Option<OffsetDateTime> = row.get("revoked_at");
    if revoked_at.is_some() {
        return Err(ApiError::NotFound { resource: "share" });
    }

    let owner_id: Uuid = row.get("created_by_user_id");
    let is_admin = matches!(user.0.user.role, Role::Admin);
    if owner_id != user.0.user.id && !is_admin {
        return Err(ApiError::ScopeRequired("owner or admin"));
    }

    sqlx::query("UPDATE tb_share_links SET revoked_at = now() WHERE id = $1")
        .bind(share_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Inner embed logic — read-only presentation.
async fn embed_inner(
    pool: &sqlx::PgPool,
    kind: &'static str,
    target_id: Uuid,
) -> Result<Json<Value>, ApiError> {
    let share_row = sqlx::query(
        "SELECT id, include_explanation, include_score, include_attribution
         FROM tb_share_links
         WHERE kind = $1 AND target_id = $2 AND revoked_at IS NULL
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(kind)
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound { resource: "embed" })?;

    Ok(Json(json!({
        "shareId": share_row.get::<Uuid, _>("id"),
        "kind": kind,
        "targetId": target_id,
        "includeExplanation": share_row.get::<bool, _>("include_explanation"),
        "includeScore": share_row.get::<bool, _>("include_score"),
        "includeAttribution": share_row.get::<bool, _>("include_attribution"),
    })))
}

pub async fn embed_quiz(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    embed_inner(&state.pool, "quiz", id).await
}

pub async fn embed_exam(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    embed_inner(&state.pool, "exam", id).await
}

pub async fn embed_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    embed_inner(&state.pool, "item", id).await
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/shares", post(create_share))
        .route("/v1/shares/{id}", get(get_share).delete(revoke_share))
        .route("/v1/quizzes/{id}/embed", get(embed_quiz))
        .route("/v1/exams/{id}/embed", get(embed_exam))
        .route("/v1/items/{id}/embed", get(embed_item))
        .with_state(state)
}
