//! Elo update math for question attempts.

use uuid::Uuid;

pub const DEFAULT_RATING: f64 = 1200.0;
pub const QUESTION_CALIBRATION_ATTEMPTS: i32 = 20;
pub const USER_EARLY_ATTEMPTS: i32 = 30;
pub const K_USER_EARLY: f64 = 32.0;
pub const K_USER_MATURE: f64 = 16.0;
pub const K_QUESTION_CALIBRATION: f64 = 48.0;
pub const K_QUESTION_MATURE: f64 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct UserTagRating {
    pub tag_id: Uuid,
    pub rating: f64,
    pub attempts_count: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestionRating {
    pub rating: f64,
    pub attempts_count: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserTagRatingUpdate {
    pub tag_id: Uuid,
    pub rating_before: f64,
    pub rating_delta: f64,
    pub rating_after: f64,
    pub attempts_before: i32,
    pub attempts_after: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuestionRatingUpdate {
    pub rating_before: f64,
    pub rating_delta: f64,
    pub rating_after: f64,
    pub attempts_before: i32,
    pub attempts_after: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EloUpdate {
    pub user_avg_before: f64,
    pub expected_score: f64,
    pub k_user: f64,
    pub k_question: f64,
    pub user_delta_total: f64,
    pub user_tags: Vec<UserTagRatingUpdate>,
    pub question: QuestionRatingUpdate,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EloError {
    #[error("score must be finite and within 0..=1")]
    InvalidScore,
    #[error("at least one question tag is required")]
    NoTags,
    #[error("rating values must be finite")]
    NonFiniteRating,
}

pub fn update_elo(
    question: QuestionRating,
    user_tags: &[UserTagRating],
    score: f64,
) -> Result<EloUpdate, EloError> {
    if !score.is_finite() || !(0.0..=1.0).contains(&score) {
        return Err(EloError::InvalidScore);
    }
    if user_tags.is_empty() {
        return Err(EloError::NoTags);
    }
    if !question.rating.is_finite() || user_tags.iter().any(|tag| !tag.rating.is_finite()) {
        return Err(EloError::NonFiniteRating);
    }

    let user_avg_before =
        user_tags.iter().map(|tag| tag.rating).sum::<f64>() / user_tags.len() as f64;
    let expected_score = expected_score(user_avg_before, question.rating);
    let (k_user, k_question) = k_factors(question.attempts_count, user_tags);
    let user_delta_total = k_user * (score - expected_score);
    let question_delta = k_question * (expected_score - score);
    let per_tag_delta = user_delta_total / user_tags.len() as f64;

    let user_tags = user_tags
        .iter()
        .map(|tag| UserTagRatingUpdate {
            tag_id: tag.tag_id,
            rating_before: tag.rating,
            rating_delta: per_tag_delta,
            rating_after: tag.rating + per_tag_delta,
            attempts_before: tag.attempts_count,
            attempts_after: tag.attempts_count.saturating_add(1),
        })
        .collect();

    Ok(EloUpdate {
        user_avg_before,
        expected_score,
        k_user,
        k_question,
        user_delta_total,
        user_tags,
        question: QuestionRatingUpdate {
            rating_before: question.rating,
            rating_delta: question_delta,
            rating_after: question.rating + question_delta,
            attempts_before: question.attempts_count,
            attempts_after: question.attempts_count.saturating_add(1),
        },
    })
}

fn expected_score(user_avg: f64, question_rating: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf((question_rating - user_avg) / 400.0))
}

fn k_factors(question_attempts_count: i32, user_tags: &[UserTagRating]) -> (f64, f64) {
    if question_attempts_count < QUESTION_CALIBRATION_ATTEMPTS {
        return (0.0, K_QUESTION_CALIBRATION);
    }

    let min_user_attempts = user_tags
        .iter()
        .map(|tag| tag.attempts_count)
        .min()
        .unwrap_or(0);
    let k_user = if min_user_attempts < USER_EARLY_ATTEMPTS {
        K_USER_EARLY
    } else {
        K_USER_MATURE
    };

    (k_user, K_QUESTION_MATURE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn tag(rating: f64, attempts_count: i32) -> UserTagRating {
        UserTagRating {
            tag_id: Uuid::now_v7(),
            rating,
            attempts_count,
        }
    }

    #[test]
    fn calibration_phase_freezes_user_and_moves_question_fast() {
        let update = update_elo(
            QuestionRating {
                rating: DEFAULT_RATING,
                attempts_count: QUESTION_CALIBRATION_ATTEMPTS - 1,
            },
            &[tag(DEFAULT_RATING, USER_EARLY_ATTEMPTS)],
            1.0,
        )
        .unwrap();

        assert_eq!(update.k_user, 0.0);
        assert_eq!(update.k_question, K_QUESTION_CALIBRATION);
        assert_eq!(update.user_delta_total, 0.0);
        assert_eq!(update.user_tags[0].rating_delta, 0.0);
        assert!(update.question.rating_delta < 0.0);
    }

    #[test]
    fn mature_phase_uses_mature_k_for_user_and_question() {
        let update = update_elo(
            QuestionRating {
                rating: DEFAULT_RATING,
                attempts_count: QUESTION_CALIBRATION_ATTEMPTS,
            },
            &[
                tag(DEFAULT_RATING, USER_EARLY_ATTEMPTS),
                tag(DEFAULT_RATING + 100.0, USER_EARLY_ATTEMPTS + 5),
            ],
            0.0,
        )
        .unwrap();

        assert_eq!(update.k_user, K_USER_MATURE);
        assert_eq!(update.k_question, K_QUESTION_MATURE);
        assert_eq!(update.user_tags.len(), 2);
        assert!(
            (update.user_tags[0].rating_delta - update.user_tags[1].rating_delta).abs() < 1e-12
        );
        assert!(update.user_delta_total < 0.0);
        assert!(update.question.rating_delta > 0.0);
    }

    #[test]
    fn early_user_phase_uses_larger_user_k_after_question_calibrates() {
        let update = update_elo(
            QuestionRating {
                rating: DEFAULT_RATING,
                attempts_count: QUESTION_CALIBRATION_ATTEMPTS,
            },
            &[tag(DEFAULT_RATING, USER_EARLY_ATTEMPTS - 1)],
            1.0,
        )
        .unwrap();

        assert_eq!(update.k_user, K_USER_EARLY);
        assert_eq!(update.k_question, K_QUESTION_MATURE);
    }

    proptest! {
        #[test]
        fn elo_outputs_stay_finite(
            question_rating in 100.0_f64..3000.0,
            user_rating in 100.0_f64..3000.0,
            score in 0.0_f64..1.0,
            question_attempts in 0_i32..5000,
            user_attempts in 0_i32..5000,
        ) {
            let update = update_elo(
                QuestionRating {
                    rating: question_rating,
                    attempts_count: question_attempts,
                },
                &[tag(user_rating, user_attempts)],
                score,
            ).unwrap();

            prop_assert!(update.user_avg_before.is_finite());
            prop_assert!(update.expected_score.is_finite());
            prop_assert!((0.0..=1.0).contains(&update.expected_score));
            prop_assert!(update.user_delta_total.is_finite());
            prop_assert!(update.question.rating_delta.is_finite());
            prop_assert!(update.question.rating_after.is_finite());
            prop_assert!(update.user_tags.iter().all(|tag| tag.rating_after.is_finite()));
        }

        #[test]
        fn correct_answers_never_decrease_user_rating(
            question_rating in 100.0_f64..3000.0,
            user_rating in 100.0_f64..3000.0,
            question_attempts in 0_i32..5000,
            user_attempts in 0_i32..5000,
        ) {
            let update = update_elo(
                QuestionRating {
                    rating: question_rating,
                    attempts_count: question_attempts,
                },
                &[tag(user_rating, user_attempts)],
                1.0,
            ).unwrap();

            prop_assert!(update.user_delta_total >= -1e-12);
            prop_assert!(update.user_tags.iter().all(|tag| tag.rating_delta >= -1e-12));
        }

        #[test]
        fn wrong_answers_never_increase_user_rating(
            question_rating in 100.0_f64..3000.0,
            user_rating in 100.0_f64..3000.0,
            question_attempts in 0_i32..5000,
            user_attempts in 0_i32..5000,
        ) {
            let update = update_elo(
                QuestionRating {
                    rating: question_rating,
                    attempts_count: question_attempts,
                },
                &[tag(user_rating, user_attempts)],
                0.0,
            ).unwrap();

            prop_assert!(update.user_delta_total <= 1e-12);
            prop_assert!(update.user_tags.iter().all(|tag| tag.rating_delta <= 1e-12));
        }

        #[test]
        fn mature_equal_k_updates_are_energy_conserving(
            question_rating in 100.0_f64..3000.0,
            user_rating_1 in 100.0_f64..3000.0,
            user_rating_2 in 100.0_f64..3000.0,
            score in 0.0_f64..1.0,
        ) {
            let update = update_elo(
                QuestionRating {
                    rating: question_rating,
                    attempts_count: QUESTION_CALIBRATION_ATTEMPTS,
                },
                &[
                    tag(user_rating_1, USER_EARLY_ATTEMPTS),
                    tag(user_rating_2, USER_EARLY_ATTEMPTS + 1),
                ],
                score,
            ).unwrap();

            let user_delta_sum: f64 = update.user_tags.iter().map(|tag| tag.rating_delta).sum();
            prop_assert!((user_delta_sum + update.question.rating_delta).abs() < 1e-9);
        }
    }
}
