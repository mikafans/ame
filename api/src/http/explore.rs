use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sqlx::{Arguments, Row, postgres::PgArguments};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{auth::extractor::AuthenticatedUser, domain::error::ApiError, http::AppState};

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ExploreQuery {
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub mode: Option<String>,
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub after: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResponse {
    pub items: Vec<serde_json::Value>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreFacetsResponse {
    pub tags: Vec<String>,
    pub counts: ExploreCounts,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExploreCounts {
    pub practice: i64,
    pub graded: i64,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/explore", get(explore))
        .route("/v1/explore/facets", get(explore_facets))
        .with_state(state)
}

fn encode_cursor(created_at: OffsetDateTime, id: Uuid) -> String {
    let raw = format!("{}_{}", created_at.unix_timestamp_nanos(), id);
    URL_SAFE_NO_PAD.encode(raw)
}

fn decode_cursor(cursor: &str) -> Option<(OffsetDateTime, Uuid)> {
    let decoded = URL_SAFE_NO_PAD.decode(cursor).ok()?;
    let raw = String::from_utf8(decoded).ok()?;
    let (nanos, id) = raw.split_once('_')?;
    let ts = OffsetDateTime::from_unix_timestamp_nanos(nanos.parse().ok()?).ok()?;
    Some((ts, Uuid::parse_str(id).ok()?))
}

/// GET /v1/explore — searchable, paginated assessment exploration.
#[utoipa::path(
    get,
    path = "/v1/explore",
    params(ExploreQuery),
    responses(
        (status = 200, description = "Assessment list", body = ExploreResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn explore(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(q): Query<ExploreQuery>,
) -> Result<Json<ExploreResponse>, ApiError> {
    let pool = &state.pool;
    let uid = auth.owner_id;
    let limit = q.limit.clamp(1, 200);

    // Build dynamic query
    // Strict isolation: only view own items (by owner_id)
    let mut sql = String::from(
        "SELECT id, title, status, mode, objectives, created_at, \
         ( \
             SELECT EXISTS( \
                 SELECT 1 FROM tb_sessions s \
                 WHERE s.assessment_id = tb_assessments.id AND s.user_id = $2 \
                   AND s.status = 'finished' \
             ) \
         ) AS completed \
         FROM tb_assessments \
         WHERE owner_id = $1 AND deleted_at IS NULL",
    );
    let mut args = PgArguments::default();
    args.add(uid)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
    args.add(auth.user.id)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
    let mut param_idx = 3; // $1 is owner_id, $2 is user_id

    if let Some(status) = q.kind {
        sql.push_str(&format!(" AND status = ${}", param_idx));
        args.add(status)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 1;
    }

    if let Some(mode) = q.mode {
        sql.push_str(&format!(" AND mode = ${}", param_idx));
        args.add(mode)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 1;
    }

    if let Some(tags_str) = q.tags {
        let tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !tags.is_empty() {
            sql.push_str(&format!(" AND objectives && ${}", param_idx));
            args.add(tags)
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
            param_idx += 1;
        }
    }

    if let Some(search) = q.search {
        sql.push_str(&format!(" AND title ILIKE ${}", param_idx));
        args.add(format!("%{}%", search))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 1;
    }

    if let Some(after) = q.after.as_deref().and_then(decode_cursor) {
        sql.push_str(&format!(
            " AND (created_at, id) < (${}, ${})",
            param_idx,
            param_idx + 1
        ));
        args.add(after.0)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        args.add(after.1)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        param_idx += 2;
    }

    sql.push_str(&format!(
        " ORDER BY created_at DESC, id DESC LIMIT ${}",
        param_idx
    ));
    args.add(limit)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    let rows = sqlx::query_with(&sql, args)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let created_at = row.get::<OffsetDateTime, _>("created_at");
            serde_json::json!({
                "id": row.get::<Uuid, _>("id"),
                "title": row.get::<String, _>("title"),
                "status": row.get::<String, _>("status"),
                "mode": row.get::<String, _>("mode"),
                "tags": row.get::<Vec<String>, _>("objectives"),
                "createdAt": created_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
                "completed": row.get::<bool, _>("completed"),
            })
        })
        .collect();

    let next_cursor = if items.len() as i64 == limit {
        rows.last().map(|last| {
            encode_cursor(
                last.get::<OffsetDateTime, _>("created_at"),
                last.get::<Uuid, _>("id"),
            )
        })
    } else {
        None
    };

    Ok(Json(ExploreResponse { items, next_cursor }))
}

/// GET /v1/explore/facets — facets for filtering.
#[utoipa::path(
    get,
    path = "/v1/explore/facets",
    responses(
        (status = 200, description = "Explore facets", body = ExploreFacetsResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "assessments"
)]
pub async fn explore_facets(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<ExploreFacetsResponse>, ApiError> {
    let pool = &state.pool;
    let uid = auth.owner_id;

    // 1. Try to load from cache
    let cache_key = format!("ame:cache:facets:{}", uid);
    if let Ok(mut conn) = state.valkey.get().await {
        use redis::AsyncCommands;
        if let Some(cached) = conn
            .get::<_, String>(&cache_key)
            .await
            .ok()
            .and_then(|val| serde_json::from_str::<ExploreFacetsResponse>(&val).ok())
        {
            return Ok(Json(cached));
        }
    }

    // 1. Get counts grouped by mode
    // Facets must mirror the list (which is active-only): counting drafts or
    // archived assessments here over-reports vs. what Explore actually shows.
    let count_rows = sqlx::query(
        "SELECT mode, count(*) as cnt
         FROM tb_assessments
         WHERE owner_id = $1 AND deleted_at IS NULL AND status = 'active'
         GROUP BY mode",
    )
    .bind(uid)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let mut practice = 0;
    let mut graded = 0;
    for row in count_rows {
        let mode = row.get::<String, _>("mode");
        let cnt = row.get::<i64, _>("cnt");
        if mode == "practice" {
            practice = cnt;
        } else if mode == "graded" {
            graded = cnt;
        }
    }

    // 2. Get distinct tags (objectives) capped at 200
    let tag_rows = sqlx::query(
        "SELECT DISTINCT unnest(objectives) AS t
         FROM tb_assessments
         WHERE owner_id = $1 AND deleted_at IS NULL AND status = 'active'
         ORDER BY t
         LIMIT 200",
    )
    .bind(uid)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let tags: Vec<String> = tag_rows
        .iter()
        .map(|row| row.get::<String, _>("t"))
        .collect();

    let response = ExploreFacetsResponse {
        tags,
        counts: ExploreCounts { practice, graded },
    };

    // 3. Cache the response in Valkey with a short TTL (10 seconds)
    if let Ok(mut conn) = state.valkey.get().await {
        use redis::AsyncCommands;
        if let Ok(serialized) = serde_json::to_string(&response) {
            let _: Result<(), _> = conn.set_ex(&cache_key, serialized, 10).await;
        }
    }

    Ok(Json(response))
}
