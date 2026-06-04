//! Agent skill-manifest integration tests.
//!
//! Covers the agent skill manifest exposed at `/skill.json`: the
//! tools it advertises and the field-stripping done in `?strict=1` mode.

use serde_json::Value;
use sqlx::PgPool;
use std::net::SocketAddr;

async fn serve(pool: PgPool) -> String {
    let app = ame_api::http::router(pool);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap()
    });
    format!("http://{addr}")
}

/// Serve with a lazy pool — does not connect to real DB, only usable for
/// endpoints that don't touch the database (e.g. the skill manifest).
async fn serve_lazy() -> String {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .unwrap();
    serve(pool).await
}

#[tokio::test]
async fn skill_manifest_contains_assessment_tools() {
    let base = serve_lazy().await;
    let client = reqwest::Client::new();

    let manifest: Value = client
        .get(format!("{base}/skill.json"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let tools = manifest["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Tools exposed in the unified assessment manifest (post agent-identity refactor)
    let all_expected = [
        "assessment.list",
        "assessment.get",
        "assessment.create",
        "assessment.update",
        "assessment.delete",
        "question.list",
        "question.create",
        "session.create",
        "session.finish",
    ];
    for expected in &all_expected {
        assert!(
            tool_names.contains(expected),
            "skill manifest missing tool: {expected}. Got: {tool_names:?}"
        );
    }
}

#[tokio::test]
async fn skill_manifest_strict_drops_ame_fields() {
    let base = serve_lazy().await;
    let client = reqwest::Client::new();

    let manifest: Value = client
        .get(format!("{base}/skill.json?strict=1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let tools = manifest["tools"].as_array().unwrap();
    for tool in tools {
        assert!(
            tool.get("method").is_none(),
            "strict manifest should not have 'method'"
        );
        assert!(
            tool.get("path").is_none(),
            "strict manifest should not have 'path'"
        );
        assert!(
            tool.get("scope").is_none(),
            "strict manifest should not have 'scope'"
        );
    }
}
