use axum::{extract::FromRequestParts, http::request::Parts};

use super::extractor::AuthenticatedUser;
use crate::{
    domain::{error::ApiError, user::Scope},
    http::AppState,
};

/// Marker trait that pins a required [`Scope`] at the type level.
///
/// Implement it once per protected route group:
///
/// ```ignore
/// struct WriteQuestionsScope;
/// impl ScopeConstraint for WriteQuestionsScope {
///     const SCOPE: Scope = Scope::AgentWriteQuestions;
/// }
///
/// async fn handler(_user: RequireScope<WriteQuestionsScope>) { /* ... */ }
/// ```
///
/// Using the [`Scope`] enum (not `&str`) means the required scope is checked
/// against the same canonical list the auth extractor produces, so a typo or
/// rename can't quietly grant access to an unintended endpoint.
pub trait ScopeConstraint: Send + Sync {
    const SCOPE: Scope;
}

/// Axum extractor that succeeds only if the authenticated token carries `T::SCOPE`.
///
/// Wraps the underlying [`AuthenticatedUser`] so handlers can still reach the
/// user record after the scope check passes.
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

        if !auth_user.token_scopes.contains(&T::SCOPE) {
            // ApiError::ScopeRequired carries a &'static str for the wire payload;
            // Scope::as_str is `const fn` so this remains a zero-cost lookup.
            return Err(ApiError::ScopeRequired(T::SCOPE.as_str()));
        }

        Ok(RequireScope(auth_user, std::marker::PhantomData))
    }
}
