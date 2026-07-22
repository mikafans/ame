use deadpool_redis::Pool as ValkeyPool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;

const CACHE_KEY: &str = "ame:settings:all";
const CACHE_TTL_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TierLimit {
    pub burst: u32,
    pub rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitSettings {
    pub free: TierLimit,
    pub premium: TierLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveSettings {
    pub maintenance_mode: bool,
    pub ratelimit: RateLimitSettings,
}

#[derive(Debug, Clone, Copy)]
pub struct SettingsDefaults {
    pub free_burst: u32,
    pub free_rate: u32,
    pub premium_burst: u32,
    pub premium_rate: u32,
}

impl EffectiveSettings {
    pub fn from_defaults(defaults: SettingsDefaults) -> Self {
        Self {
            maintenance_mode: false,
            ratelimit: RateLimitSettings {
                free: TierLimit {
                    burst: defaults.free_burst,
                    rate: defaults.free_rate,
                },
                premium: TierLimit {
                    burst: defaults.premium_burst,
                    rate: defaults.premium_rate,
                },
            },
        }
    }
}

pub async fn get_effective(
    pool: &PgPool,
    valkey: &ValkeyPool,
    defaults: SettingsDefaults,
) -> EffectiveSettings {
    if let Ok(mut conn) = valkey.get().await
        && let Ok(Some(json)) = conn.get::<_, Option<String>>(CACHE_KEY).await
        && let Ok(settings) = serde_json::from_str::<EffectiveSettings>(&json)
    {
        return settings;
    }

    let mut settings = EffectiveSettings::from_defaults(defaults);
    if let Ok(rows) = sqlx::query("SELECT key, value FROM tb_settings")
        .fetch_all(pool)
        .await
    {
        for row in rows {
            let key: String = row.get("key");
            let value: serde_json::Value = row.get("value");
            match key.as_str() {
                "maintenance_mode" => {
                    if let Some(value) = value.as_bool() {
                        settings.maintenance_mode = value;
                    }
                }
                "ratelimit" => {
                    if let Ok(value) = serde_json::from_value(value) {
                        settings.ratelimit = value;
                    }
                }
                _ => {}
            }
        }
    }

    if let Ok(json) = serde_json::to_string(&settings)
        && let Ok(mut conn) = valkey.get().await
    {
        let _: Result<(), redis::RedisError> = conn.set_ex(CACHE_KEY, json, CACHE_TTL_SECS).await;
    }
    settings
}

pub async fn upsert(
    pool: &PgPool,
    key: &str,
    value: &serde_json::Value,
    actor: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tb_settings (key, value, updated_by, updated_at)
         VALUES ($1, $2, $3, now())
         ON CONFLICT (key) DO UPDATE
         SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = now()",
    )
    .bind(key)
    .bind(value)
    .bind(actor)
    .execute(pool)
    .await
    .map(|_| ())
}

pub async fn invalidate(valkey: &ValkeyPool) {
    if let Ok(mut conn) = valkey.get().await {
        let _: Result<(), redis::RedisError> = conn.del(CACHE_KEY).await;
    }
}
