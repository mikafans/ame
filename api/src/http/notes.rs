//! Private learner note HTTP contract.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{patch, post},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::{ApiError, FieldError},
        note::{CreateNote, NoteError},
    },
    http::AppState,
    note::{Note, NoteRepository},
    note_postgres::PgNoteRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteBody {
    pub journey_id: Uuid,
    pub activity_id: Option<Uuid>,
    pub content_version: Option<i32>,
    pub body: String,
    pub retry_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteBody {
    pub expected_revision: i32,
    pub body: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query, rename_all = "camelCase")]
pub struct NoteQuery {
    pub journey_id: Uuid,
    pub activity_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NoteResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub activity_id: Option<Uuid>,
    pub content_version: Option<i32>,
    pub body: String,
    pub revision: i32,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: time::OffsetDateTime,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/notes", post(create_note).get(list_notes))
        .route("/v1/notes/{id}", patch(update_note).delete(delete_note))
        .with_state(state)
}

#[utoipa::path(post, path = "/api/v1/notes", request_body = CreateNoteBody, responses((status = 200, body = NoteResponse)), security(("bearer" = [])), tag = "notes")]
pub async fn create_note(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<CreateNoteBody>,
) -> Result<Json<NoteResponse>, ApiError> {
    PgNoteRepository::new(state.pool)
        .create(CreateNote {
            subject_user_id: auth.owner_id(),
            journey_id: body.journey_id,
            activity_id: body.activity_id,
            content_version: body.content_version,
            body: body.body,
            retry_key: body.retry_key,
        })
        .await
        .map(note_response)
        .map(Json)
        .map_err(map_error)
}

#[utoipa::path(get, path = "/api/v1/notes", params(NoteQuery), responses((status = 200, body = [NoteResponse])), security(("bearer" = [])), tag = "notes")]
pub async fn list_notes(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<NoteQuery>,
) -> Result<Json<Vec<NoteResponse>>, ApiError> {
    PgNoteRepository::new(state.pool)
        .list(auth.owner_id(), query.journey_id, query.activity_id)
        .await
        .map(|notes| Json(notes.into_iter().map(note_response).collect()))
        .map_err(map_error)
}

#[utoipa::path(patch, path = "/api/v1/notes/{id}", params(("id" = Uuid, Path)), request_body = UpdateNoteBody, responses((status = 200, body = NoteResponse)), security(("bearer" = [])), tag = "notes")]
pub async fn update_note(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateNoteBody>,
) -> Result<Json<NoteResponse>, ApiError> {
    PgNoteRepository::new(state.pool)
        .update(auth.owner_id(), id, body.expected_revision, body.body)
        .await
        .map(note_response)
        .map(Json)
        .map_err(map_error)
}

#[utoipa::path(delete, path = "/api/v1/notes/{id}", params(("id" = Uuid, Path)), responses((status = 204)), security(("bearer" = [])), tag = "notes")]
pub async fn delete_note(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    PgNoteRepository::new(state.pool)
        .delete(auth.owner_id(), id)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_error)
}

fn note_response(note: Note) -> NoteResponse {
    NoteResponse {
        id: note.id,
        journey_id: note.journey_id,
        activity_id: note.activity_id,
        content_version: note.content_version,
        body: note.body,
        revision: note.revision,
        created_at: note.created_at,
        updated_at: note.updated_at,
    }
}

fn map_error(error: NoteError) -> ApiError {
    match error {
        NoteError::NotFound => ApiError::NotFound { resource: "note" },
        NoteError::Storage(message) => ApiError::Internal(anyhow::anyhow!(message)),
        other => ApiError::Validation(vec![FieldError {
            field: "note".into(),
            message: other.to_string(),
        }]),
    }
}
