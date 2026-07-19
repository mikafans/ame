use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::error::ApiError,
    http::{AppState, db::DbConn},
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub assessments: serde_json::Value,
    pub questions: serde_json::Value,
    pub sessions: serde_json::Value,
    pub attempts: serde_json::Value,
    pub tag_ratings: serde_json::Value,
}

/// GET /v1/me/export — export the owner's data bundle
///
/// Returns assessments, questions, sessions, attempts, and tag ratings owned by this user
/// and any of their agents. Throttled via governor and audited.
#[utoipa::path(
    get,
    path = "/v1/me/export",
    responses(
        (status = 200, description = "Owner data successfully exported", body = ExportResponse),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Too Many Requests (Throttled)"),
    ),
    security(("bearer" = [])),
    tag = "export"
)]
pub async fn export_data(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    mut db: DbConn,
) -> Result<Json<ExportResponse>, ApiError> {
    let pool = &state.pool;
    let owner_id = auth.owner_id();

    // 2. Export assessments
    let assessments: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(json_agg(q), '[]'::json) FROM (
             SELECT * FROM tb_assessments 
             WHERE created_by = $1 OR created_by IN (SELECT id FROM tb_agents WHERE owner_user_id = $1)
         ) q",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // 3. Export questions
    let questions: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(json_agg(q), '[]'::json) FROM (
             SELECT * FROM tb_questions
             WHERE created_by = $1 OR created_by IN (SELECT id FROM tb_agents WHERE owner_user_id = $1)
         ) q",
    )
    .bind(owner_id)
    .fetch_one(&mut *db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // 4. Export sessions
    let sessions: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(json_agg(q), '[]'::json) FROM (
             SELECT * FROM tb_sessions 
             WHERE owner_id = $1
         ) q",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // 5. Export attempts
    let attempts: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(json_agg(q), '[]'::json) FROM (
             SELECT * FROM tb_attempts 
             WHERE owner_id = $1
         ) q",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // 6. Export ELO stats/tag ratings
    let tag_ratings: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(json_agg(q), '[]'::json) FROM (
             SELECT * FROM tb_user_tag_ratings 
             WHERE user_id = $1
         ) q",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    crate::audit::audit(
        state.pool.clone(),
        Some(auth.user.id),
        "data.export",
        None,
        None,
        serde_json::json!({
            "plan": "premium",
        }),
    );

    Ok(Json(ExportResponse {
        assessments,
        questions,
        sessions,
        attempts,
        tag_ratings,
    }))
}
