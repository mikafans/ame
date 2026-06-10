//! Operator-tunable platform settings, resolved as the `ame.toml` config
//! defaults overlaid with any override rows in `tb_settings`.
//!
//! The resolver is cache-first (one Valkey blob, short TTL) and **fail-open**:
//! a Valkey or Postgres outage degrades to the compiled config defaults rather
//! than blocking request handling. A present `tb_settings` row overrides the
//! corresponding config default; an absent row means "use config".

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;

/// Single cache key holding the whole resolved settings blob.
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
pub struct TierQuota {
    pub free: i64,
    pub premium: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSettings {
    pub agents: TierQuota,
    pub assessments: TierQuota,
    pub questions: TierQuota,
}

/// The effective platform settings: config defaults with `tb_settings` overrides
/// applied. Serialized both as the admin API response and as the cache blob.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveSettings {
    pub maintenance_mode: bool,
    pub ratelimit: RateLimitSettings,
    pub quota: QuotaSettings,
}

impl EffectiveSettings {
    /// The baseline: every value taken from the compiled `ame.toml` config,
    /// maintenance mode off.
    pub fn from_config(c: &crate::config::Config) -> Self {
        Self {
            maintenance_mode: false,
            ratelimit: RateLimitSettings {
                free: TierLimit {
                    burst: c.ratelimit.free.burst,
                    rate: c.ratelimit.free.rate,
                },
                premium: TierLimit {
                    burst: c.ratelimit.premium.burst,
                    rate: c.ratelimit.premium.rate,
                },
            },
            quota: QuotaSettings {
                agents: TierQuota {
                    free: c.quota.agents.free,
                    premium: c.quota.agents.premium,
                },
                assessments: TierQuota {
                    free: c.quota.assessments.free,
                    premium: c.quota.assessments.premium,
                },
                questions: TierQuota {
                    free: c.quota.questions.free,
                    premium: c.quota.questions.premium,
                },
            },
        }
    }
}

/// Resolve the effective settings, preferring the Valkey cache and falling back
/// to a Postgres read; any failure degrades to the compiled config defaults.
pub async fn get_effective(
    pool: &PgPool,
    valkey: &deadpool_redis::Pool,
    config: &crate::config::Config,
) -> EffectiveSettings {
    // 1. Cache hit (fail-open: ignore connection/deserialize errors).
    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(CACHE_KEY).await
            && let Ok(settings) = serde_json::from_str::<EffectiveSettings>(&json)
        {
            return settings;
        }
    }

    // 2. Resolve from Postgres overlaid on config defaults.
    let settings = resolve_from_db(pool, config).await;

    // 3. Warm the cache (fail-open).
    if let Ok(json) = serde_json::to_string(&settings)
        && let Ok(mut conn) = valkey.get().await
    {
        use redis::AsyncCommands;
        let _: Result<(), redis::RedisError> = conn.set_ex(CACHE_KEY, json, CACHE_TTL_SECS).await;
    }

    settings
}

async fn resolve_from_db(pool: &PgPool, config: &crate::config::Config) -> EffectiveSettings {
    let mut settings = EffectiveSettings::from_config(config);

    let rows = sqlx::query("SELECT key, value FROM tb_settings")
        .fetch_all(pool)
        .await;

    match rows {
        Ok(rows) => {
            for row in rows {
                let key: String = row.get("key");
                let value: serde_json::Value = row.get("value");
                match key.as_str() {
                    "maintenance_mode" => {
                        if let Some(b) = value.as_bool() {
                            settings.maintenance_mode = b;
                        }
                    }
                    "ratelimit" => {
                        if let Ok(rl) = serde_json::from_value(value) {
                            settings.ratelimit = rl;
                        }
                    }
                    "quota" => {
                        if let Ok(q) = serde_json::from_value(value) {
                            settings.quota = q;
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            // Fail-open: a settings store outage must not block requests.
            tracing::error!("settings DB read failed, using config defaults: {e}");
        }
    }

    settings
}

/// Drop the cached settings blob so the next read re-resolves from Postgres.
/// Call after any write to `tb_settings`.
pub async fn invalidate(valkey: &deadpool_redis::Pool) {
    if let Ok(mut conn) = valkey.get().await {
        use redis::AsyncCommands;
        let _: Result<(), redis::RedisError> = conn.del(CACHE_KEY).await;
    }
}
