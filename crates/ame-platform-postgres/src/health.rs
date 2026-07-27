use deadpool_redis::Pool as ValkeyPool;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub async fn postgres_ready(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}

pub async fn valkey_ready(valkey: &ValkeyPool) -> bool {
    match valkey.get().await {
        Ok(mut connection) => {
            let result: Result<(), _> = redis::cmd("PING").query_async(&mut *connection).await;
            result.is_ok()
        }
        Err(_) => false,
    }
}

pub async fn table_count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

pub async fn set_rls_guc(
    conn: &mut PgConnection,
    owner_id: Uuid,
    is_admin: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('app.owner', $1, false), set_config('app.is_admin', $2, false)")
        .bind(owner_id.to_string())
        .bind(if is_admin { "true" } else { "false" })
        .execute(conn)
        .await
        .map(|_| ())
}

pub async fn reset_connection(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("RESET ALL;").execute(conn).await.map(|_| ())
}
