//! Admin-only user management and moderation routes.

use crate::auth::scope::ScopeConstraint;
use crate::domain::user::Scope;
use crate::http::AppState;
use axum::{Router, routing::get};

mod assessments;
mod audit;
mod health;
mod settings;
mod tokens;
mod users;

pub use assessments::*;
pub use audit::*;
pub use health::*;
pub use settings::*;
pub use tokens::*;
pub use users::*;

pub struct AdminScope;
impl ScopeConstraint for AdminScope {
    const SCOPE: Scope = Scope::Admin;
}

pub async fn admin_guard_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, crate::domain::error::ApiError> {
    let auth = req
        .extensions()
        .get::<crate::auth::extractor::AuthenticatedUser>()
        .ok_or(crate::domain::error::ApiError::Unauthorized)?;

    if !auth.token_scopes.contains(&Scope::Admin) {
        return Err(crate::domain::error::ApiError::ScopeRequired(
            std::borrow::Cow::Borrowed(Scope::Admin.as_str()),
        ));
    }

    Ok(next.run(req).await)
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/admin/users", get(users::list_users))
        .route(
            "/v1/admin/users/{id}",
            axum::routing::patch(users::patch_user_admin),
        )
        .route("/v1/admin/tokens", get(tokens::list_tokens))
        .route(
            "/v1/admin/tokens/{id}",
            axum::routing::delete(tokens::delete_token_admin),
        )
        .route("/v1/admin/audit", get(audit::list_audit_logs))
        .route("/v1/admin/health", get(health::get_admin_health))
        .route(
            "/v1/admin/assessments",
            get(assessments::list_assessments_admin),
        )
        .route(
            "/v1/admin/assessments/{id}",
            axum::routing::delete(assessments::delete_assessment_admin),
        )
        .route(
            "/v1/admin/assessments/{id}/restore",
            axum::routing::post(assessments::restore_assessment_admin),
        )
        .route(
            "/v1/admin/settings",
            get(settings::get_settings).put(settings::put_settings),
        )
        .layer(axum::middleware::from_fn(admin_guard_middleware))
        .with_state(state)
}
