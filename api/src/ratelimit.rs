use crate::auth::extractor::AuthenticatedUser;
use crate::http::AppState;
use axum::{extract::State, response::IntoResponse};
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

            -- Fail closed if an invalid operator setting reaches Valkey.
            if burst < 1 or rate <= 0 or cost < 0 then
                return {0, 0, 60}
            end

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
                return {1, math.floor(tokens), math.ceil(math.max(0, 1 - tokens) / rate)}
            else
                redis.call('HMSET', key, 'tokens', tokens, 'last_updated', now, 'burst', burst)
                redis.call('EXPIRE', key, 86400)
                return {0, math.floor(tokens), math.ceil((cost - tokens) / rate)}
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitSnapshot {
    pub limit: u32,
    pub remaining: u32,
    pub reset_after_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumeResult {
    pub allowed: bool,
    pub snapshot: RateLimitSnapshot,
}

#[derive(Debug, Clone)]
pub struct RequestLimit {
    pub key: String,
    pub burst: u32,
    pub refill_rate: f64,
    pub cost: u32,
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
        self.consume(key, burst, refill_rate, cost).await.allowed
    }

    pub async fn consume(
        &self,
        key: &str,
        burst: u32,
        refill_rate: f64,
        cost: u32,
    ) -> ConsumeResult {
        let now = get_now_secs();

        match self.pool.get().await {
            Ok(mut conn) => {
                let script = get_lua_script();
                let res: Result<(i32, i64, i64), redis::RedisError> = script
                    .key(key)
                    .arg(burst)
                    .arg(refill_rate)
                    .arg(cost)
                    .arg(now)
                    .invoke_async(&mut *conn)
                    .await;

                match res {
                    Ok((allowed, remaining, reset_after_seconds)) => ConsumeResult {
                        allowed: allowed == 1,
                        snapshot: RateLimitSnapshot {
                            limit: burst,
                            remaining: remaining.max(0) as u32,
                            reset_after_seconds: reset_after_seconds.max(0) as u64,
                        },
                    },
                    Err(e) => {
                        tracing::error!(
                            "Valkey Lua script execution failed: {:?}. Falling back to in-memory.",
                            e
                        );
                        self.consume_fallback(key, burst, refill_rate, cost)
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get connection from Valkey pool: {:?}. Falling back to in-memory.",
                    e
                );
                self.consume_fallback(key, burst, refill_rate, cost)
            }
        }
    }

    pub async fn inspect(&self, key: &str, burst: u32, refill_rate: f64) -> RateLimitSnapshot {
        self.consume(key, burst, refill_rate, 0).await.snapshot
    }

    fn consume_fallback(
        &self,
        key: &str,
        burst: u32,
        refill_rate: f64,
        cost: u32,
    ) -> ConsumeResult {
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

        let allowed = bucket.tokens >= cost_f;
        if allowed {
            bucket.tokens -= cost_f;
        }

        let remaining = bucket.tokens.floor().max(0.0) as u32;
        let reset_after_seconds = if bucket.tokens >= cost_f {
            if bucket.tokens >= 1.0 || refill_rate <= 0.0 {
                0
            } else {
                ((1.0 - bucket.tokens) / refill_rate).ceil() as u64
            }
        } else if refill_rate <= 0.0 {
            u64::MAX
        } else {
            ((cost_f - bucket.tokens) / refill_rate).ceil() as u64
        };
        ConsumeResult {
            allowed,
            snapshot: RateLimitSnapshot {
                limit: burst,
                remaining,
                reset_after_seconds,
            },
        }
    }
}

pub fn authenticated_request_limit(
    state: &AppState,
    auth: &AuthenticatedUser,
    method: &axum::http::Method,
    path: &str,
) -> RequestLimit {
    let tier = if auth.user.plan == "premium" {
        &state.config.ratelimit.premium
    } else {
        &state.config.ratelimit.free
    };
    let is_read = matches!(
        method,
        &axum::http::Method::GET | &axum::http::Method::HEAD | &axum::http::Method::OPTIONS
    );
    let base_cost = if is_read {
        state.config.ratelimit.cost.read
    } else {
        state.config.ratelimit.cost.write
    };
    let cost = authenticated_request_cost(path, base_cost);
    RequestLimit {
        key: format!("ame:limiter:owner:{}", auth.owner_id),
        burst: tier.burst,
        refill_rate: tier.rate as f64,
        cost,
    }
}

fn apply_headers(response: &mut axum::response::Response, snapshot: RateLimitSnapshot) {
    let too_many_requests = response.status() == axum::http::StatusCode::TOO_MANY_REQUESTS;
    let headers = response.headers_mut();
    headers.insert(
        "x-ratelimit-limit",
        snapshot
            .limit
            .to_string()
            .parse()
            .expect("valid rate limit"),
    );
    headers.insert(
        "x-ratelimit-remaining",
        snapshot
            .remaining
            .to_string()
            .parse()
            .expect("valid rate limit"),
    );
    headers.insert(
        "x-ratelimit-reset",
        snapshot
            .reset_after_seconds
            .to_string()
            .parse()
            .expect("valid rate limit"),
    );
    if too_many_requests {
        headers.insert(
            "retry-after",
            snapshot
                .reset_after_seconds
                .max(1)
                .to_string()
                .parse()
                .expect("valid retry-after"),
        );
    }
}

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, crate::domain::error::ApiError> {
    let path = req.uri().path();
    if path == "/healthz" || path == "/metrics" {
        return Ok(next.run(req).await);
    }

    let auth = req.extensions().get::<AuthenticatedUser>();
    let method = req.method().clone();
    let trusted_proxies = state.config.ratelimit.trusted_proxies.unwrap_or(1);

    let request_limit = if let Some(auth) = auth {
        // Prefer the operator-tunable tiers resolved by the maintenance
        // middleware (stashed in extensions); fall back to config if absent.
        let mut request_limit = authenticated_request_limit(&state, auth, &method, path);
        if let Some(settings) = req.extensions().get::<crate::settings::EffectiveSettings>() {
            let tier = if auth.user.plan == "premium" {
                &settings.ratelimit.premium
            } else {
                &settings.ratelimit.free
            };
            request_limit.burst = tier.burst;
            request_limit.refill_rate = tier.rate as f64;
        }
        request_limit
    } else {
        let ip = get_client_ip(&req, trusted_proxies);
        let public_period_secs = state.config.ratelimit.public.period_secs.max(1);
        RequestLimit {
            key: public_rate_limit_key(path, &ip),
            burst: state.config.ratelimit.public.burst,
            refill_rate: 1.0 / (public_period_secs as f64),
            cost: 1,
        }
    };

    let result = state
        .limiter
        .consume(
            &request_limit.key,
            request_limit.burst,
            request_limit.refill_rate,
            request_limit.cost,
        )
        .await;
    if result.allowed {
        let mut response = next.run(req).await;
        apply_headers(&mut response, result.snapshot);
        Ok(response)
    } else {
        metrics::counter!("ratelimit_rejection_total").increment(1);
        let mut response = crate::domain::error::ApiError::TooManyRequests.into_response();
        apply_headers(&mut response, result.snapshot);
        Ok(response)
    }
}

fn public_rate_limit_key(path: &str, ip: &str) -> String {
    format!("ame:limiter:public:{}:{ip}", public_bucket(path))
}

fn public_bucket(path: &str) -> &'static str {
    match path {
        "/public/v1/auth/login" => "auth-login",
        "/public/v1/auth/register" => "auth-register",
        "/public/v1/onboarding/preview" | "/public/v1/onboarding/start" => "onboarding",
        _ => "api",
    }
}

fn authenticated_request_cost(path: &str, base_cost: u32) -> u32 {
    let _ = path;
    base_cost.max(1)
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

    #[test]
    fn public_routes_use_separate_rate_limit_buckets() {
        assert_eq!(public_bucket("/public/v1/auth/login"), "auth-login");
        assert_eq!(public_bucket("/public/v1/auth/register"), "auth-register");
        assert_eq!(public_bucket("/public/v1/onboarding/start"), "onboarding");
        assert_eq!(public_bucket("/public/v1/onboarding/preview"), "onboarding");
        assert_eq!(public_bucket("/api/v1/explore"), "api");
        assert_eq!(
            public_rate_limit_key("/public/v1/onboarding/start", "192.0.2.10"),
            "ame:limiter:public:onboarding:192.0.2.10"
        );
    }

    #[test]
    fn all_authenticated_requests_use_the_same_cost_policy() {
        assert_eq!(authenticated_request_cost("/v1/learning/journeys", 5), 5);
        assert_eq!(authenticated_request_cost("/v1/learning/sessions", 5), 5);
        assert_eq!(authenticated_request_cost("/v1/learning/journeys", 0), 1);
    }
}
