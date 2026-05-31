use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Asynchronously write a record to tb_audit_log in a non-blocking background task.
/// Swallows database insertion errors silently so audit issues never block critical request flows.
pub fn audit(
    pool: PgPool,
    actor_user_id: Option<Uuid>,
    action: &'static str,
    target_type: Option<&'static str>,
    target_id: Option<Uuid>,
    metadata: Value,
) {
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO tb_audit_log (actor_user_id, action, target_type, target_id, metadata)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(actor_user_id)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(metadata)
        .execute(&pool)
        .await;
    });
}
