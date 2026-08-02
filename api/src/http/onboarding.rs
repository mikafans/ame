//! Public self-host onboarding boundary.

use axum::{
    Extension, Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::error::ApiError,
    http::AppState,
    identity_postgres::PgIdentityRepository,
    learning_postgres::PgLearningRepository,
    onboarding::{
        CatalogPromptInterpreter, LearningPreview, SelfHostOnboardingService, StartLearningError,
        StartLearningPromptRequest,
    },
    session::{BrowserSessionService, SessionCredentialError},
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartLearningBody {
    pub email: String,
    pub display_name: String,
    pub prompt: String,
    pub catalog_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLearningBody {
    pub prompt: String,
    pub catalog_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NativeJourneyCatalogResponse {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub description: String,
    pub field: String,
    pub level: String,
    pub estimated_minutes: i64,
    pub outcomes: Vec<String>,
    pub source_summary: String,
    pub sources: Vec<NativeJourneySourceResponse>,
    pub content_review: NativeJourneyContentReviewResponse,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NativeJourneySourceResponse {
    pub title: String,
    pub url: String,
    pub locator: Option<String>,
    pub license: String,
    pub source_version: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NativeJourneyContentReviewResponse {
    pub status: String,
    pub reviewed_at: String,
    pub reviewer: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLearningResponse {
    pub normalized_statement: String,
    pub promise: String,
    pub template_id: String,
    pub template_version: u32,
    pub catalog_id: Option<String>,
    pub catalog_version: Option<u32>,
    pub objectives: Vec<PreviewObjectiveResponse>,
    pub first_activity: PreviewActivityResponse,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewObjectiveResponse {
    pub verb: String,
    pub statement: String,
    pub success_criteria: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewActivityResponse {
    pub kind: crate::domain::learning::ActivityKind,
    pub title: String,
    pub purpose: String,
    pub estimated_minutes: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartLearningResponse {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub goal_id: Uuid,
    pub journey_id: Uuid,
    pub template_id: String,
    pub template_version: u32,
    pub catalog_id: Option<String>,
    pub catalog_version: Option<u32>,
    pub origin: crate::domain::learning::JourneyOrigin,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/catalog/journeys", get(list_native_journeys))
        .route("/v1/onboarding/preview", post(preview_learning))
        .route("/v1/onboarding/start", post(start_learning))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .with_state(state)
}

/// GET /public/v1/catalog/journeys — list reviewed, selectable native journeys.
#[utoipa::path(
    get,
    path = "/public/v1/catalog/journeys",
    responses((status = 200, description = "Reviewed native journeys", body = [NativeJourneyCatalogResponse])),
    tag = "onboarding"
)]
pub async fn list_native_journeys() -> Result<Json<Vec<NativeJourneyCatalogResponse>>, ApiError> {
    let journeys = crate::templates::native_journey_blueprints()
        .map_err(|error| ApiError::Internal(anyhow::anyhow!(error)))?
        .into_iter()
        .filter_map(|topic| {
            topic.catalog.map(|catalog| NativeJourneyCatalogResponse {
                id: topic.id,
                version: catalog.version,
                title: catalog.title,
                description: catalog.description,
                field: topic.field,
                level: catalog.level,
                estimated_minutes: catalog.estimated_minutes,
                outcomes: catalog.outcomes,
                source_summary: catalog.source_summary,
                sources: catalog
                    .sources
                    .into_iter()
                    .map(|source| NativeJourneySourceResponse {
                        title: source.title,
                        url: source.url,
                        locator: source.locator,
                        license: source.license,
                        source_version: source.source_version,
                    })
                    .collect(),
                content_review: NativeJourneyContentReviewResponse {
                    status: catalog.content_review.status,
                    reviewed_at: catalog.content_review.reviewed_at,
                    reviewer: catalog.content_review.reviewer,
                },
            })
        })
        .collect();
    Ok(Json(journeys))
}

/// POST /public/v1/onboarding/preview — explain the first journey without creating state.
#[utoipa::path(
    post,
    path = "/public/v1/onboarding/preview",
    request_body = PreviewLearningBody,
    responses(
        (status = 200, description = "Preview of the first learning journey", body = PreviewLearningResponse),
        (status = 422, description = "Validation failed")
    ),
    tag = "onboarding"
)]
pub async fn preview_learning(
    State(state): State<AppState>,
    Json(body): Json<PreviewLearningBody>,
) -> Result<Json<PreviewLearningResponse>, ApiError> {
    let identity = PgIdentityRepository::new(state.pool.clone());
    let onboarding =
        SelfHostOnboardingService::new(identity, PgLearningRepository::new(state.pool.clone()));
    let preview = match body.catalog_id.as_deref() {
        Some(catalog_id) => onboarding.preview_from_catalog_entry(catalog_id).await,
        None => {
            onboarding
                .preview_from_prompt(&body.prompt, &CatalogPromptInterpreter)
                .await
        }
    }
    .map_err(map_preview_error)?;
    Ok(Json(preview_response(preview)))
}

/// POST /public/v1/onboarding/start — create or resume a learner's first journey.
#[utoipa::path(
    post,
    path = "/public/v1/onboarding/start",
    request_body = StartLearningBody,
    responses(
        (status = 201, description = "Learner and first journey created", body = StartLearningResponse),
        (status = 401, description = "Existing learner requires authentication"),
        (status = 422, description = "Validation failed; an existing learner must supply a valid bearer token")
    ),
    tag = "onboarding"
)]
pub async fn start_learning(
    State(state): State<AppState>,
    authenticated: Option<Extension<AuthenticatedUser>>,
    Json(body): Json<StartLearningBody>,
) -> Result<
    (
        StatusCode,
        [(String, String); 1],
        Json<StartLearningResponse>,
    ),
    ApiError,
> {
    let identity = PgIdentityRepository::new(state.pool.clone());
    let onboarding = SelfHostOnboardingService::new(
        identity.clone(),
        PgLearningRepository::new(state.pool.clone()),
    );
    let auth_user_id = authenticated.map(|Extension(user)| user.owner_id());
    let request = StartLearningPromptRequest {
        authenticated_user_id: auth_user_id,
        email: body.email,
        display_name: body.display_name,
        raw_prompt: body.prompt,
        idempotency_key: body.idempotency_key,
        registration_mode: state.config.registration.mode,
    };
    let started = match body.catalog_id.as_deref() {
        Some(catalog_id) => {
            onboarding
                .start_from_catalog_entry(request, catalog_id)
                .await
        }
        None => {
            onboarding
                .start_from_prompt(request, &CatalogPromptInterpreter)
                .await
        }
    }
    .map_err(map_start_learning_error)?;

    let issued = BrowserSessionService::new(identity)
        .issue(
            started.account.id,
            Duration::seconds(state.config.login.ttl_seconds as i64),
        )
        .await
        .map_err(map_session_error)?;

    let token = issued.token;
    let cookie = format_session_cookie(&token, state.config.server.production);

    Ok((
        StatusCode::CREATED,
        [("set-cookie".to_string(), cookie)],
        Json(StartLearningResponse {
            token,
            user_id: started.account.id,
            email: started.account.email,
            display_name: started.account.display_name,
            goal_id: started.bootstrap.goal.id,
            journey_id: started.bootstrap.journey.id,
            template_id: started.bootstrap.template_id,
            template_version: started.bootstrap.template_version,
            catalog_id: started.bootstrap.goal.catalog_entry_id,
            catalog_version: started.bootstrap.goal.catalog_entry_version,
            origin: started.bootstrap.journey.origin,
        }),
    ))
}

fn format_session_cookie(token: &str, secure: bool) -> String {
    let secure_suffix = if secure { "; Secure" } else { "" };
    format!("ame_token={token}; HttpOnly{secure_suffix}; SameSite=Lax; Path=/; Max-Age=2592000")
}

fn map_start_learning_error(error: StartLearningError) -> ApiError {
    match error {
        StartLearningError::Identity(
            crate::domain::identity::IdentityRepositoryError::AccountAlreadyExists,
        ) => ApiError::Validation(vec![crate::domain::error::FieldError {
            field: "email".into(),
            message: "email already belongs to a learner; authenticate to resume".into(),
        }]),
        StartLearningError::Identity(
            crate::domain::identity::IdentityRepositoryError::AccountEmailMismatch,
        ) => ApiError::Unauthorized,
        StartLearningError::Identity(error) => ApiError::Internal(error.into()),
        StartLearningError::Prompt(crate::onboarding::PromptInterpretationError::EmptyPrompt) => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "prompt".into(),
                message: "must not be empty".into(),
            }])
        }
        StartLearningError::Prompt(
            crate::onboarding::PromptInterpretationError::UnknownCatalogEntry(id),
        ) => ApiError::Validation(vec![crate::domain::error::FieldError {
            field: "catalogId".into(),
            message: format!("native journey catalog entry is unavailable: {id}"),
        }]),
        StartLearningError::Prompt(error) => ApiError::Internal(error.into()),
        StartLearningError::AuthenticationRequired => ApiError::Unauthorized,
        StartLearningError::Bootstrap(error) => ApiError::Internal(error.into()),
    }
}

fn map_session_error(error: SessionCredentialError) -> ApiError {
    ApiError::Internal(error.into())
}

fn map_preview_error(error: crate::onboarding::PromptInterpretationError) -> ApiError {
    match error {
        crate::onboarding::PromptInterpretationError::EmptyPrompt => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "prompt".into(),
                message: "must not be empty".into(),
            }])
        }
        crate::onboarding::PromptInterpretationError::UnknownCatalogEntry(id) => {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "catalogId".into(),
                message: format!("native journey catalog entry is unavailable: {id}"),
            }])
        }
        error => ApiError::Internal(error.into()),
    }
}

fn preview_response(preview: LearningPreview) -> PreviewLearningResponse {
    PreviewLearningResponse {
        normalized_statement: preview.interpretation.normalized_statement,
        promise: preview.interpretation.promise,
        template_id: preview.interpretation.template_id,
        template_version: preview.interpretation.template_version,
        catalog_id: preview.interpretation.catalog_entry_id,
        catalog_version: preview.interpretation.catalog_entry_version,
        objectives: preview
            .objectives
            .into_iter()
            .map(|objective| PreviewObjectiveResponse {
                verb: objective.verb,
                statement: objective.statement,
                success_criteria: objective.success_criteria,
            })
            .collect(),
        first_activity: PreviewActivityResponse {
            kind: preview.first_activity.kind,
            title: preview.first_activity.title,
            purpose: preview.first_activity.purpose,
            estimated_minutes: preview.first_activity.estimated_minutes,
        },
    }
}
