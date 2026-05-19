use axum::{extract::FromRequestParts, http::request::Parts};

use super::extractor::AuthenticatedUser;
use crate::{domain::error::ApiError, http::AppState};

pub trait ScopeConstraint: Send + Sync {
    const SCOPE: &'static str;
}

pub struct RequireScope<T: ScopeConstraint>(pub AuthenticatedUser, std::marker::PhantomData<T>);

impl<T> FromRequestParts<AppState> for RequireScope<T>
where
    T: ScopeConstraint + Send + Sync + 'static,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthenticatedUser::from_request_parts(parts, state).await?;

        if !auth_user.token_scopes.iter().any(|scope| scope == T::SCOPE) {
            return Err(ApiError::ScopeRequired(T::SCOPE));
        }

        Ok(RequireScope(auth_user, std::marker::PhantomData))
    }
}
