//! Outbound webhook dispatch with HMAC-SHA256 signatures and exponential retry.
//!
//! Signature format: `X-Harus-Signature: t=<unix_ts>,v1=<hmac_sha256(signing_key, ts + "." + body)>`.
//! Retries: up to 3 attempts with delays of 10s, 60s, 300s after the initial attempt.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::{PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

fn sign_payload(signing_key: &str, ts: u64, body: &str) -> String {
    let msg = format!("{ts}.{body}");
    let mut mac =
        HmacSha256::new_from_slice(signing_key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(msg.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Fire webhooks for `event` to all active subscribers.
/// Each dispatch runs in a background task — this function returns immediately.
pub async fn fire_webhooks(pool: &PgPool, event: &str, payload: serde_json::Value) {
    let rows = sqlx::query(
        "SELECT id, url, signing_key FROM tb_webhooks
         WHERE $1 = ANY(events) AND revoked_at IS NULL AND signing_key IS NOT NULL",
    )
    .bind(event)
    .fetch_all(pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("fire_webhooks: failed to query webhooks: {e}");
            return;
        }
    };

    for row in rows {
        let webhook_id: Uuid = row.get("id");
        let url: String = row.get("url");
        let signing_key: String = row.get("signing_key");
        let pool = pool.clone();
        let payload = payload.clone();
        let event = event.to_string();

        tokio::spawn(async move {
            dispatch_with_retries(&pool, webhook_id, &url, &signing_key, &event, payload).await;
        });
    }
}

async fn dispatch_with_retries(
    pool: &PgPool,
    webhook_id: Uuid,
    url: &str,
    signing_key: &str,
    event: &str,
    payload: serde_json::Value,
) {
    let body_str = serde_json::to_string(&payload).unwrap_or_default();
    let delivery_id = Uuid::now_v7();

    let _ = sqlx::query(
        "INSERT INTO tb_webhook_deliveries (id, webhook_id, event, payload, status, next_attempt_at)
         VALUES ($1, $2, $3, $4, 'pending', now())",
    )
    .bind(delivery_id)
    .bind(webhook_id)
    .bind(event)
    .bind(&payload)
    .execute(pool)
    .await;

    let client = reqwest::Client::new();
    // delays: 0s (immediate), then 10s, 60s, 300s before each retry
    let delays_secs: &[u64] = &[0, 10, 60, 300];

    for (attempt, &delay) in delays_secs.iter().enumerate() {
        if delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let sig = sign_payload(signing_key, ts, &body_str);
        let attempt_num = attempt as i32 + 1;

        let result = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Harus-Signature", format!("t={ts},v1={sig}"))
            .body(body_str.clone())
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                let _ = sqlx::query(
                    "UPDATE tb_webhook_deliveries
                     SET status = 'delivered', attempt_num = $1, completed_at = now()
                     WHERE id = $2",
                )
                .bind(attempt_num)
                .bind(delivery_id)
                .execute(pool)
                .await;
                return;
            }
            Ok(resp) => {
                let err = format!("HTTP {}", resp.status().as_u16());
                let _ = sqlx::query(
                    "UPDATE tb_webhook_deliveries SET attempt_num = $1, last_error = $2 WHERE id = $3",
                )
                .bind(attempt_num)
                .bind(&err)
                .bind(delivery_id)
                .execute(pool)
                .await;
            }
            Err(e) => {
                let _ = sqlx::query(
                    "UPDATE tb_webhook_deliveries SET attempt_num = $1, last_error = $2 WHERE id = $3",
                )
                .bind(attempt_num)
                .bind(e.to_string())
                .bind(delivery_id)
                .execute(pool)
                .await;
            }
        }
    }

    let _ = sqlx::query(
        "UPDATE tb_webhook_deliveries SET status = 'failed', completed_at = now() WHERE id = $1",
    )
    .bind(delivery_id)
    .execute(pool)
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_payload_is_deterministic() {
        let sig1 = sign_payload("secret", 1234567890, r#"{"event":"test"}"#);
        let sig2 = sign_payload("secret", 1234567890, r#"{"event":"test"}"#);
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn sign_payload_varies_with_ts() {
        let sig1 = sign_payload("secret", 1000, "body");
        let sig2 = sign_payload("secret", 2000, "body");
        assert_ne!(sig1, sig2);
    }
}
