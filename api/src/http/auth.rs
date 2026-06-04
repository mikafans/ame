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

fn set_token_cookie_header(token: &str, secure: bool) -> String {
    let secure_suffix = if secure { "; Secure" } else { "" };
    // Note: SameSite=Lax is fine for same-host dev or same-site prod deploys.
    // True cross-domain deploys (e.g. web and api on completely different domains)
    // will require SameSite=None and Secure to allow the cookie on subrequests.
    format!("ame_token={token}; HttpOnly{secure_suffix}; SameSite=Lax; Path=/; Max-Age=2592000")
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
    let trimmed_email = body.email.trim();
    if trimmed_email.is_empty() {
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

    // Validate role — only user is valid for human registration
    let normalized_role = body.role.to_lowercase();
    match normalized_role.as_str() {
        "user" => {}
        _ => {
            return Err(ApiError::Validation(vec![FieldError {
                field: "role".into(),
                message: "must be 'user'".to_string(),
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
    .bind(trimmed_email)
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
    let cookie_header = set_token_cookie_header(&token_str, state.config.server.production);

    metrics::counter!("signup_total").increment(1);

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
    let canonical = crate::auth::token::canonical_email(&body.email);

    // Per-account rate limiting to prevent distributed login brute force
    let (login_burst, login_refill) = if state.config.server.production {
        (5, 5.0 / 60.0)
    } else {
        (1000000, 1000000.0)
    };

    let account_limiter_key = format!("ame:limiter:login_account:{}", canonical);
    if !state
        .limiter
        .try_consume(&account_limiter_key, login_burst, login_refill, 1)
        .await
    {
        return Err(ApiError::TooManyRequests);
    }

    // Fetch user by email canonical
    let user_row = sqlx::query(
        "SELECT id, email, display_name, role, password_hash, deactivated_at FROM tb_users WHERE email_canonical = $1",
    )
    .bind(&canonical)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    // A valid dummy hash to equalize timing on the miss path
    let dummy_hash = "$argon2id$v=19$m=19456,t=2,p=1$fgRxq3Ut+2IcRLdMp41ROw$xzZcJdQfimlft7Byw21EMnT/p4TcRAJa68gHeUK2EZ4";

    let (hash_to_verify, is_valid_user) = match &user_row {
        Some(row) => {
            let hash: Option<String> = row.get("password_hash");
            let deactivated_at: Option<time::OffsetDateTime> = row.get("deactivated_at");
            match (hash, deactivated_at) {
                (Some(h), None) => (h.to_string(), true),
                _ => (dummy_hash.to_string(), false),
            }
        }
        None => (dummy_hash.to_string(), false),
    };

    let password_valid = verify_password(&hash_to_verify, &body.password);

    if !password_valid || !is_valid_user {
        metrics::counter!("login_total", "result" => "failure").increment(1);
        return Err(ApiError::Unauthorized);
    }

    let user_row = match user_row {
        Some(row) => row,
        None => {
            metrics::counter!("login_total", "result" => "failure").increment(1);
            return Err(ApiError::Unauthorized);
        }
    };

    let user_id: Uuid = user_row.get("id");
    let email: String = user_row.get("email");
    let display_name: String = user_row.get("display_name");
    let role: String = user_row.get("role");

    let token_str = issue_token(&state.pool, user_id, &role).await?;
    let cookie_header = set_token_cookie_header(&token_str, state.config.server.production);

    metrics::counter!("login_total", "result" => "success").increment(1);

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
        "INSERT INTO tb_api_tokens (id, user_id, name, token_hash, scopes, expires_at)
         VALUES ($1, $2, $3, $4, $5, NOW() + INTERVAL '30 days')",
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
pub async fn logout(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    // Try to extract the token to revoke it in the DB
    if let Some(parsed) = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(crate::auth::token::parse_bearer_token)
        .or_else(|| {
            req.headers()
                .get(axum::http::header::COOKIE)
                .and_then(|h| h.to_str().ok())
                .and_then(|cookies| {
                    cookies.split(';').find_map(|c| {
                        let c = c.trim();
                        c.strip_prefix("ame_token=").map(|v| v.to_owned())
                    })
                })
                .and_then(|v| crate::auth::token::parse_token_value(&v))
        })
    {
        let _ = sqlx::query("UPDATE tb_api_tokens SET revoked_at = NOW() WHERE id = $1")
            .bind(parsed.id)
            .execute(&state.pool)
            .await;
        crate::auth::extractor::invalidate_token(&state.valkey, parsed.id).await;
    }

    (
        StatusCode::OK,
        [(
            "set-cookie".to_string(),
            format!(
                "ame_token=; HttpOnly{}; SameSite=Lax; Path=/; Max-Age=0",
                if state.config.server.production {
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
