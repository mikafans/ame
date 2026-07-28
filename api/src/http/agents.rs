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
            "identity.register",
            "Register a learner account and receive the bearer session used by the unified learning API. Self-host mode does not send email.",
            "POST",
            "/public/v1/auth/register",
            json!({"type":"object","required":["email","name","password"],"properties":{"email":{"type":"string","format":"email"},"name":{"type":"string"},"password":{"type":"string","minLength":8}}}),
        ),
        endpoint(
            "identity.login",
            "Authenticate an existing learner account and receive the bearer session used by the unified learning API.",
            "POST",
            "/public/v1/auth/login",
            json!({"type":"object","required":["email","password"],"properties":{"email":{"type":"string","format":"email"},"password":{"type":"string"}}}),
        ),
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
            "Read one authenticated learner's intent, objectives, activities, evidence, and the same evidence-backed recommendation returned to the browser. The recommendation includes objectiveId and server-derived evidenceIds.",
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
            "learning.source.import",
            "Import bounded learner-owned source text as an immutable SHA-256 snapshot. URL imports accept public HTTPS only; document and local-file imports accept agent-supplied text and never read server filesystem paths.",
            "POST",
            "/api/v1/sources/imports",
            json!({"type":"object","required":["kind","locator","retryKey"],"properties":{"kind":{"type":"string","enum":["url","document","local_file"]},"locator":{"type":"string"},"mediaType":{"type":"string","enum":["text/plain","text/markdown","text/html","application/json"]},"content":{"type":"string","description":"Required for document and local_file; ignored for URL."},"retryKey":{"type":"string"}}}),
        ),
        endpoint(
            "learning.source.import.list",
            "Inspect completed and failed source import runs for the authenticated learner, including stable failure codes.",
            "GET",
            "/api/v1/source-imports",
            json!({"type":"object","properties":{}}),
        ),
        endpoint(
            "learning.source.snapshot.list",
            "List the authenticated learner's immutable source snapshots.",
            "GET",
            "/api/v1/source-snapshots",
            empty.clone(),
        ),
        endpoint(
            "learning.source.snapshot.get",
            "Read one authenticated learner's immutable source snapshot and digest.",
            "GET",
            "/api/v1/source-snapshots/{snapshot_id}",
            json!({"type":"object","required":["snapshotId"],"properties":{"snapshotId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.session.finish",
            "Finish a learning session and persist the learner's evidence and responses.",
            "POST",
            "/api/v1/learning/sessions/{id}/finish",
            json!({"type":"object","required":["id","completed","responses"],"properties":{"id":{"type":"string","format":"uuid"},"completed":{"type":"boolean"},"responses":{"type":"array"}}}),
        ),
        endpoint(
            "learning.session.get",
            "Read one authenticated learner's activity session so an external agent can resume the same state as the browser.",
            "GET",
            "/api/v1/learning/sessions/{id}",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.task.submission.start",
            "Start a submission for an application task at its immutable content version. The response is not evidence until evaluation reaches a reviewed state.",
            "POST",
            "/api/v1/tasks/{task_id}/submissions",
            json!({"type":"object","required":["taskId","contentVersion","response"],"properties":{"taskId":{"type":"string","format":"uuid"},"contentVersion":{"type":"integer","minimum":1},"response":{"type":"object"},"evaluationMethod":{"type":"string","enum":["self_review","automatic","agent","manual"]}}}),
        ),
        endpoint(
            "learning.task.submission.submit",
            "Submit an application-task response for evaluation. It may become pending review rather than immediately producing a score.",
            "POST",
            "/api/v1/task-submissions/{submission_id}/submit",
            json!({"type":"object","required":["submissionId"],"properties":{"submissionId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.task.submission.get",
            "Read the learner's task submission status, score, and reviewer or agent feedback.",
            "GET",
            "/api/v1/task-submissions/{submission_id}",
            json!({"type":"object","required":["submissionId"],"properties":{"submissionId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.activity.content.author",
            "Replace an uncompleted explanation or worked example with reviewed, source-backed content from a published learning.activity.content.compose generation run.",
            "PATCH",
            "/api/v1/learning/activities/{activity_id}/content",
            json!({"type":"object","required":["activityId","generationRunId","content","sourceReferences","reviewStatus"],"properties":{"activityId":{"type":"string","format":"uuid"},"generationRunId":{"type":"string","format":"uuid"},"content":{"type":"object","required":["type"],"description":"The content type must match the activity kind: explanation uses heading, body, key_points; worked_example uses heading, prompt, steps, reflection. Text and list items must be non-empty strings."},"sourceReferences":{"type":"array","items":{"type":"string"},"minItems":1},"reviewStatus":{"type":"string","enum":["approved"],"description":"Only approved content is learner-visible."}}}),
        ),
        endpoint(
            "learning.activity.rubric.author",
            "Attach a structured scoring rubric to a task/application activity from a published learning.activity.rubric.compose generation run. Criteria and points guide manual/agent review; learners see the criteria. Provenance is mandatory.",
            "PATCH",
            "/api/v1/learning/activities/{activity_id}/rubric",
            json!({"type":"object","required":["activityId","generationRunId","rubric","sourceReferences","reviewStatus"],"properties":{"activityId":{"type":"string","format":"uuid"},"generationRunId":{"type":"string","format":"uuid"},"rubric":{"type":"object","required":["version","criteria"],"description":"Structured rubric: version, criteria[] (each id, objectiveId, description, maxPoints>=1, required), optional passingScore 0..1.","properties":{"version":{"type":"integer","minimum":1},"criteria":{"type":"array","minItems":1,"items":{"type":"object","required":["id","objectiveId","description","maxPoints","required"],"properties":{"id":{"type":"string"},"objectiveId":{"type":"string","format":"uuid"},"description":{"type":"string"},"maxPoints":{"type":"integer","minimum":1},"required":{"type":"boolean"}}}},"passingScore":{"type":"number","minimum":0,"maximum":1}}},"sourceReferences":{"type":"array","items":{"type":"string"},"minItems":1},"reviewStatus":{"type":"string","enum":["approved"],"description":"Only approved rubrics are learner-visible."}}}),
        ),
        endpoint(
            "learning.question.create",
            "Create a learner-owned versioned question from a published question.compose generation run; provenance is mandatory.",
            "POST",
            "/api/v1/questions",
            json!({"type":"object","required":["generationRunId","kind","prompt","points"],"properties":{"generationRunId":{"type":"string","format":"uuid"},"kind":{"type":"string","enum":["multiple_choice","true_false","short_answer","numeric","essay","code"]},"prompt":{"type":"string"},"options":{"type":"array"},"acceptedAnswers":{"type":"array","items":{"type":"string"}},"explanation":{"type":"string"},"rationale":{"type":"string"},"difficulty":{"type":"string"},"points":{"type":"integer","minimum":1},"reviewStatus":{"type":"string"},"sourceReferences":{"type":"array","items":{"type":"string"}}}}),
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
            "Create the next immutable question version from a published question.compose generation run.",
            "POST",
            "/api/v1/questions/{question_id}/versions",
            json!({"type":"object","required":["generationRunId","kind","prompt","points"],"properties":{"generationRunId":{"type":"string","format":"uuid"},"kind":{"type":"string"},"prompt":{"type":"string"},"difficulty":{"type":"string"},"points":{"type":"integer","minimum":1}}}),
        ),
        endpoint(
            "learning.assessment.create",
            "Compose approved question versions into a learner-owned practice or graded assessment.",
            "POST",
            "/api/v1/assessments",
            json!({"type":"object","required":["activityId","mode","items"],"properties":{"activityId":{"type":"string","format":"uuid"},"mode":{"type":"string","enum":["practice","graded"]},"items":{"type":"array","items":{"type":"object","required":["objectiveId","questionId","questionVersion","orderIndex","points"],"properties":{"objectiveId":{"type":"string","format":"uuid"},"questionId":{"type":"string","format":"uuid"},"questionVersion":{"type":"integer"},"orderIndex":{"type":"integer"},"points":{"type":"integer","minimum":1}}}},"status":{"type":"string"}}}),
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
            "Finish an attempt, grade deterministic items, and return assessmentMode plus reviewStatus (complete or pending) so practice results cannot be mistaken for an exam result.",
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
            "learning.attempt.history",
            "Read all assessment attempts in one learner-owned journey, including durable results.",
            "GET",
            "/api/v1/learning/journeys/{journey_id}/attempts",
            json!({"type":"object","required":["journeyId"],"properties":{"journeyId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.progress.evidence",
            "Record rebuildable mastery evidence from a completed learner attempt.",
            "POST",
            "/api/v1/progress/evidence",
            json!({"type":"object","required":["journeyId","objectiveId","activityId","attemptId","value","derivationVersion"],"properties":{"journeyId":{"type":"string","format":"uuid"},"objectiveId":{"type":"string","format":"uuid"},"activityId":{"type":"string","format":"uuid"},"attemptId":{"type":"string","format":"uuid"},"value":{"type":"number","minimum":0,"maximum":1},"derivationVersion":{"type":"integer","minimum":1}}}),
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
            "learning.progress.streak.record",
            "Record an idempotent qualifying learning day from a completed attempt event (attempt:<id>); the day must match the attempt's graded timestamp in the supplied IANA learner timezone.",
            "POST",
            "/api/v1/progress/streaks",
            json!({"type":"object","required":["journeyId","activityId","qualifyingEventKey","learnerTimezone","qualifyingDay"],"properties":{"journeyId":{"type":"string","format":"uuid"},"activityId":{"type":"string","format":"uuid"},"qualifyingEventKey":{"type":"string"},"learnerTimezone":{"type":"string"},"qualifyingDay":{"type":"string","format":"date"}}}),
        ),
        endpoint(
            "learning.progress.streak.list",
            "Read qualifying learning days for one owned journey.",
            "GET",
            "/api/v1/progress/{journey_id}/streaks",
            json!({"type":"object","required":["journeyId"],"properties":{"journeyId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.progress.timeline",
            "Read the ordered activity, assessment, evidence, deep-dive, and streak history for one owned journey.",
            "GET",
            "/api/v1/progress/{journey_id}/timeline",
            json!({"type":"object","required":["journeyId"],"properties":{"journeyId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.deep_dive.create",
            "Create a source-backed deep dive from a published deep_dive.create generation run, linked to matching owned evidence. Unreviewed content is stored for review but is not learner-readable until approved.",
            "POST",
            "/api/v1/deep-dives",
            json!({"type":"object","required":["generationRunId","journeyId","activityId","objectiveId","triggeringEvidenceId","title","body","example","sourceReferences","applicationTask"],"properties":{"generationRunId":{"type":"string","format":"uuid"},"journeyId":{"type":"string","format":"uuid"},"activityId":{"type":"string","format":"uuid"},"objectiveId":{"type":"string","format":"uuid"},"triggeringEvidenceId":{"type":"string","format":"uuid"},"title":{"type":"string"},"body":{"type":"string"},"example":{"type":"string"},"sourceReferences":{"type":"array"},"applicationTask":{"type":"string"},"reviewStatus":{"type":"string","enum":["draft","review","approved","rejected"],"description":"Content is returned by learner reads only when approved."}}}),
        ),
        endpoint(
            "learning.deep_dive.get_for_activity",
            "Discover the approved source-backed deep dive attached to an activity. Draft and review content is not returned.",
            "GET",
            "/api/v1/deep-dives?activityId={activity_id}",
            json!({"type":"object","required":["activityId"],"properties":{"activityId":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.deep_dive.get",
            "Read one approved source-backed deep dive owned by the learner; draft and review content is not returned.",
            "GET",
            "/api/v1/deep-dives/{id}",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.generation.start",
            "Start or resume an owner-scoped provider generation run with an idempotent retry key.",
            "POST",
            "/api/v1/generation-runs",
            json!({"type":"object","required":["operation"],"properties":{"operation":{"type":"string"},"provider":{"type":"string"},"retryKey":{"type":"string"},"contentVersion":{"type":"integer","minimum":1}}}),
        ),
        endpoint(
            "learning.generation.get",
            "Read the status, provider, failure detail, and content version of an owned generation run.",
            "GET",
            "/api/v1/generation-runs/{id}",
            json!({"type":"object","required":["id"],"properties":{"id":{"type":"string","format":"uuid"}}}),
        ),
        endpoint(
            "learning.generation.transition",
            "Advance an owned generation run or record a provider failure; failed runs cannot be published.",
            "PATCH",
            "/api/v1/generation-runs/{id}",
            json!({"type":"object","required":["id","status"],"properties":{"id":{"type":"string","format":"uuid"},"status":{"type":"string","enum":["requested","running","review_required","published","failed"]},"error":{"type":"object"}}}),
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
