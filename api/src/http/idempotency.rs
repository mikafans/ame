//! Idempotency-Key middleware.
//!
//! Implements the contract from `docs/specs/2026-05-19-question-exam-platform-design.md`:
//!
//! - **No `Idempotency-Key` header** → pass through unchanged.
//! - **Header present, no bearer token (or token unknown/revoked/wrong secret)**
//!   → pass through. The downstream auth extractor will reject the request with
//!   401; the middleware never stores or replays without an authenticated token.
//!   This is what keeps a stolen idempotency key from echoing a victim's stored
//!   response back at an attacker.
//! - **Header present, auth valid, no prior record** → forward the request,
//!   capture the response, and store it only if the status is 2xx AND the body
//!   parses as JSON. Non-2xx and non-JSON responses are returned to the caller
//!   but **not** cached — a transient 500 must not be replayed as the "answer"
//!   on retry.
//! - **Header present, auth valid, prior record with the same request hash**
//!   → replay the stored `(status, body)` without invoking the handler.
//! - **Header present, auth valid, prior record with a different request hash**
//!   → return `409 idempotency_conflict` (caller reused a key with a different
//!   payload).
//!
//! The request hash is `sha256(method || path || body)`. The pair
//! `(token_id, key)` is the storage primary key, so two different tokens can
//! reuse the same idempotency key without colliding.

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::{
    auth::token::{parse_bearer_token, verify_token_secret},
    domain::error::ApiError,
    http::AppState,
};

/// Axum middleware implementing the idempotency contract documented at the
/// module level. Apply it as a `route_layer` on the subset of POST routes that
/// accept `Idempotency-Key`; routes without the header pass through with zero
/// extra DB work.
pub async fn idempotency_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let headers = req.headers().clone();
    let idempotency_key = match headers.get("Idempotency-Key").and_then(|h| h.to_str().ok()) {
        Some(k) => k.to_string(),
        None => return Ok(next.run(req).await),
    };

    let auth_header = match headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        Some(h) => h,
        None => return Ok(next.run(req).await),
    };

    let parsed_token = match parse_bearer_token(auth_header) {
        Some(t) => t,
        None => return Ok(next.run(req).await),
    };
    let token_id = parsed_token.id;

    let token_record = sqlx::query(
        r#"
        SELECT token_hash, revoked_at
        FROM api_tokens
        WHERE id = $1
        "#,
    )
    .bind(token_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let Some(token_record) = token_record else {
        return Ok(next.run(req).await);
    };

    let revoked_at: Option<time::OffsetDateTime> = token_record.get("revoked_at");
    let token_hash: String = token_record.get("token_hash");
    if revoked_at.is_some() || !verify_token_secret(&token_hash, &parsed_token.secret) {
        return Ok(next.run(req).await);
    }

    let (parts, body) = req.into_parts();
    let bytes = body
        .collect()
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?
        .to_bytes();

    let mut hasher = Sha256::new();
    hasher.update(parts.method.as_str().as_bytes());
    hasher.update(parts.uri.path().as_bytes());
    hasher.update(&bytes);
    let request_hash = hex::encode(hasher.finalize());

    let existing = sqlx::query(
        r#"
        SELECT request_hash, response_status, response_body
        FROM idempotency_keys
        WHERE token_id = $1 AND key = $2
        "#,
    )
    .bind(token_id)
    .bind(&idempotency_key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if let Some(row) = existing {
        let stored_request_hash: String = row.get("request_hash");
        if stored_request_hash != request_hash {
            return Err(ApiError::IdempotencyConflict);
        }

        let response_status: i16 = row.get("response_status");
        let response_body: Value = row.get("response_body");
        let status = StatusCode::from_u16(response_status as u16).unwrap_or(StatusCode::OK);
        return Ok((status, Json(response_body)).into_response());
    }

    let req = Request::from_parts(parts, Body::from(bytes));
    let response = next.run(req).await;

    let (resp_parts, resp_body) = response.into_parts();
    let resp_bytes = match resp_body.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => Bytes::new(), // If we can't collect response body, just return what we have (this shouldn't happen for our JSON responses)
    };

    if resp_parts.status.is_success()
        && !resp_bytes.is_empty()
        && let Ok(json_body) = serde_json::from_slice::<Value>(&resp_bytes)
    {
        let status = resp_parts.status.as_u16() as i16;
        sqlx::query(
            r#"
            INSERT INTO idempotency_keys (token_id, key, request_hash, response_status, response_body)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (token_id, key) DO NOTHING
            "#,
        )
        .bind(token_id)
        .bind(idempotency_key)
        .bind(request_hash)
        .bind(status)
        .bind(json_body)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    Ok(Response::from_parts(resp_parts, Body::from(resp_bytes)))
}
