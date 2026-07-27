pub use ame_platform_postgres::settings::{EffectiveSettings, RateLimitSettings, TierLimit};

pub async fn get_effective(
    pool: &sqlx::PgPool,
    valkey: &deadpool_redis::Pool,
    config: &crate::config::Config,
) -> EffectiveSettings {
    ame_platform_postgres::settings::get_effective(
        pool,
        valkey,
        ame_platform_postgres::settings::SettingsDefaults {
            free_burst: config.ratelimit.free.burst,
            free_rate: config.ratelimit.free.rate,
            premium_burst: config.ratelimit.premium.burst,
            premium_rate: config.ratelimit.premium.rate,
        },
    )
    .await
}

pub async fn upsert(
    pool: &sqlx::PgPool,
    key: &str,
    value: &serde_json::Value,
    actor: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    ame_platform_postgres::settings::upsert(pool, key, value, actor).await
}

pub async fn invalidate(valkey: &deadpool_redis::Pool) {
    ame_platform_postgres::settings::invalidate(valkey).await;
}
