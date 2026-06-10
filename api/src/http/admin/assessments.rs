use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::scope::RequireScope, domain::error::ApiError, http::AppState};

use super::AdminScope;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct AdminListAssessmentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
    pub mode: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminAssessmentEntry {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub mode: String,
    pub created_by: Uuid,
    pub created_by_email: Option<String>,
    pub objectives: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub deleted_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminListAssessmentsResponse {
    pub assessments: Vec<AdminAssessmentEntry>,
    pub total: i64,
}

/// GET /v1/admin/assessments — list all assessments on the platform
#[utoipa::path(
    get,
    path = "/v1/admin/assessments",
    params(AdminListAssessmentsQuery),
    responses(
        (status = 200, description = "Assessments retrieved successfully", body = AdminListAssessmentsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn list_assessments_admin(
    State(state): State<AppState>,
    _admin: RequireScope<AdminScope>,
    Query(query): Query<AdminListAssessmentsQuery>,
) -> Result<Json<AdminListAssessmentsResponse>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut count_qb = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM tb_assessments a
         LEFT JOIN tb_users u ON a.created_by = u.id",
    );
    let mut has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        count_qb.push(" WHERE (a.title ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR a.description ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR u.email ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR u.display_name ILIKE ");
        count_qb.push_bind(search_pat.clone());
        count_qb.push(" OR EXISTS (SELECT 1 FROM tb_users ow WHERE ow.id = u.owner_user_id AND ow.email ILIKE ");
        count_qb.push_bind(search_pat);
        count_qb.push("))");
        has_where = true;
    }

    if let Some(mode) = query.mode.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb.push(" AND a.mode = ");
        } else {
            count_qb.push(" WHERE a.mode = ");
            has_where = true;
        }
        count_qb.push_bind(mode);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            count_qb.push(" AND a.status = ");
        } else {
            count_qb.push(" WHERE a.status = ");
        }
        count_qb.push_bind(status);
    }

    let total: i64 = count_qb
        .build_query_as::<(i64,)>()
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .0;

    let mut qb = sqlx::QueryBuilder::new(
        "SELECT a.id, a.title, a.description, a.status, a.mode, a.created_by,
                COALESCE(u.email, (SELECT email FROM tb_users WHERE id = u.owner_user_id), u.display_name) as created_by_email,
                a.objectives, a.created_at, a.deleted_at
         FROM tb_assessments a
         LEFT JOIN tb_users u ON a.created_by = u.id"
    );

    let mut has_where = false;

    if let Some(search) = query.q.as_deref().filter(|s| !s.trim().is_empty()) {
        let search_pat = format!("%{}%", search.trim());
        qb.push(" WHERE (a.title ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR a.description ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR u.email ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR u.display_name ILIKE ");
        qb.push_bind(search_pat.clone());
        qb.push(" OR EXISTS (SELECT 1 FROM tb_users ow WHERE ow.id = u.owner_user_id AND ow.email ILIKE ");
        qb.push_bind(search_pat);
        qb.push("))");
        has_where = true;
    }

    if let Some(mode) = query.mode.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND a.mode = ");
        } else {
            qb.push(" WHERE a.mode = ");
            has_where = true;
        }
        qb.push_bind(mode);
    }

    if let Some(status) = query.status.as_deref().filter(|s| !s.trim().is_empty()) {
        if has_where {
            qb.push(" AND a.status = ");
        } else {
            qb.push(" WHERE a.status = ");
        }
        qb.push_bind(status);
    }

    qb.push(" ORDER BY a.created_at DESC, a.id DESC LIMIT ");
    qb.push_bind(limit);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb
        .build()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let assessments = rows
        .into_iter()
        .map(|r| {
            let status_str: String = r.get("status");
            let mode_str: String = r.get("mode");
            AdminAssessmentEntry {
                id: r.get("id"),
                title: r.get("title"),
                description: r.get("description"),
                status: status_str,
                mode: mode_str,
                created_by: r.get("created_by"),
                created_by_email: r.get("created_by_email"),
                objectives: r.get("objectives"),
                created_at: r.get("created_at"),
                deleted_at: r.get("deleted_at"),
            }
        })
        .collect();

    Ok(Json(AdminListAssessmentsResponse { assessments, total }))
}

/// DELETE /v1/admin/assessments/{id} — soft-delete any assessment (moderation)
#[utoipa::path(
    delete,
    path = "/v1/admin/assessments/{id}",
    params(("id" = Uuid, Path, description = "Assessment ID")),
    responses(
        (status = 204, description = "Assessment deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "Assessment not found"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn delete_assessment_admin(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let res = sqlx::query(
        "UPDATE tb_assessments SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "assessment",
        });
    }

    crate::audit::audit(
        state.pool.clone(),
        Some(admin.0.user.id),
        "assessment.delete_admin",
        Some("assessment"),
        Some(id),
        serde_json::json!({}),
    );

    Ok(StatusCode::NO_CONTENT)
}

/// POST /v1/admin/assessments/{id}/restore — restore a soft-deleted assessment
#[utoipa::path(
    post,
    path = "/v1/admin/assessments/{id}/restore",
    params(("id" = Uuid, Path, description = "Assessment ID")),
    responses(
        (status = 204, description = "Assessment restored successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden (Admin required)"),
        (status = 404, description = "Assessment not found or not deleted"),
    ),
    security(("bearer" = [])),
    tag = "admin"
)]
pub async fn restore_assessment_admin(
    State(state): State<AppState>,
    admin: RequireScope<AdminScope>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let res = sqlx::query(
        "UPDATE tb_assessments SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound {
            resource: "assessment",
        });
    }

    crate::audit::audit(
        state.pool.clone(),
        Some(admin.0.user.id),
        "assessment.restore_admin",
        Some("assessment"),
        Some(id),
        serde_json::json!({}),
    );

    Ok(StatusCode::NO_CONTENT)
}
