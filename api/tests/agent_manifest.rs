//! Generated machine contract tests.

use serde_json::Value;

#[test]
fn skill_manifest_contains_namespaced_unified_learning_tools() {
    let manifest = ame_api::http::agents::build_skill_manifest();
    let tools = manifest["tools"].as_array().expect("tools array");

    let paths: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["path"].as_str())
        .collect();
    assert!(paths.contains(&"/public/v1/onboarding/preview"));
    assert!(paths.contains(&"/public/v1/onboarding/start"));
    assert!(paths.contains(&"/public/v1/auth/register"));
    assert!(paths.contains(&"/public/v1/auth/login"));
    assert!(paths.contains(&"/api/v1/learning/journeys"));
    assert!(paths.contains(&"/api/v1/learning/journeys/{journey_id}/chapters"));
    assert!(paths.contains(&"/api/v1/learning/journeys/{journey_id}/activities"));
    assert!(paths.contains(&"/api/v1/learning/sessions/{id}"));
    assert!(paths.contains(&"/api/v1/learning/activities/{activity_id}/content"));
    assert!(paths.contains(&"/api/v1/assessments?activityId={activity_id}"));
    assert!(paths.contains(&"/api/v1/deep-dives?activityId={activity_id}"));
    assert!(paths.contains(&"/api/v1/attempts/{attempt_id}/finish"));
    assert!(paths.contains(&"/api/v1/sources/imports"));
    assert!(paths.contains(&"/api/v1/source-imports"));
    assert!(paths.contains(&"/api/v1/source-snapshots/{snapshot_id}"));
    assert!(paths.contains(&"/api/v1/citations"));
    assert!(paths.contains(&"/api/v1/citations/{id}"));
    assert!(paths.contains(&"/api/v1/notes"));
    assert!(paths.contains(&"/api/v1/notes/{id}"));
    assert!(paths.contains(&"/api/v1/learning-variants"));
    assert!(paths.contains(&"/api/v1/learning-variants/{id}"));
    assert!(paths.contains(&"/api/v1/task-submissions/{submission_id}/revise"));
    assert!(paths.contains(&"/api/v1/learning/journeys/{id}/export"));
    assert!(paths.contains(&"/api/v1/learning/imports"));
    assert!(paths.contains(&"/api/v1/learning/journeys/{id}/analytics"));
    assert_eq!(
        manifest["entrypoint"],
        Value::String("/public/llms.md".into())
    );
}

#[test]
fn advertised_activity_authoring_is_present_in_openapi() {
    let manifest = ame_api::http::agents::build_skill_manifest();
    let tools = manifest["tools"].as_array().unwrap();
    assert!(tools.iter().any(|tool| {
        tool["method"] == "PATCH"
            && tool["path"] == "/api/v1/learning/activities/{activity_id}/content"
    }));
    assert!(tools.iter().any(|tool| {
        tool["method"] == "PATCH"
            && tool["path"] == "/api/v1/learning/activities/{activity_id}/rubric"
    }));

    let openapi = ame_api::http::openapi::openapi_yaml();
    assert!(openapi.contains("/api/v1/learning/activities/{activity_id}/content:"));
    assert!(openapi.contains("/api/v1/learning/activities/{activity_id}/rubric:"));
    assert!(openapi.contains("/api/v1/learning/journeys/{journey_id}/chapters:"));
    assert!(openapi.contains("/api/v1/learning/journeys/{journey_id}/activities:"));
    assert!(openapi.contains("  patch:\n"));
}
