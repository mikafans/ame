//! Study plan HTTP routes.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::post};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    auth::scope::{RequireAnyScope, ScopeOneOf},
    domain::{error::ApiError, user::Scope},
    engine::planner::{DEFAULT_LOOKBACK_DAYS, StudyPlan, create_study_plan},
    http::AppState,
};

pub struct PlanWriteScopes;
impl ScopeOneOf for PlanWriteScopes {
    const SCOPES: &'static [Scope] = &[Scope::Human];
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanBody {
    pub goal: String,
    #[serde(default)]
    pub lookback_days: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/v1/plans",
    request_body = CreatePlanBody,
    responses(
        (status = 201, description = "Study plan created", body = StudyPlan),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Token lacks required scope"),
        (status = 422, description = "Validation failed"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_plan(
    State(state): State<AppState>,
    user: RequireAnyScope<PlanWriteScopes>,
    Json(body): Json<CreatePlanBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.goal.trim().is_empty() {
        return Err(ApiError::Validation(vec![
            crate::domain::error::FieldError {
                field: "goal".to_string(),
                message: "goal must not be empty".to_string(),
            },
        ]));
    }
    let lookback_days = body.lookback_days.unwrap_or(DEFAULT_LOOKBACK_DAYS);
    let plan = create_study_plan(&state.pool, user.0.user.id, &body.goal, lookback_days).await?;
    Ok((StatusCode::CREATED, Json(plan)))
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/plans", post(create_plan))
        .with_state(state)
}
