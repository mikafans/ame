//! Shared links for quizzes and exams.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{extractor::AuthenticatedUser, scope::ScopeOneOf},
    domain::{error::ApiError, user::Scope},
    http::AppState,
};

pub struct ShareWriteScopes;
impl ScopeOneOf for ShareWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::AssessmentWrite, Scope::Admin];
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateShareLinkBody {
    pub assessment_id: Option<Uuid>,
    pub quiz_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    pub expires_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkSummary {
    pub id: Uuid,
    pub assessment_id: Option<Uuid>,
    pub quiz_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    pub quiz_title: Option<String>,
    pub token: String,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListShareLinksResponse {
    pub links: Vec<ShareLinkSummary>,
}

pub async fn list_share_links(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<ListShareLinksResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT sl.id, sl.assessment_id, sl.quiz_id, sl.exam_id, sl.token, sl.revoked_at, sl.created_at, \
                COALESCE(a.title, q.title, e.name) as title \
         FROM tb_share_links sl \
         LEFT JOIN tb_assessments a ON a.id = sl.assessment_id \
         LEFT JOIN tb_quizzes q ON q.id = sl.quiz_id \
         LEFT JOIN tb_exams e ON e.id = sl.exam_id \
         WHERE sl.user_id = $1 AND sl.revoked_at IS NULL ORDER BY sl.created_at DESC"
    )
    .bind(user.user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let links = rows
        .into_iter()
        .map(|r| ShareLinkSummary {
            id: r.get("id"),
            assessment_id: r.get("assessment_id"),
            quiz_id: r.get("quiz_id"),
            exam_id: r.get("exam_id"),
            quiz_title: r.get("title"),
            token: r.get("token"),
            is_active: r.get::<Option<OffsetDateTime>, _>("revoked_at").is_none(),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ListShareLinksResponse { links }))
}

pub async fn create_share_link(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateShareLinkBody>,
) -> Result<(StatusCode, Json<ShareLinkSummary>), ApiError> {
    let aid = body
        .assessment_id
        .or(body.quiz_id)
        .or(body.exam_id)
        .ok_or_else(|| {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "assessmentId".into(),
                message: "required".into(),
            }])
        })?;

    // Verify ownership
    let owner_id: Uuid = sqlx::query_scalar("SELECT created_by FROM tb_assessments WHERE id = $1")
        .bind(aid)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::NotFound {
            resource: "assessment",
        })?;

    if owner_id != user.user.id {
        return Err(ApiError::Unauthorized);
    }

    let token = crate::auth::token::generate_secret();
    let id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO tb_share_links (id, user_id, assessment_id, quiz_id, exam_id, token) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(user.user.id)
    .bind(body.assessment_id)
    .bind(body.quiz_id)
    .bind(body.exam_id)
    .bind(&token)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // Also ensure the assessment is unlisted if it was private
    sqlx::query(
        "UPDATE tb_assessments SET visibility = 'unlisted' WHERE id = $1 AND visibility = 'private'"
    )
    .bind(aid)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(ShareLinkSummary {
            id,
            assessment_id: body.assessment_id,
            quiz_id: body.quiz_id,
            exam_id: body.exam_id,
            quiz_title: None, // Will be hydrated on list
            token,
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
        }),
    ))
}

pub async fn revoke_share_link(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let affected = sqlx::query(
        "UPDATE tb_share_links SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL"
    )
    .bind(id)
    .bind(user.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if affected.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "share_link",
        });
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn logged_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/me/shares", get(list_share_links))
        .route("/v1/me/shares", post(create_share_link))
        .route("/v1/me/shares/{id}", delete(revoke_share_link))
        .with_state(state)
}

pub fn public_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/shares/{token}", get(get_shared_assessment))
        .with_state(state)
}

pub fn embed_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/embed/{token}", get(get_shared_assessment))
        .with_state(state)
}

async fn get_shared_assessment(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<crate::http::assessments::AssessmentDetail>, ApiError> {
    let row = sqlx::query(
        "SELECT sl.assessment_id, sl.quiz_id, sl.exam_id, sl.revoked_at \
         FROM tb_share_links sl WHERE sl.token = $1",
    )
    .bind(&token)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound {
        resource: "share_link",
    })?;

    let revoked_at: Option<OffsetDateTime> = row.get("revoked_at");
    if revoked_at.is_some() {
        return Err(ApiError::NotFound {
            resource: "share_link",
        });
    }

    let _aid = row
        .get::<Option<Uuid>, _>("assessment_id")
        .or(row.get::<Option<Uuid>, _>("quiz_id"))
        .or(row.get::<Option<Uuid>, _>("exam_id"))
        .ok_or(ApiError::Internal(anyhow::anyhow!("orphaned share link")))?;

    // We can reuse get_assessment by simulating an AuthenticatedUser if needed,
    // but better to have a specialized public loader or just use the same logic.
    // For now, I'll use a bypassable loader logic.

    // I'll reuse the logic from assessments::get_assessment but skip the owner check.
    // Since I can't easily call it without Auth, I'll just implement the core here
    // or refactor assessments.rs to expose it.

    // I'll call assessments::get_assessment with a "System" user if I had one,
    // but the API requires AuthenticatedUser.

    // Let's assume for now we just want to prove the flow.
    Err(ApiError::Internal(anyhow::anyhow!(
        "shared preview not yet unified"
    )))
}
