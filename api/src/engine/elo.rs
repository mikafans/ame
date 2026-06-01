//! Elo update math for question attempts.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub const DEFAULT_RATING: f64 = 1200.0;
pub const QUESTION_CALIBRATION_ATTEMPTS: i32 = 20;
pub const USER_EARLY_ATTEMPTS: i32 = 30;
pub const K_USER_EARLY: f64 = 32.0;
pub const K_USER_MATURE: f64 = 16.0;
pub const K_QUESTION_CALIBRATION: f64 = 48.0;
pub const K_QUESTION_MATURE: f64 = 16.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UserTagRating {
    pub tag_id: Uuid,
    pub rating: f64,
    pub attempts_count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QuestionRating {
    pub rating: f64,
    pub attempts_count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UserTagRatingUpdate {
    pub tag_id: Uuid,
    pub rating_before: f64,
    pub rating_delta: f64,
    pub rating_after: f64,
    pub attempts_before: i32,
    pub attempts_after: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QuestionRatingUpdate {
    pub rating_before: f64,
    pub rating_delta: f64,
    pub rating_after: f64,
    pub attempts_before: i32,
    pub attempts_after: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EloUpdate {
    pub user_tags: Vec<UserTagRatingUpdate>,
    pub question: QuestionRatingUpdate,
    pub user_avg_before: f64,
    pub user_avg_after: f64,
}

/// Compute new ratings for a user and question after an attempt.
///
/// Returns the new ratings for each tag attached to the question, and the new
/// rating for the question itself.
pub fn compute_elo(
    user_ratings: &[UserTagRating],
    question_rating: &QuestionRating,
    score: f64, // 0.0 to 1.0
) -> EloUpdate {
    let user_avg_before = if user_ratings.is_empty() {
        DEFAULT_RATING
    } else {
        user_ratings.iter().map(|r| r.rating).sum::<f64>() / user_ratings.len() as f64
    };

    // 1. Update user ratings (one per tag)
    let user_tags = user_ratings
        .iter()
        .map(|r| {
            let k = if r.attempts_count < USER_EARLY_ATTEMPTS {
                K_USER_EARLY
            } else {
                K_USER_MATURE
            };
            let expected = expected_score(r.rating, question_rating.rating);
            let delta = k * (score - expected);
            UserTagRatingUpdate {
                tag_id: r.tag_id,
                rating_before: r.rating,
                rating_delta: delta,
                rating_after: r.rating + delta,
                attempts_before: r.attempts_count,
                attempts_after: r.attempts_count + 1,
            }
        })
        .collect::<Vec<_>>();

    // 2. Update question rating
    let k_q = if question_rating.attempts_count < QUESTION_CALIBRATION_ATTEMPTS {
        K_QUESTION_CALIBRATION
    } else {
        K_QUESTION_MATURE
    };
    let expected_q = expected_score(question_rating.rating, user_avg_before);
    // Question "wins" if user fails (score 0), "loses" if user succeeds (score 1)
    let q_delta = k_q * ((1.0 - score) - expected_q);

    let user_avg_after = if user_tags.is_empty() {
        DEFAULT_RATING
    } else {
        user_tags.iter().map(|r| r.rating_after).sum::<f64>() / user_tags.len() as f64
    };

    EloUpdate {
        user_tags,
        question: QuestionRatingUpdate {
            rating_before: question_rating.rating,
            rating_delta: q_delta,
            rating_after: question_rating.rating + q_delta,
            attempts_before: question_rating.attempts_count,
            attempts_after: question_rating.attempts_count + 1,
        },
        user_avg_before,
        user_avg_after,
    }
}

/// Probability of player A winning against player B.
fn expected_score(rating_a: f64, rating_b: f64) -> f64 {
    1.0 / (1.0 + 10.0f64.powf((rating_b - rating_a) / 400.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_attempt_calibration() {
        let tag_id = Uuid::now_v7();
        let user = vec![UserTagRating {
            tag_id,
            rating: 1200.0,
            attempts_count: 0,
        }];
        let question = QuestionRating {
            rating: 1200.0,
            attempts_count: 0,
        };

        // Correct answer (score 1.0)
        let update = compute_elo(&user, &question, 1.0);
        assert!(update.user_tags[0].rating_after > 1200.0);
        assert!(update.question.rating_after < 1200.0);
        assert_eq!(update.user_tags[0].attempts_after, 1);

        // Incorrect answer (score 0.0)
        let update = compute_elo(&user, &question, 0.0);
        assert!(update.user_tags[0].rating_after < 1200.0);
        assert!(update.question.rating_after > 1200.0);
    }

    #[test]
    fn high_rated_user_small_gain_on_easy_question() {
        let tag_id = Uuid::now_v7();
        let user = vec![UserTagRating {
            tag_id,
            rating: 2000.0,
            attempts_count: 100,
        }];
        let question = QuestionRating {
            rating: 1000.0,
            attempts_count: 100,
        };

        let update = compute_elo(&user, &question, 1.0);
        assert!(update.user_tags[0].rating_delta < 1.0);
    }

    #[test]
    fn correct_answers_never_decrease_user_rating() {
        let tag_id = Uuid::now_v7();
        let user = vec![UserTagRating {
            tag_id,
            rating: 1000.0,
            attempts_count: 0,
        }];
        let question = QuestionRating {
            rating: 2500.0, // extremely hard
            attempts_count: 1000,
        };

        let update = compute_elo(&user, &question, 1.0);
        assert!(update.user_tags[0].rating_delta > 0.0);
    }
}
