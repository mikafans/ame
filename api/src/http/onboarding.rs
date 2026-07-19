//! Public self-host onboarding boundary.

use axum::{Extension, Json, Router, extract::State, http::StatusCode, routing::post};
use serde::{Deserialize, Serialize};
use time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{error::ApiError, identity::RegistrationMode},
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
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLearningBody {
    pub prompt: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLearningResponse {
    pub normalized_statement: String,
    pub promise: String,
    pub template_id: String,
    pub template_version: u32,
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
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/onboarding/preview", post(preview_learning))
        .route("/v1/onboarding/start", post(start_learning))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .with_state(state)
}

/// POST /v1/onboarding/preview — explain the first journey without creating state.
#[utoipa::path(
    post,
    path = "/v1/onboarding/preview",
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
    let preview = onboarding
        .preview_from_prompt(&body.prompt, &CatalogPromptInterpreter)
        .await
        .map_err(map_preview_error)?;
    Ok(Json(preview_response(preview)))
}

/// POST /v1/onboarding/start — create or resume a learner's first journey.
#[utoipa::path(
    post,
    path = "/v1/onboarding/start",
    request_body = StartLearningBody,
    responses(
        (status = 201, description = "Learner and first journey created", body = StartLearningResponse),
        (status = 401, description = "Existing learner requires authentication"),
        (status = 409, description = "Email already belongs to a learner"),
        (status = 422, description = "Validation failed")
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
    let started = onboarding
        .start_from_prompt(
            StartLearningPromptRequest {
                authenticated_user_id: auth_user_id,
                email: body.email,
                display_name: body.display_name,
                raw_prompt: body.prompt,
                idempotency_key: body.idempotency_key,
                registration_mode: RegistrationMode::Open,
            },
            &CatalogPromptInterpreter,
        )
        .await
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
        StartLearningError::Prompt(error) => ApiError::Internal(error.into()),
        StartLearningError::UnsupportedRegistrationMode => ApiError::Internal(anyhow::anyhow!(
            "unsupported registration mode reached public onboarding route"
        )),
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
        error => ApiError::Internal(error.into()),
    }
}

fn preview_response(preview: LearningPreview) -> PreviewLearningResponse {
    PreviewLearningResponse {
        normalized_statement: preview.interpretation.normalized_statement,
        promise: preview.interpretation.promise,
        template_id: preview.interpretation.template_id,
        template_version: preview.interpretation.template_version,
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
