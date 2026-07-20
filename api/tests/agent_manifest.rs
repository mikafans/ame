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
    assert!(paths.contains(&"/api/v1/learning/journeys"));
    assert!(paths.contains(&"/api/v1/assessments?activityId={activity_id}"));
    assert!(paths.contains(&"/api/v1/deep-dives?activityId={activity_id}"));
    assert!(paths.contains(&"/api/v1/attempts/{attempt_id}/finish"));
    assert_eq!(
        manifest["entrypoint"],
        Value::String("/public/llms.txt".into())
    );
}

#[test]
fn advertised_activity_authoring_is_present_in_openapi() {
    let manifest = ame_api::http::agents::build_skill_manifest();
    assert!(manifest["tools"].as_array().unwrap().iter().any(|tool| {
        tool["method"] == "PATCH"
            && tool["path"] == "/api/v1/learning/activities/{activity_id}/content"
    }));

    let openapi = ame_api::http::openapi::openapi_yaml();
    assert!(openapi.contains("/api/v1/learning/activities/{activity_id}/content:"));
    assert!(openapi.contains("  patch:\n"));
}
