use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;

#[tokio::test]
async fn healthz_returns_ok() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&database_url)
        .unwrap();

    let app = ame_api::http::router(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let body: serde_json::Value = reqwest::get(format!("http://{addr}/healthz"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body, serde_json::json!({ "status": "ok" }));
}
