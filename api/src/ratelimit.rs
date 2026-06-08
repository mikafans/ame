use crate::auth::extractor::AuthenticatedUser;
use crate::http::AppState;
use axum::extract::State;
use deadpool_redis::Pool;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static LUA_SCRIPT: OnceLock<redis::Script> = OnceLock::new();

fn get_lua_script() -> &'static redis::Script {
    LUA_SCRIPT.get_or_init(|| {
        redis::Script::new(
            r#"
            local key = KEYS[1]
            local burst = tonumber(ARGV[1])
            local rate = tonumber(ARGV[2])
            local cost = tonumber(ARGV[3])
            local now = tonumber(ARGV[4])

            local state = redis.call('HMGET', key, 'tokens', 'last_updated', 'burst')
            local tokens = tonumber(state[1])
            local last_updated = tonumber(state[2])
            local old_burst = tonumber(state[3])

            if not tokens then
                tokens = burst
                last_updated = now
            else
                if old_burst and old_burst ~= burst then
                    tokens = burst
                    last_updated = now
                else
                    local elapsed = math.max(0, now - last_updated)
                    tokens = math.min(burst, tokens + elapsed * rate)
                end
            end

            if tokens >= cost then
                tokens = tokens - cost
                redis.call('HMSET', key, 'tokens', tokens, 'last_updated', now, 'burst', burst)
                redis.call('EXPIRE', key, 86400)
                return 1
            else
                if old_burst and old_burst ~= burst then
                    redis.call('HMSET', key, 'tokens', tokens, 'last_updated', now, 'burst', burst)
                end
                return 0
            end
        "#,
        )
    })
}

fn get_now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[derive(Debug)]
pub struct RateLimiter {
    pool: Pool,
    fallback: Mutex<HashMap<String, FallbackBucket>>,
}

#[derive(Debug, Clone)]
struct FallbackBucket {
    tokens: f64,
    last_updated: Instant,
}

impl RateLimiter {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            fallback: Mutex::new(HashMap::new()),
        }
    }

    /// Try consuming `cost` tokens from the bucket `key`.
    ///
    /// If Valkey connection fails or execution fails, falls back to a thread-safe
    /// in-process rate limiter instance.
    pub async fn try_consume(
        &self,
        key: &str,
        burst: u32,
        refill_rate: f64, // tokens per second
        cost: u32,
    ) -> bool {
        let now = get_now_secs();

        match self.pool.get().await {
            Ok(mut conn) => {
                let script = get_lua_script();
                let res: Result<i32, redis::RedisError> = script
                    .key(key)
                    .arg(burst)
                    .arg(refill_rate)
                    .arg(cost)
                    .arg(now)
                    .invoke_async(&mut *conn)
                    .await;

                match res {
                    Ok(1) => true,
                    Ok(_) => false,
                    Err(e) => {
                        tracing::error!(
                            "Valkey Lua script execution failed: {:?}. Falling back to in-memory.",
                            e
                        );
                        self.try_consume_fallback(key, burst, refill_rate, cost)
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get connection from Valkey pool: {:?}. Falling back to in-memory.",
                    e
                );
                self.try_consume_fallback(key, burst, refill_rate, cost)
            }
        }
    }

    fn try_consume_fallback(&self, key: &str, burst: u32, refill_rate: f64, cost: u32) -> bool {
        let mut fallback = self
            .fallback
            .lock()
            .expect("limiter fallback mutex poisoned");
        let now = Instant::now();
        let burst_f = burst as f64;
        let cost_f = cost as f64;

        let bucket = fallback
            .entry(key.to_string())
            .or_insert_with(|| FallbackBucket {
                tokens: burst_f,
                last_updated: now,
            });

        let elapsed = now.duration_since(bucket.last_updated).as_secs_f64();
        bucket.tokens = (burst_f).min(bucket.tokens + elapsed * refill_rate);
        bucket.last_updated = now;

        if bucket.tokens >= cost_f {
            bucket.tokens -= cost_f;
            true
        } else {
            false
        }
    }
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, crate::domain::error::ApiError> {
    let path = req.uri().path();
    if path == "/healthz"
        || path == "/metrics"
        || path == "/skill.json"
        || path == "/llms.txt"
        || path == "/openapi.yaml"
    {
        return Ok(next.run(req).await);
    }

    let auth = req.extensions().get::<AuthenticatedUser>();
    let method = req.method().clone();
    let trusted_proxies = state.config.ratelimit.trusted_proxies.unwrap_or(1);

    let (key, burst, refill_rate, cost) = if path == "/v1/me/export" {
        if let Some(auth) = auth {
            let key = format!("ame:limiter:export:{}", auth.owner_id);
            let burst = state.config.ratelimit.export.burst;
            let refill_rate = 1.0 / (state.config.ratelimit.export.period_secs as f64);
            (key, burst, refill_rate, 1)
        } else {
            let ip = get_client_ip(&req, trusted_proxies);
            let key = format!("ame:limiter:ip:{ip}");
            let burst = state.config.ratelimit.public.burst;
            let refill_rate = 1.0 / (state.config.ratelimit.public.period_secs as f64);
            (key, burst, refill_rate, 1)
        }
    } else if let Some(auth) = auth {
        let key = format!("ame:limiter:owner:{}", auth.owner_id);
        let (burst, rate) = match auth.owner_plan.as_str() {
            "premium" => (
                state.config.ratelimit.premium.burst,
                state.config.ratelimit.premium.rate as f64,
            ),
            _ => (
                state.config.ratelimit.free.burst,
                state.config.ratelimit.free.rate as f64,
            ),
        };
        let is_read = matches!(
            method,
            axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
        );
        let cost = if is_read {
            state.config.ratelimit.cost.read
        } else {
            state.config.ratelimit.cost.write
        };
        (key, burst, rate, cost)
    } else {
        let ip = get_client_ip(&req, trusted_proxies);
        let key = format!("ame:limiter:ip:{ip}");
        let burst = state.config.ratelimit.public.burst;
        let refill_rate = 1.0 / (state.config.ratelimit.public.period_secs as f64);
        (key, burst, refill_rate, 1)
    };

    if state
        .limiter
        .try_consume(&key, burst, refill_rate, cost)
        .await
    {
        Ok(next.run(req).await)
    } else {
        metrics::counter!("ratelimit_rejection_total").increment(1);
        Err(crate::domain::error::ApiError::TooManyRequests)
    }
}

fn get_client_ip(req: &axum::extract::Request, trusted_proxies: usize) -> String {
    let xff_header = if trusted_proxies > 0 {
        req.headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
    } else {
        None
    };
    if let Some(xff) = xff_header {
        let ips: Vec<&str> = xff.split(',').map(|s| s.trim()).collect();
        if ips.len() > trusted_proxies {
            return ips[ips.len() - 1 - trusted_proxies].to_string();
        }
    }
    req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::ConnectInfo;
    use std::net::SocketAddr;

    #[test]
    fn test_get_client_ip_no_xff_no_connect_info() {
        let req = axum::extract::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(get_client_ip(&req, 1), "127.0.0.1");
    }

    #[test]
    fn test_get_client_ip_connect_info_fallback() {
        let addr: SocketAddr = "192.168.1.50:8080".parse().unwrap();
        let mut req = axum::extract::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));

        assert_eq!(get_client_ip(&req, 1), "192.168.1.50");
        assert_eq!(get_client_ip(&req, 0), "192.168.1.50");
    }

    #[test]
    fn test_get_client_ip_trusted_proxies() {
        let addr: SocketAddr = "10.0.0.1:8080".parse().unwrap();

        // Case: AME_TRUSTED_PROXIES=1, XFF: "1.2.3.4, 10.0.0.1" -> "1.2.3.4"
        let mut req = axum::extract::Request::builder()
            .header("x-forwarded-for", "1.2.3.4, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));
        assert_eq!(get_client_ip(&req, 1), "1.2.3.4");

        // Case: AME_TRUSTED_PROXIES=1, XFF: "evil, 1.2.3.4, 10.0.0.1" -> "1.2.3.4"
        let mut req = axum::extract::Request::builder()
            .header("x-forwarded-for", "evil, 1.2.3.4, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));
        assert_eq!(get_client_ip(&req, 1), "1.2.3.4");

        // Case: AME_TRUSTED_PROXIES=2, XFF: "1.2.3.4, 10.0.0.2, 10.0.0.1" -> "1.2.3.4"
        let mut req = axum::extract::Request::builder()
            .header("x-forwarded-for", "1.2.3.4, 10.0.0.2, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));
        assert_eq!(get_client_ip(&req, 2), "1.2.3.4");

        // Case: XFF has fewer entries than trusted_proxies + 1 -> fallback to ConnectInfo
        let mut req = axum::extract::Request::builder()
            .header("x-forwarded-for", "10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));
        assert_eq!(get_client_ip(&req, 1), "10.0.0.1");

        // Case: trusted_proxies=0 -> fallback to ConnectInfo
        let mut req = axum::extract::Request::builder()
            .header("x-forwarded-for", "1.2.3.4, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(addr));
        assert_eq!(get_client_ip(&req, 0), "10.0.0.1");
    }
}
