use deadpool_redis::Pool as ValkeyPool;
use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn invalidate_token(valkey: &ValkeyPool, token_id: Uuid) {
    if let Ok(mut conn) = valkey.get().await {
        let _: Result<(), redis::RedisError> = conn.del(format!("ame:login:{token_id}")).await;
    }
}

pub async fn invalidate_user_caches(pool: &PgPool, valkey: &ValkeyPool, user_id: Uuid) {
    let login_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM tb_login_sessions WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    if let Ok(mut conn) = valkey.get().await {
        for id in login_ids {
            let _: Result<(), redis::RedisError> = conn.del(format!("ame:login:{id}")).await;
        }
        let _: Result<(), redis::RedisError> =
            conn.del(format!("ame:limiter:owner:{user_id}")).await;
    }
}

pub async fn invalidate_user_tokens(pool: &PgPool, valkey: &ValkeyPool, user_id: Uuid) {
    let login_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM tb_login_sessions WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    let _ = sqlx::query("DELETE FROM tb_login_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    if let Ok(mut conn) = valkey.get().await {
        for id in login_ids {
            let _: Result<(), redis::RedisError> = conn.del(format!("ame:login:{id}")).await;
        }
        let _: Result<(), redis::RedisError> =
            conn.del(format!("ame:limiter:owner:{user_id}")).await;
    }
}
