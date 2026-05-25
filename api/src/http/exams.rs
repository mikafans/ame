//! Exam composition and retrieval HTTP routes.

use std::str::FromStr;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireAnyScope, ScopeOneOf},
    },
    domain::{
        error::{ApiError, FieldError},
        exam::{Exam, ExamMethod, ExamSection, ExamStatus},
        user::Scope,
    },
    http::{AppState, idempotency::idempotency_middleware},
};

pub struct ExamWriteScopes;
impl ScopeOneOf for ExamWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::QuizWrite];
}

// ── request / response shapes ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SectionSpec {
    pub title: String,
    pub weight: f64,
    /// Static: explicit question ids from the bank.
    #[serde(default)]
    pub question_ids: Vec<Uuid>,
    /// Dynamic: draw `items` questions matching these filters.
    pub items: Option<i32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub types: Vec<String>,
    pub difficulty_min: Option<f64>,
    pub difficulty_max: Option<f64>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeExamBody {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    pub duration: Option<i32>,
    pub passing_points: Option<i32>,
    #[serde(default)]
    pub objectives: Vec<String>,
    #[serde(default)]
    pub affects_rating: Option<bool>,
    pub sections: Vec<SectionSpec>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeExamResponse {
    pub exam_id: Uuid,
    pub total_points: i32,
    pub sections: Vec<ExamSection>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetExamResponse {
    pub exam: Exam,
    pub sections: Vec<ExamSection>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListExamsResponse {
    pub exams: Vec<Exam>,
}

// ── handlers ─────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/exams",
    request_body = ComposeExamBody,
    responses(
        (status = 201, description = "Exam composed", body = ComposeExamResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 422, description = "Validation failed or pool insufficient"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn compose_exam(
    State(state): State<AppState>,
    user: RequireAnyScope<ExamWriteScopes>,
    Json(body): Json<ComposeExamBody>,
) -> Result<impl IntoResponse, ApiError> {
    validate_compose_body(&body)?;

    let method = body
        .method
        .as_deref()
        .unwrap_or("manual")
        .parse::<ExamMethod>()
        .map_err(|e| {
            ApiError::Validation(vec![FieldError {
                field: "method".into(),
                message: e,
            }])
        })?;

    let affects_rating = body.affects_rating.unwrap_or(true);
    let blueprint = serde_json::to_value(&body.sections).map_err(anyhow::Error::from)?;

    // Resolve each section
    let mut resolved: Vec<ExamSection> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut total_points: i32 = 0;
    let mut trace_sections: Vec<serde_json::Value> = Vec::new();

    for (idx, spec) in body.sections.iter().enumerate() {
        let (section, section_points, trace) =
            resolve_section(&state.pool, spec, idx as i32).await?;
        total_points += section_points;
        trace_sections.push(trace);
        if section.items_count < spec.question_ids.len() as i32 + spec.items.unwrap_or(0) {
            warnings.push(format!(
                "section '{}' planned {} items (pool short)",
                spec.title, section.items_count
            ));
        }
        resolved.push(section);
    }

    let composition_trace = if method == ExamMethod::Agent {
        Some(serde_json::json!({ "tool": "exam.compose", "sections": trace_sections }))
    } else {
        None
    };

    let exam_id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();
    sqlx::query(
        "INSERT INTO tb_exams \
         (id, name, description, blueprint, method, duration_min, total_points, passing_points, \
          objectives, affects_rating, show_results_during, status, composition_trace, \
          created_by, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'draft',$12,$13,$14,$14)",
    )
    .bind(exam_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&blueprint)
    .bind(method.as_str())
    .bind(body.duration)
    .bind(total_points)
    .bind(body.passing_points)
    .bind(&body.objectives)
    .bind(affects_rating)
    .bind(false)
    .bind(composition_trace)
    .bind(user.0.user.id)
    .bind(now)
    .execute(&state.pool)
    .await
    .map_err(internal)?;

    for section in &resolved {
        insert_section(&state.pool, exam_id, section).await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(ComposeExamResponse {
            exam_id,
            total_points,
            sections: resolved,
            warnings,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/exams",
    responses(
        (status = 200, description = "List of exams", body = ListExamsResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_exams(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<ListExamsResponse>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, name, description, method, status, blueprint, duration_min, total_points, \
                passing_points, objectives, affects_rating, show_results_during, \
                composition_trace, created_by, created_at, updated_at \
         FROM tb_exams WHERE status != 'archived' ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(internal)?;

    let exams = rows
        .into_iter()
        .map(row_to_exam)
        .collect::<Result<_, _>>()?;
    Ok(Json(ListExamsResponse { exams }))
}

#[utoipa::path(
    get,
    path = "/v1/exams/{id}",
    params(("id" = Uuid, Path, description = "Exam id")),
    responses(
        (status = 200, description = "Exam with sections", body = GetExamResponse),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_exam(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<GetExamResponse>, ApiError> {
    let row = sqlx::query(
        "SELECT id, name, description, method, status, blueprint, duration_min, total_points, \
                passing_points, objectives, affects_rating, show_results_during, \
                composition_trace, created_by, created_at, updated_at \
         FROM tb_exams WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound { resource: "exam" })?;

    let mut exam = row_to_exam(row)?;

    // Learners never see composition_trace
    let is_writer = user.token_scopes.contains(&Scope::QuizWrite);
    if !is_writer {
        exam.composition_trace = None;
    }

    let sections = load_sections(&state.pool, id).await?;
    Ok(Json(GetExamResponse { exam, sections }))
}

// ── section resolution ────────────────────────────────────────────────────────

async fn resolve_section(
    pool: &PgPool,
    spec: &SectionSpec,
    order_index: i32,
) -> Result<(ExamSection, i32, serde_json::Value), ApiError> {
    let is_static = !spec.question_ids.is_empty();

    if is_static {
        resolve_static_section(pool, spec, order_index).await
    } else {
        resolve_dynamic_section(pool, spec, order_index).await
    }
}

async fn resolve_static_section(
    pool: &PgPool,
    spec: &SectionSpec,
    order_index: i32,
) -> Result<(ExamSection, i32, serde_json::Value), ApiError> {
    let ids = &spec.question_ids;

    // Verify all question ids exist and are live
    let rows = sqlx::query(
        "SELECT id, points FROM tb_questions WHERE id = ANY($1::uuid[]) AND status = 'live'",
    )
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    if rows.len() < ids.len() {
        let found: Vec<Uuid> = rows.iter().map(|r| r.get("id")).collect();
        let missing: Vec<&Uuid> = ids.iter().filter(|id| !found.contains(id)).collect();
        return Err(ApiError::Validation(vec![FieldError {
            field: format!("sections[{}].questionIds", order_index),
            message: format!(
                "{} question(s) not found or not live: {}",
                missing.len(),
                missing
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }]));
    }

    let points: i32 = rows.iter().map(|r| r.get::<i32, _>("points")).sum();
    let section = ExamSection {
        id: Uuid::now_v7(),
        exam_id: Uuid::nil(), // filled on insert
        title: spec.title.clone(),
        order_index,
        weight: spec.weight,
        question_ids: Some(ids.clone()),
        mix: None,
        items_count: ids.len() as i32,
    };
    let trace = serde_json::json!({
        "strategy": "static",
        "question_ids": ids,
        "points": points,
    });
    Ok((section, points, trace))
}

async fn resolve_dynamic_section(
    pool: &PgPool,
    spec: &SectionSpec,
    order_index: i32,
) -> Result<(ExamSection, i32, serde_json::Value), ApiError> {
    let items = spec.items.unwrap_or(10);
    if items <= 0 {
        return Err(ApiError::Validation(vec![FieldError {
            field: format!("sections[{}].items", order_index),
            message: "items must be > 0 for dynamic sections".into(),
        }]));
    }

    let tags: Vec<String> = spec.tags.iter().map(|t| t.trim().to_lowercase()).collect();
    let types = &spec.types;

    let rows = sqlx::query(
        "SELECT q.id, q.points FROM tb_questions q \
         WHERE q.status = 'live' \
           AND (cardinality($1::text[]) = 0 OR q.kind = ANY($1::text[])) \
           AND ($2::double precision IS NULL OR q.rating >= $2) \
           AND ($3::double precision IS NULL OR q.rating <= $3) \
           AND (cardinality($4::text[]) = 0 OR EXISTS ( \
                SELECT 1 FROM tb_question_tags qt JOIN tb_tags t ON t.id = qt.tag_id \
                WHERE qt.question_id = q.id AND t.name = ANY($4::text[]) \
           )) \
         ORDER BY random() LIMIT $5",
    )
    .bind(types)
    .bind(spec.difficulty_min)
    .bind(spec.difficulty_max)
    .bind(&tags)
    .bind(items)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    let available = rows.len();
    if (available as i32) < items {
        return Err(ApiError::ExamPoolInsufficient {
            section: spec.title.clone(),
            required: items as usize,
            available,
        });
    }

    let question_ids: Vec<Uuid> = rows.iter().map(|r| r.get("id")).collect();
    let points: i32 = rows.iter().map(|r| r.get::<i32, _>("points")).sum();
    let mix = serde_json::json!({
        "tags": tags,
        "types": types,
        "difficulty_min": spec.difficulty_min,
        "difficulty_max": spec.difficulty_max,
    });
    let section = ExamSection {
        id: Uuid::now_v7(),
        exam_id: Uuid::nil(),
        title: spec.title.clone(),
        order_index,
        weight: spec.weight,
        question_ids: Some(question_ids.clone()),
        mix: Some(mix.clone()),
        items_count: available as i32,
    };
    let trace = serde_json::json!({
        "strategy": "dynamic",
        "mix": mix,
        "question_ids": question_ids,
        "points": points,
    });
    Ok((section, points, trace))
}

// ── DB helpers ────────────────────────────────────────────────────────────────

async fn insert_section(
    pool: &PgPool,
    exam_id: Uuid,
    section: &ExamSection,
) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO tb_exam_sections \
         (id, exam_id, title, order_index, weight, question_ids, mix, items_count) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(section.id)
    .bind(exam_id)
    .bind(&section.title)
    .bind(section.order_index)
    .bind(section.weight)
    .bind(section.question_ids.as_deref())
    .bind(&section.mix)
    .bind(section.items_count)
    .execute(pool)
    .await
    .map_err(internal)?;
    Ok(())
}

async fn load_sections(pool: &PgPool, exam_id: Uuid) -> Result<Vec<ExamSection>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, exam_id, title, order_index, weight, question_ids, mix, items_count \
         FROM tb_exam_sections WHERE exam_id = $1 ORDER BY order_index ASC",
    )
    .bind(exam_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    rows.into_iter()
        .map(|row| {
            Ok(ExamSection {
                id: row.get("id"),
                exam_id: row.get("exam_id"),
                title: row.get("title"),
                order_index: row.get("order_index"),
                weight: row.get("weight"),
                question_ids: row.get("question_ids"),
                mix: row.get("mix"),
                items_count: row.get("items_count"),
            })
        })
        .collect()
}

fn row_to_exam(row: sqlx::postgres::PgRow) -> Result<Exam, ApiError> {
    let method_str: String = row.get("method");
    let status_str: String = row.get("status");
    Ok(Exam {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        method: ExamMethod::from_str(&method_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        status: ExamStatus::from_str(&status_str)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?,
        blueprint: row.get("blueprint"),
        duration_min: row.get("duration_min"),
        total_points: row.get("total_points"),
        passing_points: row.get("passing_points"),
        objectives: row.get("objectives"),
        affects_rating: row.get("affects_rating"),
        show_results_during: row.get("show_results_during"),
        composition_trace: row.get("composition_trace"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn validate_compose_body(body: &ComposeExamBody) -> Result<(), ApiError> {
    let mut fields = Vec::new();
    if body.name.trim().is_empty() {
        fields.push(FieldError {
            field: "name".into(),
            message: "must not be empty".into(),
        });
    }
    if body.sections.is_empty() {
        fields.push(FieldError {
            field: "sections".into(),
            message: "must include at least one section".into(),
        });
    }
    for (i, s) in body.sections.iter().enumerate() {
        if s.title.trim().is_empty() {
            fields.push(FieldError {
                field: format!("sections[{i}].title"),
                message: "must not be empty".into(),
            });
        }
        if s.weight <= 0.0 {
            fields.push(FieldError {
                field: format!("sections[{i}].weight"),
                message: "must be > 0".into(),
            });
        }
        let is_static = !s.question_ids.is_empty();
        let is_dynamic = s.items.is_some();
        if is_static && is_dynamic {
            fields.push(FieldError {
                field: format!("sections[{i}]"),
                message: "provide either questionIds (static) or items (dynamic), not both".into(),
            });
        }
        if !is_static && !is_dynamic {
            fields.push(FieldError {
                field: format!("sections[{i}]"),
                message: "provide either questionIds (static) or items (dynamic)".into(),
            });
        }
    }
    if fields.is_empty() {
        Ok(())
    } else {
        Err(ApiError::Validation(fields))
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchExamStatusBody {
    pub status: ExamStatus,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PatchExamStatusResponse {
    pub id: Uuid,
    pub status: String,
}

#[utoipa::path(
    patch,
    path = "/v1/exams/{id}",
    params(("id" = Uuid, Path, description = "Exam id")),
    request_body = PatchExamStatusBody,
    responses(
        (status = 200, description = "Exam status updated", body = PatchExamStatusResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 404, description = "Exam not found"),
        (status = 422, description = "Invalid status value"),
    ),
    security(("bearer_auth" = []))
)]
async fn patch_exam_status(
    State(state): State<AppState>,
    _user: RequireAnyScope<ExamWriteScopes>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchExamStatusBody>,
) -> Result<impl IntoResponse, ApiError> {
    let status = body.status.as_str();

    let result = sqlx::query("UPDATE tb_exams SET status = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(status)
        .execute(&state.pool)
        .await
        .map_err(internal)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound { resource: "exam" });
    }

    Ok((
        StatusCode::OK,
        Json(PatchExamStatusResponse {
            id,
            status: status.to_string(),
        }),
    ))
}

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

pub fn router(state: AppState) -> Router<AppState> {
    let write_router = Router::new()
        .route("/v1/exams", post(compose_exam))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            idempotency_middleware,
        ));
    Router::new()
        .merge(write_router)
        .route("/v1/exams", get(list_exams))
        .route("/v1/exams/{id}", get(get_exam))
        .route("/v1/exams/{id}", patch(patch_exam_status))
        .with_state(state)
}
