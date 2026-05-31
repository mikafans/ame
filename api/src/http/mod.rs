use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use axum_prometheus::PrometheusMetricLayer;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_governor::GovernorLayer;
use tower_governor::errors::GovernorError;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Shared HTTP state. Cloned per request by Axum, so anything added here must
/// be cheap to clone (PgPool is internally an `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}
#[derive(Clone)]
struct TokenKeyExtractor;
impl KeyExtractor for TokenKeyExtractor {
    type Key = String;
    fn extract<T>(&self, req: &axum::http::Request<T>) -> Result<Self::Key, GovernorError> {
        // 1. Try Authorization header (agents, direct API callers)
        if let Some(auth) = req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
        {
            return Ok(auth.to_string());
        }

        // 2. Try ame_token cookie (browser clients)
        if let Some(cookie_key) = req
            .headers()
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|c| {
                    let c = c.trim();
                    c.strip_prefix("ame_token=").map(|v| v.to_owned())
                })
            })
        {
            return Ok(cookie_key);
        }

        // 3. Fall back to peer IP so unauthenticated requests still get rate-limited
        PeerIpKeyExtractor.extract(req).map(|ip| ip.to_string())
    }
}

pub mod activity;
pub mod admin;
pub mod agents;
pub mod assessments;
pub mod auth;
pub mod export;
pub mod idempotency;
pub mod me;
pub mod messages;
pub mod openapi;
pub mod plans;
pub mod questions;
pub mod quota;
pub mod sessions;
pub mod stats;
pub mod tags;

/// Build the production HTTP router.
///
/// `/healthz` is intentionally outside any middleware so a misconfigured DB
/// cannot mask a healthy process. Question/tag routes live under their own
/// modules and bring their own scope guards + idempotency wiring.
pub fn router(pool: PgPool) -> Router {
    let state = AppState { pool };

    // Routes wrapped with activity-log middleware (records all authenticated calls)
    let global_burst = env_u32("AME_GLOBAL_RATELIMIT_BURST", 100);
    let global_period_secs = env_u64("AME_GLOBAL_RATELIMIT_PERIOD_SECS", 1);
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(global_period_secs)
            .burst_size(global_burst)
            .key_extractor(TokenKeyExtractor)
            .finish()
            .expect("valid rate-limit config"),
    );

    let export_governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(60) // Refill rate: 1 per 60s
            .burst_size(1) // Tighter burst size of 1
            .key_extractor(TokenKeyExtractor)
            .finish()
            .expect("valid export rate-limit config"),
    );

    let logged = Router::new()
        .merge(tags::router(state.clone()))
        .merge(questions::router(state.clone()))
        .merge(sessions::router(state.clone()))
        .merge(plans::router(state.clone()))
        .merge(assessments::router(state.clone()))
        .merge(stats::router(state.clone()))
        .merge(messages::router(state.clone()))
        .merge(me::router(state.clone()))
        .merge(admin::router(state.clone()))
        .merge(agents::logged_router(state.clone()))
        .route(
            "/v1/me/export",
            axum::routing::get(export::export_data).layer(GovernorLayer {
                config: export_governor_conf,
            }),
        )
        .layer(GovernorLayer {
            config: governor_conf,
        })
        .layer(middleware::from_fn_with_state(
            state.clone(),
            activity::activity_log_middleware,
        ));

    // Public, rate-limited endpoints. Credential-stuffing surface: limit
    // harder than the rest of the API.
    //
    // Defaults: 10 req burst, refill 1 per 2s (≈30 req/min sustained per IP).
    // Override with AME_RATELIMIT_BURST and AME_RATELIMIT_PERIOD_SECS.
    let burst = env_u32("AME_RATELIMIT_BURST", 10);
    let period_secs = env_u64("AME_RATELIMIT_PERIOD_SECS", 2);
    let public_governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(period_secs)
            .burst_size(burst)
            .key_extractor(PeerIpKeyExtractor)
            .finish()
            .expect("valid rate-limit config"),
    );
    let public_limited = Router::new()
        .route("/v1/auth/register", post(auth::register))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/logout", post(auth::logout))
        .layer(GovernorLayer {
            config: public_governor_conf,
        })
        .with_state(state.clone());

    let public_unlimited = Router::new()
        .merge(agents::public_router(state.clone()))
        .with_state(state.clone());

    let cors = build_cors_layer();

    // Tag every request with an x-request-id (incoming value is reused if
    // present, else a UUID is minted), surface it on the tracing span so logs
    // correlate, and echo it back on the response.
    let trace = TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<axum::body::Body>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            tracing::info_span!(
                "http",
                method = %request.method(),
                uri = %request.uri(),
                request_id = %request_id,
            )
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO));
    let observability = tower::ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(trace)
        .layer(PropagateRequestIdLayer::x_request_id());

    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/llms.txt", get(agents::llms_txt))
        .merge(openapi::router(state.clone()))
        .merge(public_limited)
        .merge(public_unlimited)
        .merge(logged)
        .layer(cors)
        .layer(observability)
        .with_state(state)
}

/// Prometheus `/metrics` route + the layer that records per-request metrics.
///
/// Kept separate from [`router`] because [`PrometheusMetricLayer::pair`]
/// installs a *global* recorder, which can only happen once per process.
/// `main` calls this exactly once; tests that rebuild [`router`] do not.
pub fn metrics_layer() -> (PrometheusMetricLayer<'static>, Router) {
    let (layer, handle) = PrometheusMetricLayer::pair();
    let route = Router::new().route(
        "/metrics",
        get(move || {
            let rendered = handle.render();
            async move { rendered }
        }),
    );
    (layer, route)
}

/// Liveness: the process is up. Does not touch the database.
async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Readiness: the process can serve traffic, i.e. the DB pool is reachable.
/// Returns 503 so k8s can hold traffic off until the pool is live.
async fn readyz(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => Ok(Json(json!({ "status": "ready" }))),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// CORS policy:
///
/// Default to `http://localhost:3000` so a fresh checkout works for `make dev`.
/// In any other deployment, set `AME_CORS_ORIGINS` to a comma-separated list of
/// allowed origins. Setting it to `*` re-enables permissive CORS (use only when
/// the API is intentionally public).
///
/// Credentials are allowed when specific origins are configured. When using
/// wildcard (`*`), credentials are not allowed per CORS spec.
fn build_cors_layer() -> CorsLayer {
    use axum::http::{HeaderValue, Method, header};

    let raw =
        std::env::var("AME_CORS_ORIGINS").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];
    let headers = [header::AUTHORIZATION, header::CONTENT_TYPE];

    if raw.trim() == "*" {
        return CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods(methods)
            .allow_headers(headers);
    }

    let origins: Vec<HeaderValue> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| HeaderValue::from_str(s).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(methods)
        .allow_headers(headers)
        .allow_credentials(true)
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
