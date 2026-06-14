//! Stats routes: GET /v1/assessments/{id}/stats, GET /v1/me/stats.

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

// ── assessment stats ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentStatsParams {
    pub cohort_id: Option<Uuid>,
    pub window: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentStatsResponse {
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
    path = "/v1/assessments/{id}/stats",
    params(
        ("id" = Uuid, Path, description = "Assessment ID"),
        ("window" = Option<String>, Query, description = "'last30d' | 'all'"),
    ),
    responses(
        (status = 200, description = "Assessment stats", body = AssessmentStatsResponse),
        (status = 403, description = "Requires stats.read scope"),
        (status = 404, description = "Assessment not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn assessment_stats(
    State(state): State<AppState>,
    user: RequireScope<StatsReadScope>,
    Path(id): Path<Uuid>,
    Query(params): Query<AssessmentStatsParams>,
) -> Result<Json<AssessmentStatsResponse>, ApiError> {
    // Verify assessment exists and is owner-scoped
    let exists: bool = sqlx::query_scalar(
        "SELECT exists(SELECT 1 FROM tb_assessments WHERE id = $1 \
             AND owner_id = $2)",
    )
    .bind(id)
    .bind(user.0.owner_id())
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;
    if !exists {
        return Err(ApiError::NotFound {
            resource: "assessment",
        });
    }

    let window_filter = window_sql_filter(params.window.as_deref());

    // avg + median + distribution in a single scan over the filtered sessions.
    // GROUPING SETS ((), (bucket)) yields one grand-total row (bucket NULL) with
    // the overall avg/median, plus one row per score bucket with its count.
    let agg_dist_rows = sqlx::query(&format!(
        "WITH filtered AS (
            SELECT
                (result->>'percent')::double precision AS pct,
                least(floor((result->>'percent')::double precision * 10)::int, 9) AS bucket
            FROM tb_sessions
            WHERE assessment_id = $1
              AND status = 'finished'
              AND result is not null
              {window_filter}
        )
        SELECT
            bucket,
            coalesce(avg(pct), 0) AS avg,
            coalesce(percentile_cont(0.5) within group (order by pct), 0) AS median,
            count(*) AS count
         FROM filtered
         GROUP BY GROUPING SETS ((), (bucket))
         ORDER BY bucket NULLS FIRST"
    ))
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let mut avg = 0.0;
    let mut median = 0.0;
    let mut distribution = Vec::new();
    for row in &agg_dist_rows {
        match row.get::<Option<i32>, _>("bucket") {
            // grand-total grouping set: overall avg/median
            None => {
                avg = row.get("avg");
                median = row.get("median");
            }
            Some(bucket) => distribution.push(DistributionBucket {
                bucket,
                count: row.get("count"),
            }),
        }
    }

    // Per-question stats
    let item_rows = sqlx::query(&format!(
        "WITH session_scores AS (
            SELECT s.id as session_id,
                   (s.result->>'percent')::double precision as pct
            FROM tb_sessions s
            WHERE s.assessment_id = $1
              AND s.status = 'finished'
              AND s.result is not null
              {window_filter}
         ),
         item_stats AS (
            SELECT
                a.question_id,
                avg(a.is_correct::int::double precision)          as correct_rate,
                avg(a.time_to_answer_ms::double precision)        as avg_time_ms,
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
            FROM tb_attempts a
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

    Ok(Json(AssessmentStatsResponse {
        avg,
        median,
        distribution,
        items,
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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MeStatsQuery {
    pub window: Option<String>,
}

fn window_days(window: Option<&str>) -> Option<i32> {
    match window {
        Some("last30d") => Some(30),
        Some("last90d") => Some(90),
        _ => None,
    }
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_active_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_graded_attempts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_curated_assessments: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/v1/me/stats",
    params(MeStatsQuery),
    responses(
        (status = 200, description = "User stats summary", body = MeStatsResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn me_stats(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(q): Query<MeStatsQuery>,
) -> Result<Json<MeStatsResponse>, ApiError> {
    // Scope to the owner, not the calling identity: an agent sub-account shares
    // its owner's learning record, so an agent token must read the owner's
    // attempts/sessions/ratings. For a human token owner_id == user.id, so this
    // leaves their view unchanged.
    let uid = user.owner_id;
    let days = window_days(q.window.as_deref());

    // These eight read-only stat queries are independent and share no state, so
    // run them concurrently — each future acquires its own pooled connection.
    let attempts_fut = async {
        let row = sqlx::query(
            "SELECT COUNT(*) AS total, \
                    COUNT(*) FILTER (WHERE created_at > now() - interval '7 days') AS this_week \
             FROM tb_attempts \
             WHERE owner_id = $1",
        )
        .bind(uid)
        .fetch_one(&state.pool)
        .await
        .map_err(internal)?;
        Ok::<_, ApiError>((row.get::<i64, _>("total"), row.get::<i64, _>("this_week")))
    };

    let score_fut = async {
        let row = sqlx::query(
            "SELECT \
                AVG(CASE WHEN (result->>'max_points')::float > 0 \
                    THEN (result->>'points_awarded')::float / (result->>'max_points')::float \
                    ELSE NULL END) FILTER (WHERE finished_at > now() - interval '28 days') AS avg_recent, \
                AVG(CASE WHEN (result->>'max_points')::float > 0 \
                    THEN (result->>'points_awarded')::float / (result->>'max_points')::float \
                    ELSE NULL END) FILTER (WHERE finished_at BETWEEN now() - interval '56 days' AND now() - interval '28 days') AS avg_prior \
             FROM tb_sessions \
             WHERE owner_id = $1 \
               AND status = 'finished' AND result IS NOT NULL",
        )
        .bind(uid)
        .fetch_one(&state.pool)
        .await
        .map_err(internal)?;
        Ok::<_, ApiError>((
            row.try_get::<f64, _>("avg_recent").unwrap_or(0.0),
            row.try_get::<f64, _>("avg_prior").unwrap_or(0.0),
        ))
    };

    let hours_fut = async {
        let row = sqlx::query(
            "SELECT \
                COALESCE(SUM(EXTRACT(EPOCH FROM (finished_at - started_at)) / 3600.0) \
                    FILTER (WHERE $2::int IS NULL \
                        OR finished_at > now() - make_interval(days => $2::int)), 0)::float8 AS hours_recent, \
                COALESCE(SUM(EXTRACT(EPOCH FROM (finished_at - started_at)) / 3600.0) \
                    FILTER (WHERE $2::int IS NOT NULL \
                        AND finished_at BETWEEN now() - make_interval(days => $2::int * 2) \
                                             AND now() - make_interval(days => $2::int)), 0)::float8 AS hours_prior \
             FROM tb_sessions \
             WHERE owner_id = $1 \
               AND status = 'finished'",
        )
        .bind(uid)
        .bind(days)
        .fetch_one(&state.pool)
        .await
        .map_err(internal)?;
        Ok::<_, ApiError>((
            row.get::<f64, _>("hours_recent"),
            row.get::<f64, _>("hours_prior"),
        ))
    };

    let streak_fut = async {
        let row = sqlx::query(
            "WITH daily AS ( \
                SELECT DISTINCT date_trunc('day', created_at AT TIME ZONE 'UTC')::date AS day \
                FROM tb_attempts \
                WHERE owner_id = $1 \
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
        Ok::<_, ApiError>((
            row.get::<i64, _>("current_streak"),
            row.get::<i64, _>("best_streak"),
        ))
    };

    let mastery_fut = async {
        let row = sqlx::query(
            "SELECT \
                COUNT(*) FILTER (WHERE rating >= 1400) AS mastered, \
                COUNT(*) AS total \
             FROM tb_user_tag_ratings WHERE user_id = $1",
        )
        .bind(uid)
        .fetch_one(&state.pool)
        .await
        .map_err(internal)?;
        Ok::<_, ApiError>((row.get::<i64, _>("mastered"), row.get::<i64, _>("total")))
    };

    let agent_active_fut = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tb_agents WHERE owner_user_id = $1 AND deactivated_at IS NULL",
    )
    .bind(uid)
    .fetch_one(&state.pool);

    // Fold the former get_agent_ids round trip into a correlated subquery.
    let agent_graded_fut = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tb_activity_log \
         WHERE tool_name = 'attempt.grade' AND status = 200 \
           AND actor_id IN (SELECT id FROM tb_agents WHERE owner_user_id = $1)",
    )
    .bind(uid)
    .fetch_one(&state.pool);

    let agent_curated_fut = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tb_assessments WHERE method = 'agent' AND owner_id = $1 AND deleted_at IS NULL",
    )
    .bind(uid)
    .fetch_one(&state.pool);

    let (
        (attempts_total, attempts_this_week),
        (avg_score, avg_prior),
        (hours_spent, hours_prior),
        (current_streak, best_streak),
        (mastered_topics, mastered_topics_total),
        agent_active_count,
        agent_graded_attempts,
        agent_curated_assessments,
    ) = tokio::try_join!(
        attempts_fut,
        score_fut,
        hours_fut,
        streak_fut,
        mastery_fut,
        async { agent_active_fut.await.map_err(internal) },
        async { agent_graded_fut.await.map_err(internal) },
        async { agent_curated_fut.await.map_err(internal) },
    )?;

    let avg_score_delta = avg_score - avg_prior;
    let hours_spent_delta = hours_spent - hours_prior;

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
        agent_active_count: Some(agent_active_count),
        agent_graded_attempts: Some(agent_graded_attempts),
        agent_curated_assessments: Some(agent_curated_assessments),
    }))
}

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/assessments/{id}/stats", get(assessment_stats))
        .route("/v1/me/stats", get(me_stats))
        .with_state(state)
}
