//! Learner-owned spaced-review resources.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{
        error::{ApiError, FieldError},
        review::{ReviewRating, ReviewScheduleError},
    },
    http::AppState,
    review::{RateReview, ReviewItem, ReviewRepository},
    review_postgres::PgReviewRepository,
};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeedReviewBody {
    pub evidence_id: Uuid,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct DueReviewQuery {
    #[serde(default)]
    pub due_before: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateReviewBody {
    pub rating: ReviewRating,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItemResponse {
    pub id: Uuid,
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub activity_id: Uuid,
    pub content_version: u32,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub due_at: OffsetDateTime,
    pub interval_days: u32,
    pub review_count: u32,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/reviews", post(seed))
        .route("/v1/reviews/due", get(list_due))
        .route("/v1/reviews/{review_item_id}/ratings", post(rate))
        .with_state(state)
}

#[utoipa::path(post, path = "/api/v1/reviews", request_body = SeedReviewBody, responses((status = 200, body = ReviewItemResponse)), security(("bearer" = [])), tag = "reviews")]
pub async fn seed(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<SeedReviewBody>,
) -> Result<Json<ReviewItemResponse>, ApiError> {
    PgReviewRepository::new(state.pool)
        .seed_from_evidence(auth.owner_id(), body.evidence_id)
        .await
        .map(response)
        .map(Json)
        .map_err(map_error)
}

#[utoipa::path(get, path = "/api/v1/reviews/due", params(DueReviewQuery), responses((status = 200, body = [ReviewItemResponse])), security(("bearer" = [])), tag = "reviews")]
pub async fn list_due(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<DueReviewQuery>,
) -> Result<Json<Vec<ReviewItemResponse>>, ApiError> {
    PgReviewRepository::new(state.pool)
        .list_due(auth.owner_id(), query.due_before)
        .await
        .map(|items| Json(items.into_iter().map(response).collect()))
        .map_err(map_error)
}

#[utoipa::path(post, path = "/api/v1/reviews/{review_item_id}/ratings", params(("review_item_id" = Uuid, Path)), request_body = RateReviewBody, responses((status = 200, body = ReviewItemResponse)), security(("bearer" = [])), tag = "reviews")]
pub async fn rate(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(review_item_id): Path<Uuid>,
    Json(body): Json<RateReviewBody>,
) -> Result<Json<ReviewItemResponse>, ApiError> {
    PgReviewRepository::new(state.pool)
        .rate(RateReview {
            subject_user_id: auth.owner_id(),
            review_item_id,
            rating: body.rating,
            reviewed_at: OffsetDateTime::now_utc(),
        })
        .await
        .map(response)
        .map(Json)
        .map_err(map_error)
}

fn response(item: ReviewItem) -> ReviewItemResponse {
    ReviewItemResponse {
        id: item.id,
        journey_id: item.journey_id,
        objective_id: item.objective_id,
        activity_id: item.activity_id,
        content_version: item.content_version,
        due_at: item.due_at,
        interval_days: item.interval_days,
        review_count: item.review_count,
    }
}

fn map_error(error: ReviewScheduleError) -> ApiError {
    match error {
        ReviewScheduleError::InvalidDesiredRetention | ReviewScheduleError::InvalidReviewOrder => {
            ApiError::Validation(vec![FieldError {
                field: "review".into(),
                message: error.to_string(),
            }])
        }
        ReviewScheduleError::Scheduler(message) if message.contains("not found") => {
            ApiError::NotFound {
                resource: "review item",
            }
        }
        ReviewScheduleError::Scheduler(message) => ApiError::Internal(anyhow::anyhow!(message)),
    }
}
