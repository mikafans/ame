use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use axum_prometheus::PrometheusMetricLayer;
use serde_json::json;
use sqlx::PgPool;
use tower_governor::GovernorLayer;
use tower_governor::errors::GovernorError;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

/// Key extractor that identifies requests by their Bearer token.
/// Used for the global per-token rate limit on authenticated routes.
#[derive(Clone)]
struct TokenKeyExtractor;
impl KeyExtractor for TokenKeyExtractor {
    type Key = String;
    fn extract<T>(&self, req: &axum::http::Request<T>) -> Result<Self::Key, GovernorError> {
        let parsed = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(crate::auth::token::parse_bearer_token)
            .or_else(|| {
                req.headers()
                    .get(axum::http::header::COOKIE)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|cookies| {
                        cookies.split(';').find_map(|c| {
                            let c = c.trim();
                            c.strip_prefix("ame_token=").map(|v| v.to_owned())
                        })
                    })
                    .and_then(|v| crate::auth::token::parse_token_value(&v))
            });

        match parsed {
            Some(p) => Ok(p.id.to_string()),
            None => Ok("anonymous".to_string()),
        }
    }
}

/// Middleware that pre-extracts and caches the [`AuthenticatedUser`] into request
/// extensions, so downstream extractors skip the DB round-trip.
///
/// Note: this does NOT rate-limit — see the GovernorLayer applied to `logged_router`.
pub async fn auth_extract_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::extract::FromRequestParts;
    let (mut parts, body) = req.into_parts();
    if let Ok(auth) =
        crate::auth::extractor::AuthenticatedUser::from_request_parts(&mut parts, &state).await
    {
        parts.extensions.insert(auth);
    }
    let req = axum::extract::Request::from_parts(parts, body);
    next.run(req).await
}

pub mod activity;
pub mod admin;
pub mod agents;
pub mod assessments;
pub mod auth;
pub mod db;
pub mod explore;
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

pub fn metrics_layer() -> (PrometheusMetricLayer<'static>, Router) {
    let (layer, handle) = PrometheusMetricLayer::pair();
    let route = Router::new().route("/metrics", get(|| async move { handle.render() }));
    (layer, route)
}

pub fn router(pool: PgPool) -> Router {
    let pool = db::convert_pool_to_ame_app(&pool);
    let state = AppState { pool };

    // CORS: Default to localhost:23000 if AME_CORS_ORIGINS is unset.
    // Use "*" to allow everything (prod preview).
    let cors_origins =
        std::env::var("AME_CORS_ORIGINS").unwrap_or_else(|_| "http://localhost:23000".to_string());

    let cors = if cors_origins == "*" {
        // Warning: `allow_origin(Any)` and `allow_credentials(true)` are mutually exclusive in CORS.
        // If `AME_CORS_ORIGINS=*` is used (e.g. for prod preview), cookie-based auth will silently fail
        // because the browser will refuse to send credentials to a wildcard origin.
        // To use cookie auth, set AME_CORS_ORIGINS to specific explicit origins.
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = cors_origins
            .split(',')
            .filter_map(|s| s.parse().ok())
            .collect();

        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PATCH,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
            ])
            .allow_credentials(true)
    };

    // Global rate limit: per-token, applied to all authenticated routes.
    // Defaults: burst 100, refill 1 per 1s.
    // Override with AME_GLOBAL_RATELIMIT_BURST and AME_GLOBAL_RATELIMIT_PERIOD_SECS.
    let governor_error_handler = |e: GovernorError| -> axum::response::Response {
        use axum::response::IntoResponse;
        match e {
            GovernorError::TooManyRequests { .. } => {
                crate::domain::error::ApiError::TooManyRequests.into_response()
            }
            GovernorError::UnableToExtractKey => crate::domain::error::ApiError::Internal(
                anyhow::anyhow!("Rate limit key extraction failed"),
            )
            .into_response(),
            _ => crate::domain::error::ApiError::TooManyRequests.into_response(),
        }
    };

    let global_burst = env_u32("AME_GLOBAL_RATELIMIT_BURST", 100);
    let global_period_secs = env_u64("AME_GLOBAL_RATELIMIT_PERIOD_SECS", 1);
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(global_period_secs)
            .burst_size(global_burst)
            .key_extractor(TokenKeyExtractor)
            .error_handler(governor_error_handler)
            .finish()
            .expect("valid rate-limit config"),
    );

    // Export-specific governor: 1 request per 60s, burst 1, per token.
    // Tighter than the global limit — export is an expensive full-bundle query.
    let export_governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(60)
            .burst_size(1)
            .key_extractor(TokenKeyExtractor)
            .error_handler(governor_error_handler)
            .finish()
            .expect("valid export rate-limit config"),
    );

    // Endpoints that should be logged (activity_log) and rate-limited (governor)
    let logged_router = Router::new()
        .merge(assessments::router(state.clone()))
        .merge(sessions::router(state.clone()))
        .merge(me::router(state.clone()))
        .merge(plans::router(state.clone()))
        .merge(agents::logged_router(state.clone()))
        .merge(messages::router(state.clone()))
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
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_extract_middleware,
        ));

    // Public, rate-limited endpoints. Credential-stuffing and key-faucet
    // surface: limit harder than the rest of the API.
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
            .error_handler(governor_error_handler)
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

    Router::new()
        .merge(admin::router(state.clone()))
        .merge(questions::router(state.clone()))
        .merge(explore::router(state.clone()))
        .merge(openapi::router(state.clone()))
        .merge(stats::router(state.clone()))
        .merge(tags::router(state.clone()))
        .merge(agents::public_router(state.clone()))
        .merge(public_limited)
        .merge(logged_router)
        .route("/healthz", get(healthz))
        .layer(cors)
        .with_state(state)
}

async fn healthz() -> impl axum::response::IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
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
