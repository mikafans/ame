//! Deterministic FSRS review scheduling contracts.

use fsrs::{FSRS, MemoryState};
use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime, Time};
use utoipa::ToSchema;

pub const DEFAULT_DESIRED_RETENTION: f32 = 0.9;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRating {
    Again,
    Hard,
    Good,
    Easy,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct ReviewMemoryState {
    pub stability: f32,
    pub difficulty: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScheduleReview {
    pub reviewed_at: OffsetDateTime,
    pub previous_reviewed_at: Option<OffsetDateTime>,
    pub memory_state: Option<ReviewMemoryState>,
    pub rating: ReviewRating,
    pub desired_retention: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewSchedule {
    pub reviewed_at: OffsetDateTime,
    pub due_at: OffsetDateTime,
    pub interval_days: u32,
    pub memory_state: ReviewMemoryState,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ReviewScheduleError {
    #[error("desired retention must be between 0.7 and 0.97")]
    InvalidDesiredRetention,
    #[error("previous review must not be after this review")]
    InvalidReviewOrder,
    #[error("FSRS could not calculate the next review: {0}")]
    Scheduler(String),
}

pub fn schedule_review(input: ScheduleReview) -> Result<ReviewSchedule, ReviewScheduleError> {
    if !(0.7..=0.97).contains(&input.desired_retention) {
        return Err(ReviewScheduleError::InvalidDesiredRetention);
    }
    if input
        .previous_reviewed_at
        .is_some_and(|previous| previous > input.reviewed_at)
    {
        return Err(ReviewScheduleError::InvalidReviewOrder);
    }

    let elapsed_days = input
        .previous_reviewed_at
        .map(|previous| (input.reviewed_at - previous).whole_days().max(0) as u32)
        .unwrap_or(0);
    let state = input.memory_state.map(|state| MemoryState {
        stability: state.stability,
        difficulty: state.difficulty,
    });
    let states = FSRS::default()
        .next_states(state, input.desired_retention, elapsed_days)
        .map_err(|error| ReviewScheduleError::Scheduler(error.to_string()))?;
    let next = match input.rating {
        ReviewRating::Again => states.again,
        ReviewRating::Hard => states.hard,
        ReviewRating::Good => states.good,
        ReviewRating::Easy => states.easy,
    };
    let interval_days = next.interval.round().max(1.0) as u32;
    let reviewed_day = input.reviewed_at.date();
    let due_day = reviewed_day + Duration::days(i64::from(interval_days));
    let due_at = OffsetDateTime::new_in_offset(due_day, Time::MIDNIGHT, input.reviewed_at.offset());

    Ok(ReviewSchedule {
        reviewed_at: input.reviewed_at,
        due_at,
        interval_days,
        memory_state: ReviewMemoryState {
            stability: next.memory.stability,
            difficulty: next.memory.difficulty,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn new_review(rating: ReviewRating) -> ScheduleReview {
        ScheduleReview {
            reviewed_at: datetime!(2026-07-28 12:00 +09:00),
            previous_reviewed_at: None,
            memory_state: None,
            rating,
            desired_retention: DEFAULT_DESIRED_RETENTION,
        }
    }

    #[test]
    fn fsrs_default_vectors_order_new_card_intervals() {
        let again = schedule_review(new_review(ReviewRating::Again)).unwrap();
        let hard = schedule_review(new_review(ReviewRating::Hard)).unwrap();
        let good = schedule_review(new_review(ReviewRating::Good)).unwrap();
        let easy = schedule_review(new_review(ReviewRating::Easy)).unwrap();

        assert!(again.interval_days <= hard.interval_days);
        assert!(hard.interval_days <= good.interval_days);
        assert!(good.interval_days <= easy.interval_days);
        assert_eq!(
            good.due_at.date(),
            Date::from_calendar_date(2026, time::Month::July, 28).unwrap()
                + Duration::days(i64::from(good.interval_days))
        );
    }

    #[test]
    fn delayed_review_uses_previous_memory_state() {
        let scheduled = schedule_review(ScheduleReview {
            reviewed_at: datetime!(2026-07-28 12:00 +09:00),
            previous_reviewed_at: Some(datetime!(2026-07-21 12:00 +09:00)),
            memory_state: Some(ReviewMemoryState {
                stability: 7.0,
                difficulty: 5.0,
            }),
            rating: ReviewRating::Good,
            desired_retention: DEFAULT_DESIRED_RETENTION,
        })
        .unwrap();

        assert!(scheduled.interval_days > 1);
        assert!(scheduled.memory_state.stability > 7.0);
    }

    #[test]
    fn rejects_invalid_retention_and_time_order() {
        assert_eq!(
            schedule_review(ScheduleReview {
                desired_retention: 0.5,
                ..new_review(ReviewRating::Good)
            }),
            Err(ReviewScheduleError::InvalidDesiredRetention)
        );
        assert_eq!(
            schedule_review(ScheduleReview {
                previous_reviewed_at: Some(datetime!(2026-07-29 12:00 +09:00)),
                ..new_review(ReviewRating::Good)
            }),
            Err(ReviewScheduleError::InvalidReviewOrder)
        );
    }
}
