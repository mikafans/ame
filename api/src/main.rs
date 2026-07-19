use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ame_api::config::Config::load().unwrap_or_else(|e| {
        eprintln!("WARNING: Failed to load config, using defaults: {e}");
        ame_api::config::Config {
            server: ame_api::config::ServerConfig {
                port: 28080,
                production: false,
                cors_origins: "http://localhost:23000".to_string(),
                log_format: "compact".to_string(),
                database_url: None,
                valkey_url: None,
            },
            ratelimit: ame_api::config::RateLimitConfig {
                free: ame_api::config::TierConfig { burst: 60, rate: 1 },
                premium: ame_api::config::TierConfig {
                    burst: 600,
                    rate: 10,
                },
                public: ame_api::config::PublicConfig {
                    burst: 10,
                    period_secs: 2,
                },
                cost: ame_api::config::CostConfig { read: 1, write: 5 },
                trusted_proxies: Some(1),
            },
            batch: ame_api::config::BatchConfig {
                free: 50,
                premium: 500,
            },
            login: ame_api::config::LoginConfig::default(),
            registration: ame_api::config::RegistrationConfig::default(),
        }
    });

    init_tracing(&config);

    let database_url = config
        .server
        .database_url
        .clone()
        .unwrap_or_else(|| "postgres://postgres:postgres@localhost:5432/ame".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations on boot. They are embedded into the binary at compile
    // time and applied idempotently, so this is a no-op once the schema is
    // current (and the deploy needs no separate migration step).
    sqlx::migrate!("../db/migrations").run(&pool).await?;

    let sweep_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(e) = sqlx::query("DELETE FROM tb_login_sessions WHERE expires_at < now()")
                .execute(&sweep_pool)
                .await
            {
                tracing::warn!("login-session sweep failed: {e}");
            }
        }
    });

    let addr: SocketAddr = format!("0.0.0.0:{}", config.server.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "ame-api listening");

    let (prometheus_layer, metrics_route) = ame_api::http::metrics_layer();
    let app = ame_api::http::router(pool)
        .merge(metrics_route)
        .layer(prometheus_layer);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

/// Initialize tracing. Set `log_format = json` in ame.toml (or `AME_LOG_FORMAT=json` env) for structured logs in production;
/// otherwise a compact human-readable format is used.
fn init_tracing(config: &ame_api::config::Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "ame_api=debug,tower_http=info,sqlx=warn".into());
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false);

    if config.server.log_format == "json" {
        builder.json().init();
    } else {
        builder.compact().init();
    }
}
