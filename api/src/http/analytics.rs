//! Owner-scoped, event-derived learner analytics.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::{Date, OffsetDateTime};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::error::{ApiError, FieldError},
    http::AppState,
};

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query, rename_all = "camelCase")]
pub struct AnalyticsQuery {
    pub timezone: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RatioMetric {
    pub value: Option<f32>,
    pub numerator: f32,
    pub denominator: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreakMetric {
    pub current_days: u32,
    pub best_days: u32,
    pub qualifying_days: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimeMetric {
    pub seconds: u64,
    pub finished_sessions: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MasteryPoint {
    pub objective_id: Uuid,
    pub value: f32,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub occurred_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewHistoryPoint {
    pub review_item_id: Uuid,
    pub rating: String,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub reviewed_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub due_before: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String, format = DateTime)]
    pub due_after: OffsetDateTime,
    pub interval_days: u32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LearnerAnalyticsResponse {
    pub journey_id: Uuid,
    pub timezone: String,
    pub privacy_boundary: String,
    pub streak: StreakMetric,
    pub completion_rate: RatioMetric,
    pub average_score: RatioMetric,
    pub attempts: u32,
    pub time_spent: TimeMetric,
    pub mastery_trend: Vec<MasteryPoint>,
    pub review_history: Vec<ReviewHistoryPoint>,
    pub definitions: Vec<String>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/learning/journeys/{id}/analytics", get(get_analytics))
        .with_state(state)
}

#[utoipa::path(get, path = "/api/v1/learning/journeys/{id}/analytics", params(("id" = Uuid, Path), AnalyticsQuery), responses((status = 200, body = LearnerAnalyticsResponse)), security(("bearer" = [])), tag = "analytics")]
pub async fn get_analytics(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(journey_id): Path<Uuid>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<LearnerAnalyticsResponse>, ApiError> {
    query
        .timezone
        .parse::<Tz>()
        .map_err(|_| validation("timezone", "must be an IANA timezone name"))?;
    let owns_journey = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM tb_learning_journeys
            WHERE id = $1 AND subject_user_id = $2
        )",
    )
    .bind(journey_id)
    .bind(auth.owner_id())
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    if !owns_journey {
        return Err(ApiError::NotFound {
            resource: "learning journey",
        });
    }

    let activity_counts = sqlx::query(
        "SELECT count(*)::bigint AS total,
                count(*) FILTER (WHERE status = 'completed')::bigint AS completed
         FROM tb_activities WHERE journey_id = $1 AND subject_user_id = $2",
    )
    .bind(journey_id)
    .bind(auth.owner_id())
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    let total_activities = activity_counts.get::<i64, _>("total") as u32;
    let completed_activities = activity_counts.get::<i64, _>("completed") as u32;

    let scores = sqlx::query(
        "SELECT COALESCE(sum(value), 0)::float4 AS score_sum, count(*)::bigint AS scored
         FROM (
            SELECT (a.score / NULLIF(a.max_score, 0))::float4 AS value
            FROM tb_attempts a
            JOIN tb_learning_sessions s ON s.id = a.learning_session_id
            WHERE a.subject_user_id = $1 AND s.journey_id = $2
              AND a.status = 'graded' AND a.review_status IN ('not_required', 'complete')
              AND a.score IS NOT NULL AND a.max_score > 0
            UNION ALL
            SELECT t.score::float4
            FROM tb_task_submissions t
            WHERE t.subject_user_id = $1 AND t.journey_id = $2
              AND t.status = 'reviewed' AND t.review_status = 'complete'
              AND t.score IS NOT NULL
         ) scored_outcomes",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    let score_sum = scores.get::<f32, _>("score_sum");
    let scored_outcomes = scores.get::<i64, _>("scored") as u32;

    let attempts = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM tb_attempts a
         JOIN tb_learning_sessions s ON s.id = a.learning_session_id
         WHERE a.subject_user_id = $1 AND s.journey_id = $2
           AND a.status IN ('submitted', 'graded')",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)? as u32;

    let time = sqlx::query(
        "SELECT COALESCE(sum(EXTRACT(EPOCH FROM (finished_at - started_at))), 0)::bigint AS seconds,
                count(*)::bigint AS sessions
         FROM tb_learning_sessions
         WHERE subject_user_id = $1 AND journey_id = $2
           AND status = 'finished' AND finished_at >= started_at",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;

    let day_rows = sqlx::query(
        "SELECT DISTINCT (created_at AT TIME ZONE $3)::date AS day
         FROM tb_streak_events
         WHERE subject_user_id = $1 AND journey_id = $2
         ORDER BY day",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .bind(&query.timezone)
    .fetch_all(&state.pool)
    .await
    .map_err(internal)?;
    let days = day_rows
        .iter()
        .map(|row| row.get::<Date, _>("day"))
        .collect::<Vec<_>>();
    let today = sqlx::query_scalar::<_, Date>("SELECT (now() AT TIME ZONE $1)::date")
        .bind(&query.timezone)
        .fetch_one(&state.pool)
        .await
        .map_err(internal)?;
    let (current_days, best_days) = streaks(&days, today);

    let mastery_trend = sqlx::query(
        "SELECT objective_id, value::float4 AS value, created_at
         FROM tb_mastery_evidence
         WHERE subject_user_id = $1 AND journey_id = $2
         ORDER BY created_at, id",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal)?
    .iter()
    .map(|row| MasteryPoint {
        objective_id: row.get("objective_id"),
        value: row.get("value"),
        occurred_at: row.get("created_at"),
    })
    .collect();

    let review_history = sqlx::query(
        "SELECT review_item_id, rating, reviewed_at, due_before, due_after, interval_days
         FROM tb_review_events
         WHERE subject_user_id = $1 AND journey_id = $2
         ORDER BY reviewed_at, id",
    )
    .bind(auth.owner_id())
    .bind(journey_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal)?
    .iter()
    .map(|row| ReviewHistoryPoint {
        review_item_id: row.get("review_item_id"),
        rating: row.get("rating"),
        reviewed_at: row.get("reviewed_at"),
        due_before: row.get("due_before"),
        due_after: row.get("due_after"),
        interval_days: row.get::<i32, _>("interval_days") as u32,
    })
    .collect();

    Ok(Json(LearnerAnalyticsResponse {
        journey_id,
        timezone: query.timezone,
        privacy_boundary: "authenticated_owner_only".into(),
        streak: StreakMetric {
            current_days,
            best_days,
            qualifying_days: days.len() as u32,
        },
        completion_rate: ratio(completed_activities as f32, total_activities),
        average_score: ratio(score_sum, scored_outcomes),
        attempts,
        time_spent: TimeMetric {
            seconds: time.get::<i64, _>("seconds").max(0) as u64,
            finished_sessions: time.get::<i64, _>("sessions") as u32,
        },
        mastery_trend,
        review_history,
        definitions: vec![
            "Streaks use distinct qualifying-event dates converted to the requested IANA timezone; current includes a run ending today or yesterday.".into(),
            "Completion is distinct completed curriculum activities divided by all journey activities.".into(),
            "Average score is the sum of normalized graded assessments and completed reviewed task scores divided by those scored outcomes; pending reviews are excluded.".into(),
            "Attempts count submitted or graded assessment attempts; repeated answer writes do not add attempts.".into(),
            "Time spent sums non-negative finished-session durations; open or abandoned sessions are excluded.".into(),
            "Mastery trend contains recorded evidence events without interpolation; review history contains immutable rating events.".into(),
        ],
    }))
}

fn ratio(numerator: f32, denominator: u32) -> RatioMetric {
    RatioMetric {
        value: (denominator > 0).then_some(numerator / denominator as f32),
        numerator,
        denominator,
    }
}

fn streaks(days: &[Date], today: Date) -> (u32, u32) {
    if days.is_empty() {
        return (0, 0);
    }
    let mut best = 1;
    let mut run = 1;
    for window in days.windows(2) {
        if (window[1] - window[0]).whole_days() == 1 {
            run += 1;
            best = best.max(run);
        } else {
            run = 1;
        }
    }
    let last = days[days.len() - 1];
    if !matches!((today - last).whole_days(), 0 | 1) {
        return (0, best);
    }
    let mut current = 1;
    for window in days.windows(2).rev() {
        if (window[1] - window[0]).whole_days() == 1 {
            current += 1;
        } else {
            break;
        }
    }
    (current, best)
}

fn validation(field: &str, message: &str) -> ApiError {
    ApiError::Validation(vec![FieldError {
        field: field.into(),
        message: message.into(),
    }])
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn streaks_are_deterministic_across_gaps_and_yesterday_boundary() {
        let day = |number| Date::from_calendar_date(2026, Month::July, number).unwrap();
        let days = [day(20), day(21), day(23), day(24), day(25)];
        assert_eq!(streaks(&days, day(26)), (3, 3));
        assert_eq!(streaks(&days, day(28)), (0, 3));
        assert_eq!(streaks(&[], day(28)), (0, 0));
    }

    #[test]
    fn missing_denominator_never_invents_a_rate() {
        let metric = ratio(0.0, 0);
        assert_eq!(metric.value, None);
        assert_eq!(metric.denominator, 0);
    }
}
