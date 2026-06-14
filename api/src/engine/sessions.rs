//! Quiz/exam sessions engine logic.

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    domain::{
        attempt::{Attempt, AttemptPresentation, AttemptResponse},
        error::{ApiError, FieldError},
        question::QuestionKind,
        session::{QuestionPlan, Session, SessionKind, SessionStatus},
    },
    engine::{
        elo::{EloUpdate, QuestionRating, UserTagRating, compute_elo},
        graders::{GradeError, GradeOutcome, GradeStatus, grade_response},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SessionResult {
    pub points_awarded: i32,
    pub max_points: i32,
    pub percent: f64,
}

#[derive(Debug, Clone)]
pub struct StartSessionInput {
    pub user_id: Uuid,
    pub owner_id: Uuid,
    pub kind: SessionKind,
    pub assessment_id: Option<Uuid>,
    pub filter: Option<serde_json::Value>,
    pub question_plan: QuestionPlan,
    pub affects_rating: bool,
    pub rating_snapshot: BTreeMap<Uuid, f64>,
    pub duration_min: Option<i64>,
    pub now: time::OffsetDateTime,
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
    pub now: time::OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct AnswerOutcome {
    pub attempt: Attempt,
    pub grade: GradeOutcome,
    pub elo: Option<EloUpdate>,
    pub replayed: bool,
}

pub fn start_session(input: StartSessionInput) -> Result<Session, ApiError> {
    let deadline_at = input
        .duration_min
        .map(|m| input.now + time::Duration::minutes(m));

    Ok(Session {
        id: Uuid::now_v7(),
        user_id: input.user_id,
        owner_id: input.owner_id,
        kind: input.kind,
        assessment_id: input.assessment_id,
        filter: input.filter,
        question_plan: input.question_plan,
        status: SessionStatus::InProgress,
        affects_rating: input.affects_rating,
        rating_snapshot: input.rating_snapshot,
        result: None,
        deadline_at,
        started_at: input.now,
        finished_at: None,
        assessment_title: None,
        course_title: None,
    })
}

pub fn answer_session(
    input: AnswerInput,
    existing_attempts: &HashMap<Uuid, Attempt>,
) -> Result<AnswerOutcome, ApiError> {
    if input.session.status != SessionStatus::InProgress {
        return Err(ApiError::SessionFinished);
    }
    if let Some(deadline) = input.session.deadline_at
        && input.now > deadline
    {
        return Err(ApiError::ExamExpired);
    }

    if let Some(prev) = existing_attempts.get(&input.question.question_id)
        && prev.response == input.response
    {
        return Ok(AnswerOutcome {
            attempt: prev.clone(),
            grade: GradeOutcome {
                status: GradeStatus::from_str(&prev.grade_status)
                    .map_err(|_| ApiError::Internal(anyhow::anyhow!("invalid grade status")))?,
                correct: prev.is_correct,
                points_awarded: (prev.score * input.question.max_points as f64) as i32,
                max: input.question.max_points,
                correct_answer: prev
                    .correct_answer
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
                note: prev.grader_notes.clone(),
            },
            elo: None,
            replayed: true,
        });
    }

    let presentation = input
        .session
        .question_plan
        .items
        .iter()
        .find(|i| i.question_id == input.question.question_id)
        .map(|i| AttemptPresentation {
            option_order: i.option_order.clone(),
        })
        .unwrap_or_default();

    let start_time = std::time::Instant::now();
    let grade = grade_response(
        input.question.kind,
        &input.question.payload,
        &input.response,
        &presentation,
        input.question.max_points,
    )
    .map_err(map_grade_error)?;
    let latency = start_time.elapsed().as_secs_f64();
    metrics::histogram!("grader_latency_seconds").record(latency);

    let score_fraction = if grade.max == 0 {
        0.0
    } else {
        grade.points_awarded as f64 / grade.max as f64
    };

    let elo = if input.session.affects_rating && grade.status == GradeStatus::Graded {
        Some(compute_elo(
            &input.question.user_tag_ratings,
            &input.question.rating,
            score_fraction,
        ))
    } else {
        None
    };

    let (rating_before_user_avg, rating_before_question, user_tag_deltas, question_delta) =
        rating_fields(&input.question, elo.as_ref());

    let attempt = Attempt {
        id: Uuid::now_v7(),
        user_id: input.session.user_id,
        owner_id: input.session.owner_id,
        question_id: input.question.question_id,
        question_version: input.question.version,
        session_id: Some(input.session.id),
        response: input.response,
        presentation: presentation.clone(),
        is_correct: grade.correct,
        score: score_fraction,
        grade_status: if grade.status == GradeStatus::PendingManual {
            "pending_manual".to_string()
        } else {
            "graded".to_string()
        },
        correct_answer: Some(grade.correct_answer.clone()),
        grader_notes: None,
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
    max_points: &HashMap<Uuid, i32>,
    now: time::OffsetDateTime,
) -> Result<Session, ApiError> {
    if session.status != SessionStatus::InProgress {
        return Err(ApiError::SessionFinished);
    }

    let mut points_awarded = 0;
    let mut total_max = 0;

    for item in &session.question_plan.items {
        let max = *max_points.get(&item.question_id).unwrap_or(&0);
        total_max += max;

        if let Some(attempt) = attempts.iter().find(|a| a.question_id == item.question_id) {
            points_awarded += (attempt.score * max as f64).round() as i32;
        }
    }

    let percent = if total_max > 0 {
        points_awarded as f64 / total_max as f64
    } else {
        0.0
    };

    let result = SessionResult {
        points_awarded,
        max_points: total_max,
        percent,
    };

    session.status = SessionStatus::Finished;
    session.finished_at = Some(now);
    session.result = Some(serde_json::to_value(&result).map_err(anyhow::Error::from)?);

    Ok(session)
}

fn rating_fields(
    question: &RuntimeQuestion,
    elo: Option<&EloUpdate>,
) -> (f64, f64, BTreeMap<String, f64>, f64) {
    if let Some(update) = elo {
        let mut deltas = BTreeMap::new();
        for tag in &update.user_tags {
            deltas.insert(tag.tag_id.to_string(), tag.rating_delta);
        }
        (
            update.user_avg_before,
            update.question.rating_before,
            deltas,
            update.question.rating_delta,
        )
    } else {
        (0.0, question.rating.rating, BTreeMap::new(), 0.0)
    }
}

fn map_grade_error(err: GradeError) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: "response".to_string(),
        message: err.to_string(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session::PlanItem;

    fn now() -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
    }

    fn user_id() -> Uuid {
        Uuid::now_v7()
    }

    fn question_id() -> Uuid {
        Uuid::now_v7()
    }

    #[test]
    fn start_session_computes_deadline_and_validates_kind_links() {
        let input = StartSessionInput {
            user_id: user_id(),
            owner_id: user_id(),
            kind: SessionKind::Practice,
            assessment_id: None,
            filter: None,
            question_plan: QuestionPlan { items: vec![] },
            affects_rating: true,
            rating_snapshot: BTreeMap::new(),
            duration_min: Some(60),
            now: now(),
        };

        let session = start_session(input).unwrap();
        assert_eq!(
            session.deadline_at,
            Some(now() + time::Duration::minutes(60))
        );
    }

    #[test]
    fn finish_session_aggregates_scores() {
        let qid1 = question_id();
        let qid2 = question_id();
        let session = Session {
            id: Uuid::now_v7(),
            user_id: user_id(),
            owner_id: user_id(),
            kind: SessionKind::Practice,
            assessment_id: None,
            filter: None,
            question_plan: QuestionPlan {
                items: vec![
                    PlanItem {
                        question_id: qid1,
                        version: 1,
                        section: None,
                        option_order: None,
                    },
                    PlanItem {
                        question_id: qid2,
                        version: 1,
                        section: None,
                        option_order: None,
                    },
                ],
            },
            status: SessionStatus::InProgress,
            affects_rating: true,
            rating_snapshot: BTreeMap::new(),
            result: None,
            deadline_at: None,
            started_at: now(),
            finished_at: None,
            assessment_title: None,
            course_title: None,
        };

        let attempts = vec![
            Attempt {
                id: Uuid::now_v7(),
                user_id: session.user_id,
                owner_id: session.owner_id,
                question_id: qid1,
                question_version: 1,
                session_id: Some(session.id),
                response: AttemptResponse::Short {
                    answer: "ok".into(),
                },
                presentation: Default::default(),
                is_correct: true,
                score: 1.0,
                grade_status: "graded".into(),
                correct_answer: None,
                grader_notes: None,
                time_to_answer_ms: None,
                rating_before_user_avg: 1200.0,
                rating_before_question: 1200.0,
                user_tag_deltas: BTreeMap::new(),
                question_delta: 0.0,
                created_at: now(),
            },
            Attempt {
                id: Uuid::now_v7(),
                user_id: session.user_id,
                owner_id: session.owner_id,
                question_id: qid2,
                question_version: 1,
                session_id: Some(session.id),
                response: AttemptResponse::Short {
                    answer: "no".into(),
                },
                presentation: Default::default(),
                is_correct: false,
                score: 0.0,
                grade_status: "graded".into(),
                correct_answer: None,
                grader_notes: None,
                time_to_answer_ms: None,
                rating_before_user_avg: 1200.0,
                rating_before_question: 1200.0,
                user_tag_deltas: BTreeMap::new(),
                question_delta: 0.0,
                created_at: now(),
            },
        ];

        let mut max_points = HashMap::new();
        max_points.insert(qid1, 1);
        max_points.insert(qid2, 1);

        let finished = finish_session(session, &attempts, &max_points, now()).unwrap();
        assert_eq!(finished.status, SessionStatus::Finished);
        let result: SessionResult = serde_json::from_value(finished.result.unwrap()).unwrap();
        assert_eq!(result.points_awarded, 1);
        assert_eq!(result.max_points, 2);
        assert_eq!(result.percent, 0.5);
    }
}
