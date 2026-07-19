//! Public machine-facing documentation for the unified learning API.
//!
//! Agents do not have a separate resource model. They use the same learner
//! bearer session and owner-scoped REST endpoints as the web application.

use serde_json::{Value, json};

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
            "/public/v1/onboarding/preview",
            json!({"type":"object","required":["prompt"],"properties":{"prompt":{"type":"string"}}}),
        ),
        endpoint(
            "learning.start",
            "Create or resume a learner account and bootstrap the first owner-scoped journey from one prompt.",
            "POST",
            "/public/v1/onboarding/start",
            json!({"type":"object","required":["email","displayName","prompt","idempotencyKey"],"properties":{"email":{"type":"string","format":"email"},"displayName":{"type":"string"},"prompt":{"type":"string"},"idempotencyKey":{"type":"string"}}}),
        ),
        endpoint(
            "learning.journey.list",
            "List the authenticated learner's journeys and the next activity for each.",
            "GET",
            "/api/v1/learning/journeys",
            empty.clone(),
        ),
        endpoint(
            "learning.journey.get",
            "Read one authenticated learner's intent, objectives, activities, and evidence.",
            "GET",
            "/api/v1/learning/journeys/{id}",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.activity.start",
            "Start an activity in an owned journey.",
            "POST",
            "/api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
            json!({"type":"object","required":["journeyId","activityId"],"properties":{"journeyId":{"type":"string","format":"uuid"},"activityId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.session.finish",
            "Finish a learning session and persist the learner's evidence and responses.",
            "POST",
            "/api/v1/learning/sessions/{id}/finish",
            json!({"type":"object","required":["id","completed","responses"],"properties":{"id":{"type":"string","format":"uuid"},"completed":{"type":"boolean"},"responses":{"type":"array"}}}),
        ),
        endpoint(
            "learning.question.create",
            "Create a learner-owned versioned question with explanation and provenance metadata.",
            "POST",
            "/api/v1/questions",
            json!({"type":"object","required":["kind","prompt","points"],"properties":{"kind":{"type":"string","enum":["multiple_choice","true_false","short_answer","essay","code"]},"prompt":{"type":"string"},"options":{"type":"array"},"acceptedAnswers":{"type":"array","items":{"type":"string"}},"explanation":{"type":"string"},"rationale":{"type":"string"},"points":{"type":"integer","minimum":1},"reviewStatus":{"type":"string"},"sourceReferences":{"type":"array","items":{"type":"string"}}}}),
        ),
        endpoint(
            "learning.question.get",
            "Read one exact learner-owned question version for reproducible assessment history.",
            "GET",
            "/api/v1/questions/{question_id}/versions/{version}",
            json!({"type":"object","required":["questionId","version"],"properties":{"questionId":{"type":"string","format":"uuid"},"version":{"type":"integer","minimum":1}}}),
        ),
        endpoint(
            "learning.question.revise",
            "Create the next immutable version of a learner-owned question.",
            "POST",
            "/api/v1/questions/{question_id}/versions",
            json!({"type":"object","required":["kind","prompt","points"],"properties":{"kind":{"type":"string"},"prompt":{"type":"string"},"points":{"type":"integer","minimum":1}}}),
        ),
        endpoint(
            "learning.assessment.create",
            "Compose approved question versions into a learner-owned practice or graded assessment.",
            "POST",
            "/api/v1/assessments",
            json!({"type":"object","required":["activityId","mode","items"],"properties":{"activityId":{"type":"string","format":"uuid"},"mode":{"type":"string","enum":["practice","graded"]},"items":{"type":"array"},"status":{"type":"string"}}}),
        ),
        endpoint(
            "learning.assessment.get",
            "Read one learner-owned assessment and its exact question-version membership.",
            "GET",
            "/api/v1/assessments/{assessment_id}",
            json!({"type":"object","required":["assessmentId"],"properties":{"assessmentId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.assessment.get_for_activity",
            "Discover the learner-owned assessment attached to an activity before starting it.",
            "GET",
            "/api/v1/assessments?activityId={activity_id}",
            json!({"type":"object","required":["activityId"],"properties":{"activityId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.attempt.start",
            "Start or resume an attempt against one immutable assessment version.",
            "POST",
            "/api/v1/assessments/{assessment_id}/attempts",
            json!({"type":"object","required":["assessmentId","learningSessionId"],"properties":{"assessmentId":{"type":"string","format":"uuid"},"learningSessionId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.attempt.answer",
            "Save one answer while preserving assessment item and question version identity.",
            "POST",
            "/api/v1/attempts/{attempt_id}/answers",
            json!({"type":"object","required":["attemptId","assessmentItemId","questionVersionId","response"],"properties":{"attemptId":{"type":"string","format":"uuid"},"assessmentItemId":{"type":"string","format":"uuid"},"questionVersionId":{"type":"string","format":"uuid"},"response":{"type":"object"}}}),
        ),
        endpoint(
            "learning.attempt.finish",
            "Finish an attempt, grade deterministic items, and return manual-review state when needed.",
            "POST",
            "/api/v1/attempts/{attempt_id}/finish",
            json!({"type":"object","required":["attemptId"],"properties":{"attemptId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.attempt.get",
            "Read an owned attempt and its saved responses/result.",
            "GET",
            "/api/v1/attempts/{attempt_id}",
            json!({"type":"object","required":["attemptId"],"properties":{"attemptId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.progress.evidence",
            "Record rebuildable mastery evidence linked to a learner activity or attempt.",
            "POST",
            "/api/v1/progress/evidence",
            json!({"type":"object","required":["journeyId","objectiveId","activityId","value","derivationVersion"],"properties":{"journeyId":{"type":"string","format":"uuid"},"objectiveId":{"type":"string","format":"uuid"},"activityId":{"type":"string","format":"uuid"},"attemptId":{"type":"string","format":"uuid"},"value":{"type":"number","minimum":0,"maximum":1},"derivationVersion":{"type":"integer","minimum":1}}}),
        ),
        endpoint(
            "learning.progress.snapshot",
            "Read evidence-derived mastery and confidence for one objective.",
            "GET",
            "/api/v1/progress/{journey_id}/objectives/{objective_id}",
            json!({"type":"object","required":["journeyId","objectiveId"],"properties":{"journeyId":{"type":"string","format":"uuid"},"objectiveId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.progress.recommend",
            "Select the weakest objective from an explicit journey objective set.",
            "POST",
            "/api/v1/progress/{journey_id}/recommendation",
            json!({"type":"object","required":["journeyId","objectives"],"properties":{"journeyId":{"type":"string","format":"uuid"},"objectives":{"type":"array"}}}),
        ),
        endpoint(
            "learning.deep_dive.create",
            "Create a source-backed explanatory deep dive linked to evidence.",
            "POST",
            "/api/v1/deep-dives",
            json!({"type":"object","required":["journeyId","activityId","objectiveId","triggeringEvidenceId","title","body","example","sourceReferences","applicationTask"],"properties":{"journeyId":{"type":"string","format":"uuid"},"activityId":{"type":"string","format":"uuid"},"objectiveId":{"type":"string","format":"uuid"},"triggeringEvidenceId":{"type":"string","format":"uuid"},"title":{"type":"string"},"body":{"type":"string"},"example":{"type":"string"},"sourceReferences":{"type":"array"},"applicationTask":{"type":"string"}}}),
        ),
        endpoint(
            "learning.deep_dive.get_for_activity",
            "Discover the source-backed deep dive attached to an activity.",
            "GET",
            "/api/v1/deep-dives?activityId={activity_id}",
            json!({"type":"object","required":["activityId"],"properties":{"activityId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.deep_dive.get",
            "Read one source-backed deep dive owned by the learner.",
            "GET",
            "/api/v1/deep-dives/{id}",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string","format":"uuid"}}}),
        ),
    ];

    json!({
        "schema_version": "v1",
        "name": "ame",
        "description": "ame — study, sweetened. An agent-friendly learning API with one owner-scoped learner model.",
        "auth": {
            "type": "bearer",
            "format": "ame_token",
            "registration": "POST /public/v1/auth/register followed by POST /public/v1/onboarding/start returns a learner bearer token; no separate integration identity is required."
        },
        "entrypoint": "/public/llms.txt",
        "tools": tools,
        "principles": [
            "Agents and people use the same authenticated learner API.",
            "Every learning resource is scoped to the bearer token's learner.",
            "The API never sends email; email is an account identifier in self-host mode.",
            "Use idempotencyKey when starting a journey."
        ]
    })
}
