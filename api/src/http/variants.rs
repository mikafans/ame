//! Learner-requested content variants backed by the generation lifecycle.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{patch, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::{ApiError, FieldError},
        generation::{
            GenerationRepository, GenerationStatus, StartGenerationRun, validate_content_run,
        },
    },
    generation_postgres::PgGenerationRepository,
    http::AppState,
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VariantKind {
    Explanation,
    Example,
    Difficulty,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestVariantBody {
    pub source_activity_id: Uuid,
    pub objective_id: Uuid,
    pub variant_kind: VariantKind,
    pub recommendation_reason: String,
    pub requested_difficulty: Option<String>,
    pub provider: Option<String>,
    pub retry_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublishVariantBody {
    pub content: Value,
    pub source_references: Vec<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query, rename_all = "camelCase")]
pub struct VariantQuery {
    pub activity_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VariantResponse {
    pub id: Uuid,
    pub source_activity_id: Uuid,
    pub objective_id: Uuid,
    pub generation_run_id: Uuid,
    pub variant_kind: VariantKind,
    pub recommendation_reason: String,
    pub requested_difficulty: Option<String>,
    pub status: GenerationStatus,
    pub error: Option<Value>,
    pub content: Option<Value>,
    pub source_references: Vec<String>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/v1/learning-variants",
            post(request_variant).get(list_variants),
        )
        .route("/v1/learning-variants/{id}", patch(publish_variant))
        .with_state(state)
}

#[utoipa::path(post, path = "/api/v1/learning-variants", request_body = RequestVariantBody, responses((status = 201, body = VariantResponse)), security(("bearer" = [])), tag = "variants")]
pub async fn request_variant(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<RequestVariantBody>,
) -> Result<(axum::http::StatusCode, Json<VariantResponse>), ApiError> {
    if body.recommendation_reason.trim().is_empty() {
        return Err(validation(
            "recommendationReason",
            "must explain why this variant helps",
        ));
    }
    let owns_anchor = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM tb_activities a
            JOIN tb_activity_objectives ao ON ao.activity_id = a.id
            WHERE a.id = $1 AND ao.objective_id = $2 AND a.subject_user_id = $3
        )",
    )
    .bind(body.source_activity_id)
    .bind(body.objective_id)
    .bind(auth.owner_id())
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    if !owns_anchor {
        return Err(ApiError::NotFound {
            resource: "activity objective",
        });
    }
    let operation = format!("learning.variant.{}", kind_name(body.variant_kind));
    let run = PgGenerationRepository::new(state.pool.clone())
        .start(StartGenerationRun {
            subject_user_id: auth.owner_id(),
            source_actor_id: auth.owner_id(),
            operation,
            provider: body.provider,
            retry_key: Some(body.retry_key),
            content_version: 1,
        })
        .await
        .map_err(crate::http::generation::map_error)?;
    let row = sqlx::query(
        "INSERT INTO tb_learning_variants
            (id, subject_user_id, source_activity_id, objective_id,
             generation_run_id, variant_kind, recommendation_reason,
             requested_difficulty)
         SELECT $1, $2, a.id, o.id, $3, $4, $5, $6
         FROM tb_activities a
         JOIN tb_activity_objectives ao ON ao.activity_id = a.id
         JOIN tb_journey_objectives o ON o.id = ao.objective_id
         WHERE a.id = $7 AND o.id = $8 AND a.subject_user_id = $2
         ON CONFLICT (generation_run_id) DO UPDATE
         SET generation_run_id = EXCLUDED.generation_run_id
         RETURNING *",
    )
    .bind(Uuid::now_v7())
    .bind(auth.owner_id())
    .bind(run.id)
    .bind(kind_name(body.variant_kind))
    .bind(&body.recommendation_reason)
    .bind(&body.requested_difficulty)
    .bind(body.source_activity_id)
    .bind(body.objective_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound {
        resource: "activity objective",
    })?;
    if row.get::<Uuid, _>("source_activity_id") != body.source_activity_id
        || row.get::<Uuid, _>("objective_id") != body.objective_id
        || row.get::<String, _>("recommendation_reason") != body.recommendation_reason
        || row.get::<Option<String>, _>("requested_difficulty") != body.requested_difficulty
    {
        return Err(validation(
            "retryKey",
            "conflicts with another variant request",
        ));
    }
    Ok((
        axum::http::StatusCode::CREATED,
        Json(row_response(&row, run.status, run.error)?),
    ))
}

#[utoipa::path(get, path = "/api/v1/learning-variants", params(VariantQuery), responses((status = 200, body = [VariantResponse])), security(("bearer" = [])), tag = "variants")]
pub async fn list_variants(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<VariantQuery>,
) -> Result<Json<Vec<VariantResponse>>, ApiError> {
    let rows = sqlx::query(
        "SELECT v.*, g.status, g.error FROM tb_learning_variants v
         JOIN tb_generation_runs g ON g.id = v.generation_run_id
         WHERE v.subject_user_id = $1 AND v.source_activity_id = $2
         ORDER BY v.created_at DESC",
    )
    .bind(auth.owner_id())
    .bind(query.activity_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal)?;
    rows.iter()
        .map(|row| row_response(row, generation_status(row.get("status"))?, row.get("error")))
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

#[utoipa::path(patch, path = "/api/v1/learning-variants/{id}", params(("id" = Uuid, Path)), request_body = PublishVariantBody, responses((status = 200, body = VariantResponse)), security(("bearer" = [])), tag = "variants")]
pub async fn publish_variant(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PublishVariantBody>,
) -> Result<Json<VariantResponse>, ApiError> {
    crate::http::citations::certify_references(
        state.pool.clone(),
        auth.owner_id(),
        &body.source_references,
    )
    .await?;
    let row =
        sqlx::query("SELECT * FROM tb_learning_variants WHERE id = $1 AND subject_user_id = $2")
            .bind(id)
            .bind(auth.owner_id())
            .fetch_optional(&state.pool)
            .await
            .map_err(internal)?
            .ok_or(ApiError::NotFound {
                resource: "learning variant",
            })?;
    let run = PgGenerationRepository::new(state.pool.clone())
        .get(auth.owner_id(), row.get("generation_run_id"))
        .await
        .map_err(crate::http::generation::map_error)?;
    validate_content_run(
        &run,
        auth.owner_id(),
        &format!("learning.variant.{}", row.get::<String, _>("variant_kind")),
    )
    .map_err(crate::http::generation::map_error)?;
    let row = sqlx::query(
        "UPDATE tb_learning_variants
         SET content = $1, source_references = $2, updated_at = now()
         WHERE id = $3 RETURNING *",
    )
    .bind(body.content)
    .bind(serde_json::json!(body.source_references))
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    row_response(&row, run.status, run.error).map(Json)
}

fn row_response(
    row: &sqlx::postgres::PgRow,
    status: GenerationStatus,
    error: Option<Value>,
) -> Result<VariantResponse, ApiError> {
    Ok(VariantResponse {
        id: row.get("id"),
        source_activity_id: row.get("source_activity_id"),
        objective_id: row.get("objective_id"),
        generation_run_id: row.get("generation_run_id"),
        variant_kind: parse_kind(row.get("variant_kind"))?,
        recommendation_reason: row.get("recommendation_reason"),
        requested_difficulty: row.get("requested_difficulty"),
        status,
        error,
        content: row.get("content"),
        source_references: serde_json::from_value(row.get("source_references"))
            .map_err(internal)?,
    })
}

fn kind_name(kind: VariantKind) -> &'static str {
    match kind {
        VariantKind::Explanation => "explanation",
        VariantKind::Example => "example",
        VariantKind::Difficulty => "difficulty",
    }
}

fn parse_kind(value: String) -> Result<VariantKind, ApiError> {
    match value.as_str() {
        "explanation" => Ok(VariantKind::Explanation),
        "example" => Ok(VariantKind::Example),
        "difficulty" => Ok(VariantKind::Difficulty),
        _ => Err(internal("invalid variant kind")),
    }
}

fn generation_status(value: String) -> Result<GenerationStatus, ApiError> {
    match value.as_str() {
        "requested" => Ok(GenerationStatus::Requested),
        "running" => Ok(GenerationStatus::Running),
        "review_required" => Ok(GenerationStatus::ReviewRequired),
        "published" => Ok(GenerationStatus::Published),
        "failed" => Ok(GenerationStatus::Failed),
        _ => Err(internal("invalid generation status")),
    }
}

fn validation(field: &str, message: &str) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(error.to_string()))
}
