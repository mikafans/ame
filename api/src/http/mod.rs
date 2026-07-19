use axum::{
    Router,
    extract::State,
    middleware,
    routing::{get, post},
};
use axum_prometheus::PrometheusMetricLayer;
use sqlx::PgPool;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::{
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    trace::TraceLayer,
};

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

/// Agent-role tokens (sub-accounts) are confined to a small allowlist of endpoints.
/// All other REST paths return 403 Forbidden. Runs AFTER `auth_extract_middleware`
/// so the cached user is available for the role check.
pub async fn agent_guard_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use crate::auth::extractor::AuthenticatedUser;
    use crate::domain::user::Role;
    use axum::response::IntoResponse;

    if let Some(auth) = req.extensions().get::<AuthenticatedUser>()
        && auth.user.role == Role::Agent
    {
        let path = req.uri().path();
        let method = req.method().as_str();

        // Allowlist: method + pattern (using routed MatchedPath when available,
        // or raw path as fallback).
        let matched_path = req
            .extensions()
            .get::<axum::extract::MatchedPath>()
            .map(|p| p.as_str())
            .unwrap_or(path);

        let allowed = matches!(
            (method, matched_path),
            ("GET", "/v1/assessments")
                | ("GET", "/v1/assessments/{id}")
                | ("GET", "/v1/assessments/{id}/stats")
                | ("GET", "/v1/questions")
                | ("GET", "/v1/questions/{id}/deepen")
                | ("GET", "/v1/me/attempts")
                | ("GET", "/v1/me/stats")
                | ("GET", "/v1/agents/activity")
                | ("GET", "/v1/learning/journeys/{id}")
                | ("POST", "/v1/agents/run")
                | ("GET", "/llms.txt")
                | ("GET", "/skill.json")
                | ("GET", "/openapi.yaml")
        );

        if !allowed {
            return crate::domain::error::ApiError::Forbidden(std::borrow::Cow::Borrowed(
                "agents are limited to read endpoints and POST /v1/agents/run",
            ))
            .into_response();
        }
    }

    next.run(req).await
}

/// Resolves the effective platform settings once per request, stashes them in
/// request extensions (so `rate_limit_middleware` reuses the same blob), and
/// returns 503 for non-admins while maintenance mode is on. Runs AFTER
/// `auth_extract_middleware` so the admin check sees the cached user.
///
/// A small allowlist always passes — health/readiness probes and the login +
/// discovery endpoints — so an operator can still authenticate and recover.
pub async fn maintenance_mode_middleware(
    State(state): State<AppState>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use crate::auth::extractor::AuthenticatedUser;
    use crate::domain::user::Scope;
    use axum::response::IntoResponse;

    let settings = crate::settings::get_effective(&state.pool, &state.valkey, &state.config).await;

    if settings.maintenance_mode {
        let path = req.uri().path();
        let exempt = matches!(
            path,
            "/v1/auth/login" | "/llms.txt" | "/skill.json" | "/openapi.yaml" | "/metrics"
        );
        let is_admin = req
            .extensions()
            .get::<AuthenticatedUser>()
            .map(|a| a.token_scopes.contains(&Scope::Admin))
            .unwrap_or(false);
        if !exempt && !is_admin {
            return crate::domain::error::ApiError::Maintenance.into_response();
        }
    }

    // Reused downstream by the rate limiter to avoid a second settings lookup.
    req.extensions_mut().insert(settings);
    next.run(req).await
}

pub mod activity;
pub mod admin;
pub mod agents;
pub mod assessments;
pub mod auth;
pub mod db;
pub mod deep_dives;
pub mod explore;
pub mod export;
pub mod health;
pub mod idempotency;
pub mod learning;
pub mod me;
pub mod messages;
pub mod onboarding;
pub mod openapi;
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
        .merge(deep_dives::router(state.clone()))
        .merge(me::router(state.clone()))
        .merge(agents::logged_router(state.clone()))
        .merge(messages::router(state.clone()))
        .merge(learning::router(state.clone()))
        .route("/v1/me/export", axum::routing::get(export::export_data))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency::idempotency_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            activity::activity_log_middleware,
        ));

    let api_routes = Router::new()
        .merge(admin::router(state.clone()))
        .merge(questions::router(state.clone()))
        .merge(explore::router(state.clone()))
        .merge(openapi::router(state.clone()))
        .merge(onboarding::router(state.clone()))
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
            maintenance_mode_middleware,
        ))
        .layer(middleware::from_fn(agent_guard_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_extract_middleware,
        ));

    Router::new()
        .merge(api_routes)
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .layer(PropagateRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
        ))
        .layer(SetRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    let request_id = request
                        .extensions()
                        .get::<RequestId>()
                        .and_then(|id| id.header_value().to_str().ok())
                        .unwrap_or("missing");
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %request_id,
                    )
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: std::time::Duration,
                     span: &tracing::Span| {
                        tracing::info!(
                            parent: span,
                            status = %response.status(),
                            duration_ms = latency.as_secs_f64() * 1000.0,
                            "http request completed"
                        );
                    },
                ),
        )
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(cors)
        .layer(middleware::from_fn(security_headers_middleware))
        .with_state(state)
}

pub async fn security_headers_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );
    headers.insert(
        axum::http::header::STRICT_TRANSPORT_SECURITY,
        axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );
    res
}
