//! Agent skill-manifest integration tests.
//!
//! Covers the agent skill manifest exposed at `/skill.json`: the
//! tools it advertises.

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

    // Writes: assessment.create, assessment.batchCreate, assessment.update, assessment.addQuestion, assessment.delete, assessment.archive, assessment.publish, question.create, question.promote, question.update, attempt.grade
    // Reads: assessment.list, assessment.get, assessment.stats, question.list, activity.list, learning.journey.get, stats.user
    // Self: profile.get, memory.set, memory.append, target.set
    let all_expected = [
        "assessment.list",
        "assessment.get",
        "assessment.create",
        "assessment.batchCreate",
        "assessment.update",
        "assessment.addQuestion",
        "assessment.delete",
        "assessment.archive",
        "assessment.publish",
        "assessment.stats",
        "question.list",
        "question.create",
        "question.promote",
        "question.update",
        "activity.list",
        "learning.journey.get",
        "stats.user",
        "attempt.list",
        "attempt.grade",
        "profile.get",
        "memory.set",
        "memory.append",
        "target.set",
    ];
    for expected in &all_expected {
        assert!(
            tool_names.contains(expected),
            "skill manifest missing tool: {expected}. Got: {tool_names:?}"
        );
    }

    // Assert that dead tools are NOT present
    let dead_tools = [
        "assessment.generate",
        "session.create",
        "session.answer",
        "session.finish",
        "attempts.list", // old plural name; the live tool is the singular attempt.list
    ];
    for dead in &dead_tools {
        assert!(
            !tool_names.contains(dead),
            "skill manifest should not contain dead tool: {dead}"
        );
    }

    // API contract: read tools advertise their direct GET paths; write tools use POST /v1/agents/run.
    for t in tools {
        let name = t["name"].as_str().unwrap();
        let expected_method = match name {
            // Reads: direct GET
            "assessment.list" | "assessment.get" | "question.list" | "question.deepen"
            | "assessment.stats" | "activity.list" | "stats.user" | "attempt.list" => "GET",
            // Writes: all via POST /v1/agents/run
            "assessment.create"
            | "assessment.batchCreate"
            | "assessment.update"
            | "assessment.archive"
            | "assessment.publish"
            | "assessment.addQuestion"
            | "assessment.delete"
            | "question.create"
            | "question.promote"
            | "question.update"
            | "deepDive.request"
            | "deepDive.list"
            | "deepDive.publish"
            | "attempt.grade"
            | "profile.get"
            | "memory.set"
            | "memory.append"
            | "target.set"
            | "learning.journey.get" => "POST",
            _ => panic!("unknown tool name: {}", name),
        };
        let expected_path = match name {
            // Reads: direct GET paths
            "assessment.list" => "/v1/assessments",
            "assessment.get" => "/v1/assessments/{id}",
            "assessment.stats" => "/v1/assessments/{id}/stats",
            "question.list" => "/v1/questions",
            "question.deepen" => "/v1/questions/{id}/deepen",
            "activity.list" => "/v1/agents/activity",
            "stats.user" => "/v1/me/stats",
            "attempt.list" => "/v1/me/attempts",
            "learning.journey.get" => "/v1/agents/run",
            // Writes: all via /v1/agents/run
            "assessment.create"
            | "assessment.batchCreate"
            | "assessment.update"
            | "assessment.archive"
            | "assessment.publish"
            | "assessment.addQuestion"
            | "assessment.delete"
            | "question.create"
            | "question.promote"
            | "question.update"
            | "deepDive.request"
            | "deepDive.list"
            | "deepDive.publish"
            | "attempt.grade"
            | "profile.get"
            | "memory.set"
            | "memory.append"
            | "target.set" => "/v1/agents/run",
            _ => panic!("unknown tool name: {}", name),
        };

        assert_eq!(
            t["method"], expected_method,
            "tool {name} must advertise method {expected_method}"
        );
        assert_eq!(
            t["path"], expected_path,
            "tool {name} must advertise path {expected_path}"
        );
    }
}
