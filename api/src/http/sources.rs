//! Learner-owned immutable source imports and snapshots.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::{ApiError, FieldError},
        source::{
            ImportSource, MAX_SOURCE_BYTES, SourceError, SourceImportStatus, SourceKind, public_ip,
            safe_https_locator,
        },
    },
    http::AppState,
    source::{SourceImportRun, SourceRepository, SourceSnapshot},
    source_postgres::PgSourceRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceBody {
    pub kind: SourceKind,
    pub locator: String,
    pub media_type: Option<String>,
    pub content: Option<String>,
    pub retry_key: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceSnapshotResponse {
    pub id: Uuid,
    pub source_id: Uuid,
    pub import_run_id: Uuid,
    pub media_type: String,
    pub content_sha256: String,
    pub byte_length: u32,
    pub content: String,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceImportRunResponse {
    pub id: Uuid,
    pub source_id: Uuid,
    pub retry_key: String,
    pub status: SourceImportStatus,
    pub error_code: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    #[schema(value_type = Option<String>, format = DateTime)]
    pub completed_at: Option<time::OffsetDateTime>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/sources/imports", post(import))
        .route("/v1/source-imports", get(list_imports))
        .route("/v1/source-snapshots", get(list))
        .route("/v1/source-snapshots/{snapshot_id}", get(get_one))
        .with_state(state)
}

#[utoipa::path(post, path = "/api/v1/sources/imports", request_body = ImportSourceBody, responses((status = 200, body = SourceSnapshotResponse)), security(("bearer" = [])), tag = "sources")]
pub async fn import(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<ImportSourceBody>,
) -> Result<Json<SourceSnapshotResponse>, ApiError> {
    let owner_id = auth.owner_id();
    let ImportSourceBody {
        kind,
        locator,
        media_type,
        content,
        retry_key,
    } = body;
    let fetched = match kind {
        SourceKind::Url => fetch_url(&locator).await,
        SourceKind::Document | SourceKind::LocalFile => content
            .ok_or_else(|| validation("content", "is required"))
            .map(|content| {
                (
                    media_type.unwrap_or_else(|| "text/plain".into()),
                    content.into_bytes(),
                )
            }),
    };
    let (media_type, content) = match fetched {
        Ok(value) => value,
        Err(error) => {
            PgSourceRepository::new(state.pool)
                .record_failure(
                    owner_id,
                    kind,
                    locator,
                    retry_key,
                    import_error_code(&error).into(),
                )
                .await
                .map_err(map_error)?;
            return Err(error);
        }
    };
    let (_, _, snapshot) = PgSourceRepository::new(state.pool)
        .import(ImportSource {
            subject_user_id: owner_id,
            source_kind: kind,
            locator,
            media_type,
            content,
            retry_key,
        })
        .await
        .map_err(map_error)?;
    Ok(Json(response(snapshot)?))
}

#[utoipa::path(get, path = "/api/v1/source-imports", responses((status = 200, body = [SourceImportRunResponse])), security(("bearer" = [])), tag = "sources")]
pub async fn list_imports(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<SourceImportRunResponse>>, ApiError> {
    PgSourceRepository::new(state.pool)
        .list_imports(auth.owner_id())
        .await
        .map_err(map_error)
        .map(|runs| Json(runs.into_iter().map(import_response).collect()))
}

#[utoipa::path(get, path = "/api/v1/source-snapshots", responses((status = 200, body = [SourceSnapshotResponse])), security(("bearer" = [])), tag = "sources")]
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<SourceSnapshotResponse>>, ApiError> {
    let snapshots = PgSourceRepository::new(state.pool)
        .list_snapshots(auth.owner_id())
        .await
        .map_err(map_error)?;
    snapshots
        .into_iter()
        .map(response)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

#[utoipa::path(get, path = "/api/v1/source-snapshots/{snapshot_id}", params(("snapshot_id" = Uuid, Path)), responses((status = 200, body = SourceSnapshotResponse)), security(("bearer" = [])), tag = "sources")]
pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(snapshot_id): Path<Uuid>,
) -> Result<Json<SourceSnapshotResponse>, ApiError> {
    PgSourceRepository::new(state.pool)
        .get_snapshot(auth.owner_id(), snapshot_id)
        .await
        .map_err(map_error)
        .and_then(response)
        .map(Json)
}

async fn fetch_url(locator: &str) -> Result<(String, Vec<u8>), ApiError> {
    if !safe_https_locator(locator) {
        return Err(validation("locator", "must be a public HTTPS URL"));
    }
    let url = reqwest::Url::parse(locator)
        .map_err(|_| validation("locator", "must be a public HTTPS URL"))?;
    let host = url
        .host_str()
        .ok_or_else(|| validation("locator", "must include a host"))?;
    let addresses = tokio::net::lookup_host((host, 443))
        .await
        .map_err(|error| ApiError::Internal(anyhow::anyhow!(error)))?
        .collect::<Vec<_>>();
    if addresses.is_empty() || addresses.iter().any(|address| !public_ip(address.ip())) {
        return Err(validation("locator", "host resolves to a private address"));
    }
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(10))
        .resolve(host, SocketAddr::new(addresses[0].ip(), 443))
        .build()
        .map_err(|error| ApiError::Internal(anyhow::anyhow!(error)))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| ApiError::Internal(anyhow::anyhow!(error)))?;
    if !response.status().is_success() {
        return Err(validation(
            "locator",
            "source did not return a successful response",
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_SOURCE_BYTES as u64)
    {
        return Err(validation("content", "exceeds the two MiB limit"));
    }
    let media_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .unwrap_or("text/plain")
        .to_string();
    let content = response
        .bytes()
        .await
        .map_err(|error| ApiError::Internal(anyhow::anyhow!(error)))?;
    if content.len() > MAX_SOURCE_BYTES {
        return Err(validation("content", "exceeds the two MiB limit"));
    }
    Ok((media_type, content.to_vec()))
}

fn response(snapshot: SourceSnapshot) -> Result<SourceSnapshotResponse, ApiError> {
    let content = String::from_utf8(snapshot.content)
        .map_err(|_| validation("content", "must be UTF-8 text"))?;
    Ok(SourceSnapshotResponse {
        id: snapshot.id,
        source_id: snapshot.source_id,
        import_run_id: snapshot.import_run_id,
        media_type: snapshot.media_type,
        content_sha256: snapshot.content_sha256,
        byte_length: snapshot.byte_length,
        content,
        created_at: snapshot.created_at,
    })
}

fn import_response(run: SourceImportRun) -> SourceImportRunResponse {
    SourceImportRunResponse {
        id: run.id,
        source_id: run.source_id,
        retry_key: run.retry_key,
        status: run.status,
        error_code: run.error_code,
        created_at: run.created_at,
        completed_at: run.completed_at,
    }
}

fn import_error_code(error: &ApiError) -> &'static str {
    match error {
        ApiError::Validation(_) => "source_validation_failed",
        _ => "source_fetch_failed",
    }
}

fn map_error(error: SourceError) -> ApiError {
    match error {
        SourceError::NotFound | SourceError::SubjectMismatch => ApiError::NotFound {
            resource: "source snapshot",
        },
        SourceError::Storage(message) => ApiError::Internal(anyhow::anyhow!(message)),
        other => validation("source", &other.to_string()),
    }
}

fn validation(field: &str, message: &str) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}
