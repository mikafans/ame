use axum::{
    Router,
    extract::State,
    middleware,
    routing::{get, post},
};
use axum_prometheus::PrometheusMetricLayer;
use sqlx::PgPool;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: crate::config::Config,
    pub valkey: deadpool_redis::Pool,
    pub limiter: std::sync::Arc<crate::ratelimit::RateLimiter>,
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
pub mod health;
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
    let config = crate::config::Config::load().expect("failed to load config");
    let valkey = crate::config::create_valkey_pool(&config).expect("failed to create valkey pool");
    let limiter = std::sync::Arc::new(crate::ratelimit::RateLimiter::new(valkey.clone()));
    let state = AppState {
        pool,
        config,
        valkey,
        limiter,
    };

    let cors_origins = &state.config.server.cors_origins;

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

    // Endpoints that should be logged (activity_log)
    let logged_router = Router::new()
        .merge(assessments::router(state.clone()))
        .merge(sessions::router(state.clone()))
        .merge(me::router(state.clone()))
        .merge(plans::router(state.clone()))
        .merge(agents::logged_router(state.clone()))
        .merge(messages::router(state.clone()))
        .route("/v1/me/export", axum::routing::get(export::export_data))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            activity::activity_log_middleware,
        ));

    let api_routes = Router::new()
        .merge(admin::router(state.clone()))
        .merge(questions::router(state.clone()))
        .merge(explore::router(state.clone()))
        .merge(openapi::router(state.clone()))
        .merge(stats::router(state.clone()))
        .merge(tags::router(state.clone()))
        .merge(agents::public_router(state.clone()))
        .route("/v1/auth/register", post(auth::register))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/logout", post(auth::logout))
        .merge(logged_router)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::ratelimit::rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_extract_middleware,
        ));

    Router::new()
        .merge(api_routes)
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .layer(cors)
        .with_state(state)
}
