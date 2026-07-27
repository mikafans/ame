//! Admin-only user management and moderation routes.

use crate::http::AppState;
use axum::{Router, routing::get};

mod audit;
mod health;
mod settings;
mod users;

pub use audit::*;
pub use health::*;
pub use settings::*;
pub use users::*;

pub async fn admin_guard_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, crate::domain::error::ApiError> {
    let auth = req
        .extensions()
        .get::<crate::auth::extractor::AuthenticatedUser>()
        .ok_or(crate::domain::error::ApiError::Unauthorized)?;

    if auth.user.role != crate::domain::user::Role::Admin {
        return Err(crate::domain::error::ApiError::Forbidden(
            std::borrow::Cow::Borrowed("administrator role required"),
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
        .route("/v1/admin/audit", get(audit::list_audit_logs))
        .route("/v1/admin/health", get(health::get_admin_health))
        .route(
            "/v1/admin/settings",
            get(settings::get_settings).put(settings::put_settings),
        )
        .layer(axum::middleware::from_fn(admin_guard_middleware))
        .with_state(state)
}
