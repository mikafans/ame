//! Stats routes: GET /v1/quizzes/{id}/stats, GET /v1/exams/{id}/stats, GET /v1/me/stats.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        scope::{RequireScope, ScopeConstraint},
    },
    domain::{error::ApiError, user::Scope},
    http::AppState,
};

pub struct StatsReadScope;
impl ScopeConstraint for StatsReadScope {
    const SCOPE: Scope = Scope::StatsRead;
}

// ── quiz stats ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuizStatsParams {
    pub cohort_id: Option<Uuid>,
    pub window: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuizStatsResponse {
    pub avg: f64,
    pub median: f64,
    pub distribution: Vec<DistributionBucket>,
    pub items: Vec<ItemStats>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DistributionBucket {
    pub bucket: i32,
    pub count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ItemStats {
    pub question_id: Uuid,
    pub correct_rate: f64,
    pub avg_time_ms: f64,
    pub discrimination_idx: Option<f64>,
}

#[utoipa::path(
    get,
    path = "/v1/quizzes/{id}/stats",
    params(
        ("id" = Uuid, Path, description = "Quiz id"),
        ("cohortId" = Option<Uuid>, Query, description = "Filter by cohort (future)"),
        ("window" = Option<String>, Query, description = "'last30d' | 'all'"),
    ),
    responses(
        (status = 200, description = "Quiz stats", body = QuizStatsResponse),
        (status = 403, description = "Requires stats.read scope"),
        (status = 404, description = "Quiz not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn quiz_stats(
    State(state): State<AppState>,
    _user: RequireScope<StatsReadScope>,
    Path(id): Path<Uuid>,
    Query(params): Query<QuizStatsParams>,
) -> Result<Json<QuizStatsResponse>, ApiError> {
    // Verify quiz exists
    let exists: bool = sqlx::query_scalar("SELECT exists(SELECT 1 FROM quizzes WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    if !exists {
        return Err(ApiError::NotFound { resource: "quiz" });
    }

    let window_filter = window_sql_filter(params.window.as_deref());

    // avg + median of session percent scores
    let agg_row = sqlx::query(&format!(
        "SELECT
            coalesce(avg((result->>'percent')::double precision), 0) as avg,
            coalesce(percentile_cont(0.5) within group (order by (result->>'percent')::double precision), 0) as median
         FROM sessions
         WHERE quiz_id = $1
           AND status = 'finished'
           AND result is not null
           {window_filter}"
    ))
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let avg: f64 = agg_row.get("avg");
    let median: f64 = agg_row.get("median");

    // distribution: 10 buckets 0-9 (0%–9%, 10%–19%, …, 90%–100%)
    let dist_rows = sqlx::query(&format!(
        "SELECT
            floor((result->>'percent')::double precision * 10)::int as bucket,
            count(*) as count
         FROM sessions
         WHERE quiz_id = $1
           AND status = 'finished'
           AND result is not null
           {window_filter}
         GROUP BY 1
         ORDER BY 1"
    ))
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let distribution = dist_rows
        .iter()
        .map(|r| DistributionBucket {
            bucket: r.get::<i32, _>("bucket").min(9),
            count: r.get("count"),
        })
        .collect();

    // Per-question stats: correctRate, avgTimeMs, discriminationIdx
    // discriminationIdx uses point-biserial: correlation between item correctness and session score.
    let item_rows = sqlx::query(&format!(
        "WITH session_scores AS (
            SELECT s.id as session_id,
                   (s.result->>'percent')::double precision as pct
            FROM sessions s
            WHERE s.quiz_id = $1
              AND s.status = 'finished'
              AND s.result is not null
              {window_filter}
         ),
         item_stats AS (
            SELECT
                a.question_id,
                avg(a.is_correct::int::double precision)          as correct_rate,
                avg(a.time_to_answer_ms::double precision)        as avg_time_ms,
                -- point-biserial: (M_p - M_t) / S_t * sqrt(p * (1-p))
                -- only defined when there is variance in session scores
                CASE
                    WHEN stddev_pop(ss.pct) > 0
                         AND avg(a.is_correct::int::double precision) BETWEEN 0.01 AND 0.99
                    THEN
                        (avg(CASE WHEN a.is_correct THEN ss.pct END) - avg(ss.pct))
                        / stddev_pop(ss.pct)
                        * sqrt(
                            avg(a.is_correct::int::double precision)
                            * (1 - avg(a.is_correct::int::double precision))
                          )
                    ELSE NULL
                END as discrimination_idx
            FROM attempts a
            JOIN session_scores ss ON a.session_id = ss.session_id
            GROUP BY a.question_id
         )
         SELECT question_id, correct_rate, avg_time_ms, discrimination_idx
         FROM item_stats
         ORDER BY question_id"
    ))
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let items = item_rows
        .iter()
        .map(|r| ItemStats {
            question_id: r.get("question_id"),
            correct_rate: r.get::<Option<f64>, _>("correct_rate").unwrap_or(0.0),
            avg_time_ms: r.get::<Option<f64>, _>("avg_time_ms").unwrap_or(0.0),
            discrimination_idx: r.get("discrimination_idx"),
        })
        .collect();

    Ok(Json(QuizStatsResponse {
        avg,
        median,
        distribution,
        items,
    }))
}

// ── exam stats ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExamStatsResponse {
    pub pass_rate: f64,
    pub section_avgs: Vec<SectionAvg>,
    pub time_p50: Option<f64>,
    pub time_p95: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SectionAvg {
    pub section_id: Uuid,
    pub title: String,
    pub avg_score: f64,
}

#[utoipa::path(
    get,
    path = "/v1/exams/{id}/stats",
    params(("id" = Uuid, Path, description = "Exam id")),
    responses(
        (status = 200, description = "Exam stats", body = ExamStatsResponse),
        (status = 403, description = "Requires stats.read scope"),
        (status = 404, description = "Exam not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn exam_stats(
    State(state): State<AppState>,
    _user: RequireScope<StatsReadScope>,
    Path(id): Path<Uuid>,
) -> Result<Json<ExamStatsResponse>, ApiError> {
    // Verify exam exists and get passing_points
    let exam_row = sqlx::query("SELECT passing_points, total_points FROM exams WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::NotFound { resource: "exam" })?;

    let passing_points: Option<i32> = exam_row.get("passing_points");
    let total_points: i32 = exam_row.get("total_points");

    // pass rate: fraction of finished sessions where points_awarded >= passing_points
    let pass_rate: f64 = if let Some(pp) = passing_points {
        sqlx::query_scalar(
            "SELECT coalesce(
                avg(((result->>'points_awarded')::int >= $2)::int::double precision),
                0
             )
             FROM sessions
             WHERE exam_id = $1 AND status = 'finished' AND result is not null",
        )
        .bind(id)
        .bind(pp)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
    } else if total_points > 0 {
        // default pass threshold: >= 60%
        sqlx::query_scalar(
            "SELECT coalesce(
                avg(((result->>'points_awarded')::double precision
                     / $2::double precision >= 0.6)::int::double precision),
                0
             )
             FROM sessions
             WHERE exam_id = $1 AND status = 'finished' AND result is not null",
        )
        .bind(id)
        .bind(total_points)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
    } else {
        0.0
    };

    // section avgs: for each exam_section, avg score of questions in that section
    let section_rows = sqlx::query(
        "SELECT es.id, es.title,
            coalesce(
                avg(a.score),
                0
            ) as avg_score
         FROM exam_sections es
         LEFT JOIN sessions s ON s.exam_id = $1 AND s.status = 'finished'
         LEFT JOIN attempts a
            ON a.session_id = s.id
            AND a.question_id = ANY(es.question_ids)
         WHERE es.exam_id = $1
         GROUP BY es.id, es.title, es.order_index
         ORDER BY es.order_index",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let section_avgs = section_rows
        .iter()
        .map(|r| SectionAvg {
            section_id: r.get("id"),
            title: r.get("title"),
            avg_score: r.get::<f64, _>("avg_score"),
        })
        .collect();

    // session duration percentiles (finished_at - started_at in ms)
    let time_row = sqlx::query(
        "SELECT
            percentile_cont(0.5) within group
                (order by extract(epoch from (finished_at - started_at)) * 1000) as p50,
            percentile_cont(0.95) within group
                (order by extract(epoch from (finished_at - started_at)) * 1000) as p95
         FROM sessions
         WHERE exam_id = $1 AND status = 'finished' AND finished_at is not null",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let time_p50: Option<f64> = time_row.get("p50");
    let time_p95: Option<f64> = time_row.get("p95");

    Ok(Json(ExamStatsResponse {
        pass_rate,
        section_avgs,
        time_p50,
        time_p95,
    }))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn window_sql_filter(window: Option<&str>) -> String {
    match window {
        Some("last30d") => "AND started_at > now() - interval '30 days'".to_string(),
        _ => String::new(),
    }
}

// ── me stats ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct MeStatsResponse {
    pub avg_score: f64,
    pub avg_score_delta: f64,
    pub attempts_total: i64,
    pub attempts_this_week: i64,
    pub hours_spent: f64,
    pub hours_spent_delta: f64,
    pub current_streak: i64,
    pub best_streak: i64,
    pub mastered_topics: i64,
    pub mastered_topics_total: i64,
    pub mastered_topics_delta_since: String,
}

#[utoipa::path(
    get,
    path = "/v1/me/stats",
    responses(
        (status = 200, description = "User stats summary", body = MeStatsResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn me_stats(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<MeStatsResponse>, ApiError> {
    let uid = user.user.id;

    // attempts total and this week
    let attempts_row = sqlx::query(
        "SELECT COUNT(*) AS total, \
                COUNT(*) FILTER (WHERE created_at > now() - interval '7 days') AS this_week \
         FROM attempts WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    let attempts_total: i64 = attempts_row.get("total");
    let attempts_this_week: i64 = attempts_row.get("this_week");

    // avg score from finished sessions (result->points_awarded / result->max_points)
    let score_row = sqlx::query(
        "SELECT \
            AVG(CASE WHEN (result->>'max_points')::float > 0 \
                THEN (result->>'points_awarded')::float / (result->>'max_points')::float \
                ELSE NULL END) FILTER (WHERE finished_at > now() - interval '28 days') AS avg_recent, \
            AVG(CASE WHEN (result->>'max_points')::float > 0 \
                THEN (result->>'points_awarded')::float / (result->>'max_points')::float \
                ELSE NULL END) FILTER (WHERE finished_at BETWEEN now() - interval '56 days' AND now() - interval '28 days') AS avg_prior \
         FROM sessions WHERE user_id = $1 AND status = 'finished' AND result IS NOT NULL",
    )
    .bind(uid)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    let avg_score: f64 = score_row.try_get::<f64, _>("avg_recent").unwrap_or(0.0);
    let avg_prior: f64 = score_row.try_get::<f64, _>("avg_prior").unwrap_or(0.0);
    let avg_score_delta = avg_score - avg_prior;

    // hours spent
    let hours_row = sqlx::query(
        "SELECT \
            COALESCE(SUM(EXTRACT(EPOCH FROM (finished_at - started_at)) / 3600.0) \
                FILTER (WHERE finished_at > now() - interval '28 days'), 0)::float8 AS hours_recent, \
            COALESCE(SUM(EXTRACT(EPOCH FROM (finished_at - started_at)) / 3600.0) \
                FILTER (WHERE finished_at BETWEEN now() - interval '56 days' AND now() - interval '28 days'), 0)::float8 AS hours_prior \
         FROM sessions WHERE user_id = $1 AND status = 'finished'",
    )
    .bind(uid)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    let hours_spent: f64 = hours_row.get("hours_recent");
    let hours_prior: f64 = hours_row.get("hours_prior");
    let hours_spent_delta = hours_spent - hours_prior;

    // streak: consecutive days with at least one attempt, counting back from today
    let streak_row = sqlx::query(
        "WITH daily AS ( \
            SELECT DISTINCT date_trunc('day', created_at AT TIME ZONE 'UTC')::date AS day \
            FROM attempts WHERE user_id = $1 \
        ), \
        gaps AS ( \
            SELECT day, ROW_NUMBER() OVER (ORDER BY day) - \
                   (day - '2000-01-01'::date) AS grp \
            FROM daily \
        ), \
        runs AS ( \
            SELECT grp, COUNT(*) AS run_len, MAX(day) AS last_day \
            FROM gaps GROUP BY grp \
        ) \
        SELECT \
            COALESCE((SELECT run_len FROM runs WHERE last_day >= now()::date - 1 ORDER BY last_day DESC LIMIT 1), 0)::bigint AS current_streak, \
            COALESCE(MAX(run_len), 0)::bigint AS best_streak \
        FROM runs",
    )
    .bind(uid)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    let current_streak: i64 = streak_row.get("current_streak");
    let best_streak: i64 = streak_row.get("best_streak");

    // mastered topics (rating >= 1400 means above starting 1200 by meaningful margin)
    let mastery_row = sqlx::query(
        "SELECT \
            COUNT(*) FILTER (WHERE rating >= 1400) AS mastered, \
            COUNT(*) AS total \
         FROM user_tag_ratings WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_one(&state.pool)
    .await
    .map_err(internal)?;
    let mastered_topics: i64 = mastery_row.get("mastered");
    let mastered_topics_total: i64 = mastery_row.get("total");

    Ok(Json(MeStatsResponse {
        avg_score,
        avg_score_delta,
        attempts_total,
        attempts_this_week,
        hours_spent,
        hours_spent_delta,
        current_streak,
        best_streak,
        mastered_topics,
        mastered_topics_total,
        mastered_topics_delta_since: "last 4 weeks".to_string(),
    }))
}

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/quizzes/{id}/stats", get(quiz_stats))
        .route("/v1/exams/{id}/stats", get(exam_stats))
        .route("/v1/me/stats", get(me_stats))
        .with_state(state)
}
