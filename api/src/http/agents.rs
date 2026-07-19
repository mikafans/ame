//! Public machine-facing documentation for the unified learning API.
//!
//! Agents do not have a separate resource model. They use the same learner
//! bearer session and owner-scoped REST endpoints as the web application.

use axum::{Json, Router, response::IntoResponse, routing::get};
use serde_json::{Value, json};

use crate::http::AppState;

/// GET /skill.json — machine-readable public contract.
pub async fn skill_manifest() -> Json<Value> {
    Json(build_skill_manifest())
}

/// GET /llms.txt — concise agent entry document.
pub async fn llms_txt() -> impl IntoResponse {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        include_str!("../../../docs/public/llms.txt"),
    )
}

fn endpoint(name: &str, description: &str, method: &str, path: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "method": method,
        "path": path,
        "input_schema": input_schema,
    })
}

pub fn build_skill_manifest() -> Value {
    let empty = json!({"type": "object", "properties": {}});
    let tools = vec![
        endpoint(
            "learning.preview",
            "Preview the promise, objectives, and first activity for a learner's prompt without creating state.",
            "POST",
            "/v1/onboarding/preview",
            json!({"type":"object","required":["prompt"],"properties":{"prompt":{"type":"string"}}}),
        ),
        endpoint(
            "learning.start",
            "Create or resume a learner account and bootstrap the first owner-scoped journey from one prompt.",
            "POST",
            "/v1/onboarding/start",
            json!({"type":"object","required":["email","displayName","prompt","idempotencyKey"],"properties":{"email":{"type":"string","format":"email"},"displayName":{"type":"string"},"prompt":{"type":"string"},"idempotencyKey":{"type":"string"}}}),
        ),
        endpoint(
            "learning.journey.list",
            "List the authenticated learner's journeys and the next activity for each.",
            "GET",
            "/v1/learning/journeys",
            empty.clone(),
        ),
        endpoint(
            "learning.journey.get",
            "Read one authenticated learner's intent, objectives, activities, and evidence.",
            "GET",
            "/v1/learning/journeys/{id}",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.activity.start",
            "Start an activity in an owned journey.",
            "POST",
            "/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
            json!({"type":"object","required":["journeyId","activityId"],"properties":{"journeyId":{"type":"string","format":"uuid"},"activityId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.session.finish",
            "Finish a learning session and persist the learner's evidence and responses.",
            "POST",
            "/v1/learning/sessions/{id}/finish",
            json!({"type":"object","required":["id","completed","responses"],"properties":{"id":{"type":"string","format":"uuid"},"completed":{"type":"boolean"},"responses":{"type":"array"}}}),
        ),
        endpoint(
            "learning.question.create",
            "Create a learner-owned versioned question with explanation and provenance metadata.",
            "POST",
            "/v1/questions",
            json!({"type":"object","required":["kind","prompt","points"],"properties":{"kind":{"type":"string","enum":["multiple_choice","true_false","short_answer","essay","code"]},"prompt":{"type":"string"},"options":{"type":"array"},"acceptedAnswers":{"type":"array","items":{"type":"string"}},"explanation":{"type":"string"},"rationale":{"type":"string"},"points":{"type":"integer","minimum":1},"reviewStatus":{"type":"string"},"sourceReferences":{"type":"array","items":{"type":"string"}}}}),
        ),
        endpoint(
            "learning.question.get",
            "Read one exact learner-owned question version for reproducible assessment history.",
            "GET",
            "/v1/questions/{question_id}/versions/{version}",
            json!({"type":"object","required":["questionId","version"],"properties":{"questionId":{"type":"string","format":"uuid"},"version":{"type":"integer","minimum":1}}}),
        ),
        endpoint(
            "learning.question.revise",
            "Create the next immutable version of a learner-owned question.",
            "POST",
            "/v1/questions/{question_id}/versions",
            json!({"type":"object","required":["kind","prompt","points"],"properties":{"kind":{"type":"string"},"prompt":{"type":"string"},"points":{"type":"integer","minimum":1}}}),
        ),
    ];

    json!({
        "schema_version": "v1",
        "name": "ame",
        "description": "ame — study, sweetened. An agent-friendly learning API with one owner-scoped learner model.",
        "auth": {
            "type": "bearer",
            "format": "ame_token",
            "registration": "POST /v1/onboarding/start returns a learner bearer token; no separate integration identity is required."
        },
        "entrypoint": "/llms.txt",
        "tools": tools,
        "principles": [
            "Agents and people use the same authenticated learner API.",
            "Every learning resource is scoped to the bearer token's learner.",
            "The API never sends email; email is an account identifier in self-host mode.",
            "Use idempotencyKey when starting a journey."
        ]
    })
}

pub fn public_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/llms.txt", get(llms_txt))
        .route("/skill.json", get(skill_manifest))
        .with_state(state)
}
