//! Email+password auth routes: register, login, logout.

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::{Json, http::StatusCode, response::IntoResponse};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::token::{generate_secret, hash_secret},
    domain::error::{ApiError, FieldError},
    http::AppState,
};

// ── shapes ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBody {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
}

// ── handlers ──────────────────────────────────────────────────────────────────

fn set_token_cookie_header(token: &str) -> String {
    let secure = if std::env::var("AME_PRODUCTION").is_ok() {
        "; Secure"
    } else {
        ""
    };
    format!(
        "ame_token={}; HttpOnly{}; SameSite=Lax; Path=/; Max-Age=86400",
        token, secure
    )
}

/// POST /v1/auth/register — register a new user with email and password.
#[utoipa::path(
    post,
    path = "/v1/auth/register",
    request_body = RegisterBody,
    responses(
        (status = 201, description = "User registered", body = AuthResponse),
        (status = 422, description = "Validation failed")
    ),
    tag = "auth"
)]
pub async fn register(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<(StatusCode, [(String, String); 1], Json<AuthResponse>), ApiError> {
    // Validate email
    if body.email.trim().is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "email".into(),
            message: "must not be empty".into(),
        }]));
    }

    // Validate name
    if body.name.trim().is_empty() {
        return Err(ApiError::Validation(vec![FieldError {
            field: "name".into(),
            message: "must not be empty".into(),
        }]));
    }

    // Validate password
    if body.password.len() < 8 {
        return Err(ApiError::Validation(vec![FieldError {
            field: "password".into(),
            message: "must be at least 8 characters".into(),
        }]));
    }

    // Validate role — only user and admin are valid for human registration
    let normalized_role = body.role.to_lowercase();
    match normalized_role.as_str() {
        "user" | "admin" => {}
        _ => {
            return Err(ApiError::Validation(vec![FieldError {
                field: "role".into(),
                message: "must be 'user' or 'admin'".to_string(),
            }]));
        }
    }

    // Hash password
    let password_hash = hash_password(&body.password)?;

    // Insert user
    let user_id = Uuid::now_v7();
    let result = sqlx::query(
        "INSERT INTO tb_users (id, email, display_name, role, password_hash)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, email, display_name, role",
    )
    .bind(user_id)
    .bind(&body.email)
    .bind(&body.name)
    .bind(&normalized_role)
    .bind(&password_hash)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        // Check for unique violation on email
        if e.to_string()
            .contains("duplicate key value violates unique constraint")
        {
            ApiError::Validation(vec![FieldError {
                field: "email".into(),
                message: "email already registered".into(),
            }])
        } else {
            ApiError::Internal(e.into())
        }
    })?
    .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("insert returned no rows")))?;

    let user_id: Uuid = result.get("id");
    let email: String = result.get("email");
    let display_name: String = result.get("display_name");
    let role: String = result.get("role");

    let token_str = issue_token(&state.pool, user_id, &role).await?;
    let cookie_header = set_token_cookie_header(&token_str);

    Ok((
        StatusCode::CREATED,
        [("set-cookie".to_string(), cookie_header)],
        Json(AuthResponse {
            token: token_str,
            user: UserInfo {
                id: user_id,
                name: display_name,
                email,
                role,
            },
        }),
    ))
}

/// POST /v1/auth/login — authenticate with email and password.
#[utoipa::path(
    post,
    path = "/v1/auth/login",
    request_body = LoginBody,
    responses(
        (status = 200, description = "Logged in", body = AuthResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "auth"
)]
pub async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<(StatusCode, [(String, String); 1], Json<AuthResponse>), ApiError> {
    // Fetch user by email
    let user_row = sqlx::query(
        "SELECT id, email, display_name, role, password_hash, deactivated_at FROM tb_users WHERE email = $1",
    )
    .bind(&body.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::Unauthorized)?;

    let user_id: Uuid = user_row.get("id");
    let email: String = user_row.get("email");
    let display_name: String = user_row.get("display_name");
    let role: String = user_row.get("role");
    let password_hash: Option<String> = user_row.get("password_hash");
    let deactivated_at: Option<time::OffsetDateTime> = user_row.get("deactivated_at");

    if deactivated_at.is_some() {
        return Err(ApiError::Unauthorized);
    }

    // Verify password
    let password_hash_str = password_hash.ok_or(ApiError::Unauthorized)?;
    if !verify_password(&password_hash_str, &body.password) {
        return Err(ApiError::Unauthorized);
    }

    let token_str = issue_token(&state.pool, user_id, &role).await?;
    let cookie_header = set_token_cookie_header(&token_str);

    Ok((
        StatusCode::OK,
        [("set-cookie".to_string(), cookie_header)],
        Json(AuthResponse {
            token: token_str,
            user: UserInfo {
                id: user_id,
                name: display_name,
                email,
                role,
            },
        }),
    ))
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Mint a new API token row and return the plaintext `{id}_{secret}`. Only the
/// hash is persisted, and each call creates an independent row so concurrent
/// logins (multiple devices, parallel e2e specs) don't invalidate one another.
async fn issue_token(pool: &PgPool, user_id: Uuid, role: &str) -> Result<String, ApiError> {
    let token_id = Uuid::now_v7();
    let secret = generate_secret();
    let token_hash = hash_secret(&secret);

    sqlx::query(
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind("default")
    .bind(&token_hash)
    .bind(scopes_for_role(role))
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(format!("{token_id}_{secret}"))
}

fn scopes_for_role(role: &str) -> Vec<&'static str> {
    match role {
        "agent" => vec!["assessment.read"],
        "admin" => vec![
            "assessment.read",
            "assessment.write",
            "attempt.read",
            "attempt.write",
            "stats.read",
            "feedback.write",
            "plan.read",
            "plan.write",
            "admin",
        ],
        _ => vec![
            "assessment.read",
            "assessment.write",
            "attempt.read",
            "attempt.write",
            "stats.read",
            "feedback.write",
            "plan.read",
            "plan.write",
            "public.publish",
        ],
    }
}

pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("argon2 hash failed: {e}")))
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// POST /v1/auth/logout — clear the HttpOnly token cookie.
#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out")
    ),
    tag = "auth"
)]
pub async fn logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            "set-cookie".to_string(),
            format!(
                "ame_token=; HttpOnly{}; SameSite=Lax; Path=/; Max-Age=0",
                if std::env::var("AME_PRODUCTION").is_ok() {
                    "; Secure"
                } else {
                    ""
                }
            ),
        )],
    )
}

// Routes are mounted (with rate limiting) in `http::mod::router`. The handlers
// here are referenced directly from there.
