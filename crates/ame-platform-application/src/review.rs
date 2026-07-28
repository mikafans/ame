//! Owner-scoped spaced-review repository contracts.

use crate::domain::review::{ReviewMemoryState, ReviewRating, ReviewScheduleError};
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewItem {
    pub id: Uuid,
    pub subject_user_id: Uuid,
    pub journey_id: Uuid,
    pub objective_id: Uuid,
    pub activity_id: Uuid,
    pub content_version: u32,
    pub due_at: OffsetDateTime,
    pub last_reviewed_at: Option<OffsetDateTime>,
    pub interval_days: u32,
    pub memory_state: Option<ReviewMemoryState>,
    pub review_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RateReview {
    pub subject_user_id: Uuid,
    pub review_item_id: Uuid,
    pub rating: ReviewRating,
    pub reviewed_at: OffsetDateTime,
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    async fn seed_from_evidence(
        &self,
        subject_user_id: Uuid,
        evidence_id: Uuid,
    ) -> Result<ReviewItem, ReviewScheduleError>;

    async fn list_due(
        &self,
        subject_user_id: Uuid,
        due_before: OffsetDateTime,
    ) -> Result<Vec<ReviewItem>, ReviewScheduleError>;

    async fn rate(&self, input: RateReview) -> Result<ReviewItem, ReviewScheduleError>;
}
