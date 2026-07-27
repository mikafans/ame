//! Operator authorization extractor.
//!
//! Learner routes use `AuthenticatedUser` directly. This extractor is only
//! for the small set of operator routes and is not a client capability model.

use axum::extract::FromRequestParts;

use crate::{
    auth::extractor::AuthenticatedUser,
    domain::{error::ApiError, user::Role},
    http::AppState,
};

pub struct RequireAdmin(pub AuthenticatedUser);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;
        if user.user.role != Role::Admin {
            return Err(ApiError::Forbidden(std::borrow::Cow::Borrowed(
                "administrator role required",
            )));
        }
        Ok(Self(user))
    }
}
