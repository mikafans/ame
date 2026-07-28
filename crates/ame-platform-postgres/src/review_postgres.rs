//! PostgreSQL spaced-review adapter.

use crate::{
    domain::review::{
        DEFAULT_DESIRED_RETENTION, ReviewMemoryState, ReviewScheduleError, ScheduleReview,
        schedule_review,
    },
    review::{RateReview, ReviewItem, ReviewRepository},
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgReviewRepository {
    pool: PgPool,
}

impl PgReviewRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReviewRepository for PgReviewRepository {
    async fn seed_from_evidence(
        &self,
        subject_user_id: Uuid,
        evidence_id: Uuid,
    ) -> Result<ReviewItem, ReviewScheduleError> {
        let row = sqlx::query(
            r#"INSERT INTO tb_review_items (
                    subject_user_id, journey_id, objective_id, activity_id,
                    content_version, source_evidence_id, due_at
                )
                SELECT e.subject_user_id, e.journey_id, e.objective_id, e.activity_id,
                       a.content_version, e.id, now()
                  FROM tb_mastery_evidence e
                  JOIN tb_activities a ON a.id = e.activity_id
                 WHERE e.id = $1 AND e.subject_user_id = $2
                ON CONFLICT (subject_user_id, source_evidence_id)
                DO UPDATE SET source_evidence_id = EXCLUDED.source_evidence_id
                RETURNING *"#,
        )
        .bind(evidence_id)
        .bind(subject_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or_else(|| ReviewScheduleError::Scheduler("evidence not found".into()))?;
        Ok(map_row(&row))
    }

    async fn list_due(
        &self,
        subject_user_id: Uuid,
        due_before: time::OffsetDateTime,
    ) -> Result<Vec<ReviewItem>, ReviewScheduleError> {
        sqlx::query(
            "SELECT * FROM tb_review_items
             WHERE subject_user_id = $1 AND due_at <= $2
             ORDER BY due_at, created_at",
        )
        .bind(subject_user_id)
        .bind(due_before)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)
        .map(|rows| rows.iter().map(map_row).collect())
    }

    async fn rate(&self, input: RateReview) -> Result<ReviewItem, ReviewScheduleError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let row = sqlx::query(
            "SELECT * FROM tb_review_items WHERE id = $1 AND subject_user_id = $2 FOR UPDATE",
        )
        .bind(input.review_item_id)
        .bind(input.subject_user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage)?
        .ok_or_else(|| ReviewScheduleError::Scheduler("review item not found".into()))?;
        let current = map_row(&row);
        let schedule = schedule_review(ScheduleReview {
            reviewed_at: input.reviewed_at,
            previous_reviewed_at: current.last_reviewed_at,
            memory_state: current.memory_state,
            rating: input.rating,
            desired_retention: DEFAULT_DESIRED_RETENTION,
        })?;
        let row = sqlx::query(
            r#"UPDATE tb_review_items
                  SET due_at = $3, last_reviewed_at = $4, interval_days = $5,
                      stability = $6, difficulty = $7,
                      review_count = review_count + 1, updated_at = now()
                WHERE id = $1 AND subject_user_id = $2
                RETURNING *"#,
        )
        .bind(input.review_item_id)
        .bind(input.subject_user_id)
        .bind(schedule.due_at)
        .bind(schedule.reviewed_at)
        .bind(schedule.interval_days as i32)
        .bind(schedule.memory_state.stability)
        .bind(schedule.memory_state.difficulty)
        .fetch_one(&mut *transaction)
        .await
        .map_err(storage)?;
        transaction.commit().await.map_err(storage)?;
        Ok(map_row(&row))
    }
}

fn map_row(row: &sqlx::postgres::PgRow) -> ReviewItem {
    let stability: Option<f32> = row.get("stability");
    let difficulty: Option<f32> = row.get("difficulty");
    ReviewItem {
        id: row.get("id"),
        subject_user_id: row.get("subject_user_id"),
        journey_id: row.get("journey_id"),
        objective_id: row.get("objective_id"),
        activity_id: row.get("activity_id"),
        content_version: row.get::<i32, _>("content_version") as u32,
        due_at: row.get("due_at"),
        last_reviewed_at: row.get("last_reviewed_at"),
        interval_days: row.get::<i32, _>("interval_days") as u32,
        memory_state: stability.zip(difficulty).map(|(stability, difficulty)| {
            ReviewMemoryState {
                stability,
                difficulty,
            }
        }),
        review_count: row.get::<i32, _>("review_count") as u32,
    }
}

fn storage(error: impl std::fmt::Display) -> ReviewScheduleError {
    ReviewScheduleError::Scheduler(error.to_string())
}
