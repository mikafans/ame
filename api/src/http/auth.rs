//! Email+password auth routes: register, login.

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::post};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    domain::error::{ApiError, FieldError},
    http::AppState,
};

// ── shapes ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBody {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
}

// ── handlers ──────────────────────────────────────────────────────────────────

/// POST /v1/auth/register — register a new user with email and password.
pub async fn register(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
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

    // Validate role
    let normalized_role = body.role.to_lowercase();
    match normalized_role.as_str() {
        "learner" | "instructor" | "admin" | "agent" => {}
        _ => {
            return Err(ApiError::Validation(vec![FieldError {
                field: "role".into(),
                message: format!("unknown role: {}", body.role),
            }]));
        }
    }

    // Hash password
    let password_hash = hash_password(&body.password)?;

    // Insert user
    let user_id = Uuid::now_v7();
    let result = sqlx::query(
        "INSERT INTO users (id, email, display_name, role, password_hash)
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

    // Generate and insert API key
    let token_id = Uuid::now_v7();
    let secret = generate_secret();
    let token_hash = hash_secret(&secret)?;

    let initial_scopes = scopes_for_role(&role);
    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token_id)
    .bind(user_id)
    .bind("default")
    .bind(&token_hash)
    .bind(initial_scopes)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token: format!("{token_id}_{secret}"),
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
pub async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Fetch user by email
    let user_row = sqlx::query(
        "SELECT id, email, display_name, role, password_hash FROM users WHERE email = $1",
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

    // Verify password
    let password_hash_str = password_hash.ok_or(ApiError::Unauthorized)?;
    if !verify_password(&password_hash_str, &body.password) {
        return Err(ApiError::Unauthorized);
    }

    // Fetch or create API key
    let token_row = sqlx::query("SELECT id FROM api_tokens WHERE user_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC LIMIT 1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let token_id = if let Some(row) = token_row {
        row.get("id")
    } else {
        // Create new key
        let new_token_id = Uuid::now_v7();
        let secret = generate_secret();
        let token_hash = hash_secret(&secret)?;

        let initial_scopes = scopes_for_role(&role);
        sqlx::query(
            "INSERT INTO api_tokens (id, user_id, name, token_hash, scopes)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(new_token_id)
        .bind(user_id)
        .bind("default")
        .bind(&token_hash)
        .bind(initial_scopes)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

        new_token_id
    };

    // Generate a fresh secret to return (we don't store the plaintext)
    let secret = generate_secret();
    let token_hash = hash_secret(&secret)?;

    sqlx::query("UPDATE api_tokens SET token_hash = $1 WHERE id = $2")
        .bind(&token_hash)
        .bind(token_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(AuthResponse {
        token: format!("{token_id}_{secret}"),
        user: UserInfo {
            id: user_id,
            name: display_name,
            email,
            role,
        },
    }))
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn scopes_for_role(role: &str) -> Vec<&'static str> {
    match role {
        "agent" => vec!["quiz.read"],
        "admin" => vec![
            "quiz.read",
            "quiz.write",
            "attempt.read",
            "attempt.write",
            "stats.read",
            "feedback.write",
            "plan.read",
            "plan.write",
            "admin",
        ],
        _ => vec![
            "quiz.read",
            "quiz.write",
            "attempt.read",
            "attempt.write",
            "stats.read",
            "feedback.write",
            "plan.read",
            "plan.write",
        ],
    }
}

pub fn generate_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_secret(secret: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("argon2 hash failed: {e}")))
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

// ── router ────────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .with_state(state)
}
