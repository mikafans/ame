//! Pure session lifecycle rules.
//!
//! Persistence and HTTP routing sit outside this module. Keeping the lifecycle
//! pure lets the route/repository layer reuse one set of invariants for quiz,
//! exam, and practice sessions.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    domain::{
        attempt::{Attempt, AttemptPresentation, AttemptResponse},
        error::{ApiError, FieldError},
        question::QuestionKind,
        session::{PlanItem, QuestionPlan, Session, SessionKind, SessionStatus},
    },
    engine::{
        elo::{EloError, EloUpdate, QuestionRating, UserTagRating, update_elo},
        graders::{GradeError, GradeOutcome, GradeStatus, grade_response},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SessionResult {
    pub points_awarded: i32,
    pub max_points: i32,
    pub percent: f64,
    pub graded_count: usize,
    pub pending_manual_count: usize,
}

#[derive(Debug, Clone)]
pub struct StartSessionInput {
    pub user_id: Uuid,
    pub kind: SessionKind,
    pub quiz_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    pub filter: Option<serde_json::Value>,
    pub question_plan: QuestionPlan,
    pub affects_rating: bool,
    pub rating_snapshot: BTreeMap<Uuid, f64>,
    pub duration_min: Option<i64>,
    pub now: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct RuntimeQuestion {
    pub question_id: Uuid,
    pub version: i32,
    pub kind: QuestionKind,
    pub payload: serde_json::Value,
    pub max_points: i32,
    pub rating: QuestionRating,
    pub user_tag_ratings: Vec<UserTagRating>,
}

#[derive(Debug, Clone)]
pub struct AnswerInput {
    pub session: Session,
    pub question: RuntimeQuestion,
    pub response: AttemptResponse,
    pub time_to_answer_ms: Option<i32>,
    pub now: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct AnswerOutcome {
    pub attempt: Attempt,
    pub grade: GradeOutcome,
    pub elo: Option<EloUpdate>,
    pub replayed: bool,
}

pub fn start_session(input: StartSessionInput) -> Result<Session, ApiError> {
    validate_start_input(&input)?;

    let deadline_at = input
        .duration_min
        .map(|minutes| input.now + Duration::minutes(minutes));

    Ok(Session {
        id: Uuid::now_v7(),
        user_id: input.user_id,
        kind: input.kind,
        quiz_id: input.quiz_id,
        exam_id: input.exam_id,
        filter: input.filter,
        question_plan: input.question_plan,
        status: SessionStatus::InProgress,
        affects_rating: input.affects_rating,
        rating_snapshot: input.rating_snapshot,
        result: None,
        deadline_at,
        started_at: input.now,
        finished_at: None,
    })
}

pub fn answer_session(
    input: AnswerInput,
    existing_attempts: &HashMap<Uuid, Attempt>,
) -> Result<AnswerOutcome, ApiError> {
    ensure_answerable(&input.session, input.now)?;

    if let Some(existing) = existing_attempts.get(&input.question.question_id) {
        return Ok(AnswerOutcome {
            attempt: existing.clone(),
            grade: grade_from_attempt(existing, input.question.max_points),
            elo: None,
            replayed: true,
        });
    }

    let plan_item = find_plan_item(&input.session.question_plan, input.question.question_id)?;
    if plan_item.version != input.question.version {
        return Err(ApiError::Validation(vec![FieldError {
            field: "question_version".to_string(),
            message: format!(
                "planned version {} does not match runtime version {}",
                plan_item.version, input.question.version
            ),
        }]));
    }

    let presentation = AttemptPresentation {
        option_order: plan_item.option_order.clone(),
    };
    let grade = grade_response(
        input.question.kind,
        &input.question.payload,
        &input.response,
        &presentation,
        input.question.max_points,
    )
    .map_err(map_grade_error)?;

    let score_fraction = if grade.max == 0 {
        0.0
    } else {
        grade.points_awarded as f64 / grade.max as f64
    };

    let elo = if input.session.affects_rating && grade.status == GradeStatus::Graded {
        Some(
            update_elo(
                input.question.rating.clone(),
                &input.question.user_tag_ratings,
                score_fraction,
            )
            .map_err(map_elo_error)?,
        )
    } else {
        None
    };

    let (rating_before_user_avg, rating_before_question, user_tag_deltas, question_delta) =
        rating_fields(&input.question, elo.as_ref());

    let attempt = Attempt {
        id: Uuid::now_v7(),
        user_id: input.session.user_id,
        question_id: input.question.question_id,
        question_version: input.question.version,
        session_id: Some(input.session.id),
        response: input.response,
        presentation,
        is_correct: grade.correct,
        score: score_fraction,
        time_to_answer_ms: input.time_to_answer_ms,
        rating_before_user_avg,
        rating_before_question,
        user_tag_deltas,
        question_delta,
        created_at: input.now,
    };

    Ok(AnswerOutcome {
        attempt,
        grade,
        elo,
        replayed: false,
    })
}

pub fn finish_session(
    mut session: Session,
    attempts: &[Attempt],
    max_points_by_question: &HashMap<Uuid, i32>,
    now: OffsetDateTime,
) -> Result<Session, ApiError> {
    if session.status != SessionStatus::InProgress {
        return Err(ApiError::SessionFinished);
    }

    let result = summarize_attempts(attempts, max_points_by_question);
    session.status = SessionStatus::Finished;
    session.finished_at = Some(now);
    session.result = Some(serde_json::to_value(&result).map_err(anyhow::Error::from)?);
    Ok(session)
}

fn validate_start_input(input: &StartSessionInput) -> Result<(), ApiError> {
    let mut fields = Vec::new();

    match input.kind {
        SessionKind::Quiz => {
            if input.quiz_id.is_none() {
                fields.push(field_error("quiz_id", "quiz sessions require quiz_id"));
            }
            if input.exam_id.is_some() {
                fields.push(field_error(
                    "exam_id",
                    "quiz sessions cannot include exam_id",
                ));
            }
        }
        SessionKind::Exam => {
            if input.exam_id.is_none() {
                fields.push(field_error("exam_id", "exam sessions require exam_id"));
            }
            if input.quiz_id.is_some() {
                fields.push(field_error(
                    "quiz_id",
                    "exam sessions cannot include quiz_id",
                ));
            }
        }
        SessionKind::Practice => {
            if input.quiz_id.is_some() || input.exam_id.is_some() {
                fields.push(field_error(
                    "kind",
                    "practice sessions cannot include quiz_id or exam_id",
                ));
            }
            if input.filter.is_none() {
                fields.push(field_error("filter", "practice sessions require filter"));
            }
        }
    }

    if input.question_plan.items.is_empty() {
        fields.push(field_error(
            "question_plan",
            "must include at least one item",
        ));
    }
    if matches!(input.duration_min, Some(minutes) if minutes <= 0) {
        fields.push(field_error("duration_min", "must be greater than zero"));
    }

    if fields.is_empty() {
        Ok(())
    } else {
        Err(ApiError::Validation(fields))
    }
}

fn ensure_answerable(session: &Session, now: OffsetDateTime) -> Result<(), ApiError> {
    if session.status != SessionStatus::InProgress {
        return Err(ApiError::SessionFinished);
    }
    if session.deadline_at.is_some_and(|deadline| now > deadline) {
        return Err(ApiError::ExamExpired);
    }
    Ok(())
}

fn find_plan_item(plan: &QuestionPlan, question_id: Uuid) -> Result<&PlanItem, ApiError> {
    plan.items
        .iter()
        .find(|item| item.question_id == question_id)
        .ok_or_else(|| {
            ApiError::Validation(vec![FieldError {
                field: "question_id".to_string(),
                message: "question is not part of this session plan".to_string(),
            }])
        })
}

fn grade_from_attempt(attempt: &Attempt, max_points: i32) -> GradeOutcome {
    GradeOutcome {
        status: GradeStatus::Graded,
        correct: attempt.is_correct,
        points_awarded: (attempt.score * max_points as f64).round() as i32,
        max: max_points,
        note: Some("idempotent replay".to_string()),
        correct_answer: serde_json::Value::Null,
    }
}

fn rating_fields(
    question: &RuntimeQuestion,
    elo: Option<&EloUpdate>,
) -> (f64, f64, BTreeMap<String, f64>, f64) {
    if let Some(elo) = elo {
        let deltas = elo
            .user_tags
            .iter()
            .map(|tag| (tag.tag_id.to_string(), tag.rating_delta))
            .collect();
        (
            elo.user_avg_before,
            elo.question.rating_before,
            deltas,
            elo.question.rating_delta,
        )
    } else {
        let user_avg = if question.user_tag_ratings.is_empty() {
            0.0
        } else {
            question
                .user_tag_ratings
                .iter()
                .map(|rating| rating.rating)
                .sum::<f64>()
                / question.user_tag_ratings.len() as f64
        };
        (user_avg, question.rating.rating, BTreeMap::new(), 0.0)
    }
}

fn summarize_attempts(
    attempts: &[Attempt],
    max_points_by_question: &HashMap<Uuid, i32>,
) -> SessionResult {
    let mut points_awarded = 0;
    let mut max_points = 0;
    let mut graded_count = 0;
    let mut pending_manual_count = 0;

    for attempt in attempts {
        let max = max_points_by_question
            .get(&attempt.question_id)
            .copied()
            .unwrap_or(1);
        max_points += max;
        let awarded = (attempt.score * max as f64).round() as i32;
        points_awarded += awarded;
        if attempt.score == 0.0 && !attempt.is_correct {
            pending_manual_count += 1;
        } else {
            graded_count += 1;
        }
    }

    SessionResult {
        points_awarded,
        max_points,
        percent: if max_points == 0 {
            0.0
        } else {
            points_awarded as f64 / max_points as f64
        },
        graded_count,
        pending_manual_count,
    }
}

fn field_error(field: &str, message: &str) -> FieldError {
    FieldError {
        field: field.to_string(),
        message: message.to_string(),
    }
}

fn map_grade_error(err: GradeError) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: "response".to_string(),
        message: err.to_string(),
    }])
}

fn map_elo_error(err: EloError) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: "rating".to_string(),
        message: err.to_string(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::elo::{DEFAULT_RATING, QUESTION_CALIBRATION_ATTEMPTS};

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
    }

    fn user_id() -> Uuid {
        Uuid::from_u128(1)
    }

    fn question_id() -> Uuid {
        Uuid::from_u128(2)
    }

    fn quiz_session(affects_rating: bool) -> Session {
        start_session(StartSessionInput {
            user_id: user_id(),
            kind: SessionKind::Quiz,
            quiz_id: Some(Uuid::from_u128(9)),
            exam_id: None,
            filter: None,
            question_plan: QuestionPlan {
                items: vec![PlanItem {
                    question_id: question_id(),
                    version: 1,
                    section: None,
                    option_order: Some(vec![1, 0, 2]),
                }],
            },
            affects_rating,
            rating_snapshot: BTreeMap::new(),
            duration_min: Some(30),
            now: now(),
        })
        .unwrap()
    }

    fn mc_question() -> RuntimeQuestion {
        RuntimeQuestion {
            question_id: question_id(),
            version: 1,
            kind: QuestionKind::Mc,
            payload: serde_json::json!({
                "options": ["a", "b", "c"],
                "correct_index": 1
            }),
            max_points: 2,
            rating: QuestionRating {
                rating: DEFAULT_RATING,
                attempts_count: QUESTION_CALIBRATION_ATTEMPTS,
            },
            user_tag_ratings: vec![UserTagRating {
                tag_id: Uuid::from_u128(100),
                rating: DEFAULT_RATING,
                attempts_count: 30,
            }],
        }
    }

    #[test]
    fn start_session_computes_deadline_and_validates_kind_links() {
        let session = quiz_session(true);

        assert_eq!(session.kind, SessionKind::Quiz);
        assert_eq!(session.status, SessionStatus::InProgress);
        assert_eq!(session.deadline_at, Some(now() + Duration::minutes(30)));

        let err = start_session(StartSessionInput {
            kind: SessionKind::Practice,
            quiz_id: Some(Uuid::from_u128(4)),
            ..start_input_for_practice()
        })
        .unwrap_err();
        match err {
            ApiError::Validation(fields) => assert_eq!(fields[0].field, "kind"),
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn practice_session_requires_filter() {
        let err = start_session(StartSessionInput {
            filter: None,
            ..start_input_for_practice()
        })
        .unwrap_err();

        match err {
            ApiError::Validation(fields) => assert_eq!(fields[0].field, "filter"),
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn answer_grades_and_updates_elo_when_rating_is_enabled() {
        let outcome = answer_session(
            AnswerInput {
                session: quiz_session(true),
                question: mc_question(),
                response: AttemptResponse::Mc {
                    selected_position: 0,
                },
                time_to_answer_ms: Some(1200),
                now: now() + Duration::minutes(1),
            },
            &HashMap::new(),
        )
        .unwrap();

        assert!(!outcome.replayed);
        assert!(outcome.grade.correct);
        assert_eq!(outcome.grade.points_awarded, 2);
        assert!(outcome.elo.is_some());
        assert_eq!(outcome.attempt.score, 1.0);
        assert_eq!(outcome.attempt.question_delta, -8.0);
    }

    #[test]
    fn answer_replay_returns_existing_attempt_without_regrading() {
        let first = answer_session(
            AnswerInput {
                session: quiz_session(true),
                question: mc_question(),
                response: AttemptResponse::Mc {
                    selected_position: 0,
                },
                time_to_answer_ms: None,
                now: now(),
            },
            &HashMap::new(),
        )
        .unwrap();
        let existing = HashMap::from([(question_id(), first.attempt.clone())]);

        let replay = answer_session(
            AnswerInput {
                session: quiz_session(true),
                question: mc_question(),
                response: AttemptResponse::Mc {
                    selected_position: 2,
                },
                time_to_answer_ms: None,
                now: now(),
            },
            &existing,
        )
        .unwrap();

        assert!(replay.replayed);
        assert_eq!(replay.attempt.id, first.attempt.id);
        assert!(replay.elo.is_none());
    }

    #[test]
    fn answer_after_deadline_returns_exam_expired() {
        let err = answer_session(
            AnswerInput {
                session: quiz_session(true),
                question: mc_question(),
                response: AttemptResponse::Mc {
                    selected_position: 0,
                },
                time_to_answer_ms: None,
                now: now() + Duration::minutes(31),
            },
            &HashMap::new(),
        )
        .unwrap_err();

        assert!(matches!(err, ApiError::ExamExpired));
    }

    #[test]
    fn affects_rating_false_skips_elo_updates() {
        let outcome = answer_session(
            AnswerInput {
                session: quiz_session(false),
                question: mc_question(),
                response: AttemptResponse::Mc {
                    selected_position: 0,
                },
                time_to_answer_ms: None,
                now: now(),
            },
            &HashMap::new(),
        )
        .unwrap();

        assert!(outcome.grade.correct);
        assert!(outcome.elo.is_none());
        assert_eq!(outcome.attempt.question_delta, 0.0);
    }

    #[test]
    fn finish_marks_session_done_and_summarizes_result() {
        let outcome = answer_session(
            AnswerInput {
                session: quiz_session(false),
                question: mc_question(),
                response: AttemptResponse::Mc {
                    selected_position: 0,
                },
                time_to_answer_ms: None,
                now: now(),
            },
            &HashMap::new(),
        )
        .unwrap();

        let finished = finish_session(
            quiz_session(false),
            &[outcome.attempt],
            &HashMap::from([(question_id(), 2)]),
            now() + Duration::minutes(2),
        )
        .unwrap();

        assert_eq!(finished.status, SessionStatus::Finished);
        let result: SessionResult = serde_json::from_value(finished.result.unwrap()).unwrap();
        assert_eq!(result.points_awarded, 2);
        assert_eq!(result.max_points, 2);
        assert_eq!(result.percent, 1.0);
    }

    fn start_input_for_practice() -> StartSessionInput {
        StartSessionInput {
            user_id: user_id(),
            kind: SessionKind::Practice,
            quiz_id: None,
            exam_id: None,
            filter: Some(serde_json::json!({ "tags": ["rust"] })),
            question_plan: QuestionPlan {
                items: vec![PlanItem {
                    question_id: question_id(),
                    version: 1,
                    section: None,
                    option_order: None,
                }],
            },
            affects_rating: true,
            rating_snapshot: BTreeMap::new(),
            duration_min: None,
            now: now(),
        }
    }
}
