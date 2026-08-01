//! Fixture-origin boundary shared by progress-bearing API routes.

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::error::ApiError;

/// Reject an owned fixture journey before it can create or report learner
/// progress. The database separately ensures a fixture origin can only belong
/// to a registered fixture account and can never be changed after creation.
pub async fn require_learner_journey(
    pool: &PgPool,
    owner_id: Uuid,
    journey_id: Uuid,
) -> Result<(), ApiError> {
    let origin = sqlx::query_scalar::<_, String>(
        "SELECT origin FROM tb_learning_journeys WHERE id = $1 AND subject_user_id = $2",
    )
    .bind(journey_id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(anyhow::anyhow!(error)))?;

    match origin.as_deref() {
        Some("learner") => Ok(()),
        Some("fixture") => Err(ApiError::Forbidden(
            "fixture journeys cannot create or report learner progress".into(),
        )),
        Some(_) => Err(ApiError::Internal(anyhow::anyhow!(
            "unknown journey origin"
        ))),
        None => Err(ApiError::NotFound {
            resource: "learning journey",
        }),
    }
}
