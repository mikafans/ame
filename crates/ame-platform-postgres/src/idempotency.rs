use serde_json::Value;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LoginSessionCredential {
    pub token_hash: String,
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct StoredResponse {
    pub request_hash: String,
    pub response_status: i16,
    pub response_body: Value,
}

pub async fn find_login_session(
    pool: &PgPool,
    token_id: Uuid,
) -> Result<Option<LoginSessionCredential>, sqlx::Error> {
    sqlx::query("SELECT token_hash, revoked_at FROM tb_login_sessions WHERE id = $1")
        .bind(token_id)
        .fetch_optional(pool)
        .await
        .map(|row| {
            row.map(|row| LoginSessionCredential {
                token_hash: row.get("token_hash"),
                revoked_at: row.get("revoked_at"),
            })
        })
}

pub async fn find_response(
    pool: &PgPool,
    token_id: Uuid,
    key: &str,
) -> Result<Option<StoredResponse>, sqlx::Error> {
    sqlx::query(
        "SELECT request_hash, response_status, response_body
         FROM tb_idempotency_keys WHERE token_id = $1 AND key = $2",
    )
    .bind(token_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(|row| StoredResponse {
            request_hash: row.get("request_hash"),
            response_status: row.get("response_status"),
            response_body: row.get("response_body"),
        })
    })
}

pub async fn store_response(
    pool: &PgPool,
    token_id: Uuid,
    key: &str,
    request_hash: &str,
    response_status: i16,
    response_body: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tb_idempotency_keys
             (token_id, key, request_hash, response_status, response_body)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (token_id, key) DO NOTHING",
    )
    .bind(token_id)
    .bind(key)
    .bind(request_hash)
    .bind(response_status)
    .bind(response_body)
    .execute(pool)
    .await
    .map(|_| ())
}
