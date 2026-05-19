use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use sqlx::Row;

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
        let record = sqlx::query(
            r#"
            SELECT 
                t.token_hash, t.scopes, t.revoked_at,
                u.id as user_id, u.email, u.display_name, u.role, u.created_at
            FROM api_tokens t
            JOIN users u ON t.user_id = u.id
            WHERE t.id = $1
            "#,
        )
        .bind(parsed.id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::Unauthorized)?;

        let revoked_at: Option<time::OffsetDateTime> = record.get("revoked_at");
        if revoked_at.is_some() {
            return Err(ApiError::Unauthorized);
        }

        let token_hash: String = record.get("token_hash");
        if !verify_token_secret(&token_hash, &parsed.secret) {
            return Err(ApiError::Unauthorized);
        }

        // Update last_used_at in background
        let pool = state.pool.clone();
        let token_id = parsed.id;
        tokio::spawn(async move {
            let _ = sqlx::query("UPDATE api_tokens SET last_used_at = now() WHERE id = $1")
                .bind(token_id)
                .execute(&pool)
                .await;
        });

        let role: String = record.get("role");
        let role = match role.as_str() {
            "admin" => crate::domain::user::Role::Admin,
            _ => crate::domain::user::Role::User,
        };

        let user = User {
            id: record.get("user_id"),
            email: record.get("email"),
            display_name: record.get("display_name"),
            role,
            created_at: record.get("created_at"),
        };

        Ok(AuthenticatedUser {
            user,
            token_scopes: record.get("scopes"),
        })
    }
}
