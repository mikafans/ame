use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::{
    domain::{error::ApiError, user::User},
    http::AppState,
};

use super::token::{parse_bearer_token, verify_token_secret};

pub struct AuthenticatedUser {
    pub user: User,
    pub token_scopes: Vec<String>,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let parsed = parse_bearer_token(auth_header).ok_or(ApiError::Unauthorized)?;

        // Find token and user
        let record = sqlx::query!(
            r#"
            SELECT 
                t.token_hash, t.scopes, t.revoked_at,
                u.id as user_id, u.email, u.display_name, u.role, u.created_at
            FROM api_tokens t
            JOIN users u ON t.user_id = u.id
            WHERE t.id = $1
            "#,
            parsed.id
        )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::Unauthorized)?;

        if record.revoked_at.is_some() {
            return Err(ApiError::Unauthorized);
        }

        if !verify_token_secret(&record.token_hash, &parsed.secret) {
            return Err(ApiError::Unauthorized);
        }

        // Update last_used_at in background
        let pool = state.pool.clone();
        let token_id = parsed.id;
        tokio::spawn(async move {
            let _ = sqlx::query!(
                "UPDATE api_tokens SET last_used_at = now() WHERE id = $1",
                token_id
            )
            .execute(&pool)
            .await;
        });

        let role = match record.role.as_str() {
            "admin" => crate::domain::user::Role::Admin,
            _ => crate::domain::user::Role::User,
        };

        let user = User {
            id: record.user_id,
            email: record.email,
            display_name: record.display_name,
            role,
            created_at: record.created_at,
        };

        Ok(AuthenticatedUser {
            user,
            token_scopes: record.scopes,
        })
    }
}
