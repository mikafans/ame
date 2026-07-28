//! Learner-visible citation certificates anchored to immutable snapshots.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    citation::{Citation, CitationRepository},
    citation_postgres::PgCitationRepository,
    domain::{
        citation::{
            CitationError, CreateCitation, ExtractionMethod, GroundingStatus, LicenseStatus,
        },
        error::{ApiError, FieldError},
    },
    http::AppState,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCitationBody {
    pub snapshot_id: Uuid,
    pub start_byte: u32,
    pub end_byte: u32,
    pub quote: String,
    pub extraction_method: ExtractionMethod,
    pub grounding_status: GroundingStatus,
    pub grounding_note: String,
    pub license_status: LicenseStatus,
    pub license_name: Option<String>,
    pub license_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CitationResponse {
    pub id: Uuid,
    pub snapshot_id: Uuid,
    pub start_byte: u32,
    pub end_byte: u32,
    pub quote: String,
    pub extraction_method: ExtractionMethod,
    pub grounding_status: GroundingStatus,
    pub grounding_note: String,
    pub license_status: LicenseStatus,
    pub license_name: Option<String>,
    pub license_url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/citations", get(list_citations).post(create_citation))
        .route("/v1/citations/{id}", get(get_citation))
        .with_state(state)
}

#[utoipa::path(post, path = "/api/v1/citations", request_body = CreateCitationBody, responses((status = 200, body = CitationResponse)), security(("bearer" = [])), tag = "citations")]
pub async fn create_citation(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateCitationBody>,
) -> Result<Json<CitationResponse>, ApiError> {
    PgCitationRepository::new(state.pool)
        .create(CreateCitation {
            subject_user_id: auth.owner_id(),
            snapshot_id: body.snapshot_id,
            start_byte: body.start_byte,
            end_byte: body.end_byte,
            quote: body.quote,
            extraction_method: body.extraction_method,
            grounding_status: body.grounding_status,
            grounding_note: body.grounding_note,
            license_status: body.license_status,
            license_name: body.license_name,
            license_url: body.license_url,
        })
        .await
        .map(response)
        .map(Json)
        .map_err(map_error)
}

#[utoipa::path(get, path = "/api/v1/citations", responses((status = 200, body = [CitationResponse])), security(("bearer" = [])), tag = "citations")]
pub async fn list_citations(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<CitationResponse>>, ApiError> {
    PgCitationRepository::new(state.pool)
        .list(auth.owner_id())
        .await
        .map(|values| Json(values.into_iter().map(response).collect()))
        .map_err(map_error)
}

#[utoipa::path(get, path = "/api/v1/citations/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = CitationResponse)), security(("bearer" = [])), tag = "citations")]
pub async fn get_citation(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CitationResponse>, ApiError> {
    PgCitationRepository::new(state.pool)
        .get(auth.owner_id(), id)
        .await
        .map(response)
        .map(Json)
        .map_err(map_error)
}

pub async fn certify_references(
    pool: PgPool,
    subject_user_id: Uuid,
    references: &[String],
) -> Result<(), ApiError> {
    let ids = references
        .iter()
        .map(|reference| {
            reference.parse::<Uuid>().map_err(|_| {
                validation("sourceReferences", "must contain citation certificate IDs")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    PgCitationRepository::new(pool)
        .certify_publication(subject_user_id, &ids)
        .await
        .map_err(map_error)
}

fn response(value: Citation) -> CitationResponse {
    CitationResponse {
        id: value.id,
        snapshot_id: value.snapshot_id,
        start_byte: value.start_byte,
        end_byte: value.end_byte,
        quote: value.quote,
        extraction_method: value.extraction_method,
        grounding_status: value.grounding_status,
        grounding_note: value.grounding_note,
        license_status: value.license_status,
        license_name: value.license_name,
        license_url: value.license_url,
        created_at: value.created_at,
    }
}

fn map_error(error: CitationError) -> ApiError {
    match error {
        CitationError::NotFound => ApiError::NotFound {
            resource: "citation",
        },
        CitationError::Storage(message) => ApiError::Internal(anyhow::anyhow!(message)),
        other => validation("citation", &other.to_string()),
    }
}

fn validation(field: &str, message: &str) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}
