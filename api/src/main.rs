use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let database_url = std::env::var("AME_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations on boot. They are embedded into the binary at compile
    // time and applied idempotently, so this is a no-op once the schema is
    // current (and the deploy needs no separate migration step).
    sqlx::migrate!("../db/migrations").run(&pool).await?;

    let port = std::env::var("AME_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
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

/// Initialize tracing. Set `AME_LOG_FORMAT=json` for structured logs in production;
/// otherwise a compact human-readable format is used.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "ame_api=debug,tower_http=info,sqlx=warn".into());
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false);

    if std::env::var("AME_LOG_FORMAT").as_deref() == Ok("json") {
        builder.json().init();
    } else {
        builder.compact().init();
    }
}
