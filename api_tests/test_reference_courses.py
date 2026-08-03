"""Published, source-grounded reference courses exercised through the public API."""

import uuid

import pytest


def request(client, method, path, headers, **kwargs):
    response = getattr(client, method)(path, headers=headers, **kwargs)
    assert response.is_success, response.text
    return response.json()


def register_author(client, course):
    account = request(
        client,
        "post",
        "/public/v1/auth/register",
        {},
        json={
            "email": f"reference-{course['slug']}-{uuid.uuid4()}@example.test",
            "name": f"{course['title']} author",
            "password": "reference-course-2026",
        },
    )
    return {"Authorization": f"Bearer {account['token']}"}


def source_reference(client, headers, course):
    content = course["source"]
    source = request(
        client,
        "post",
        "/api/v1/sources/imports",
        headers,
        json={
            "kind": "document",
            "locator": f"reference://{course['slug']}/course-notes.txt",
            "mediaType": "text/plain",
            "content": content,
            "retryKey": f"{course['slug']}-source-{uuid.uuid4()}",
        },
    )
    citation = request(
        client,
        "post",
        "/api/v1/citations",
        headers,
        json={
            "snapshotId": source["id"],
            "startByte": 0,
            "endByte": len(content.encode()),
            "quote": content,
            "extractionMethod": "exact_quote",
            "groundingStatus": "supported",
            "groundingNote": "Reference-course source package supports its authored material.",
            "licenseStatus": "allowed",
            "licenseName": "Reference-course test material",
        },
    )
    return citation["id"]


def published_run(client, headers, operation, slug):
    run = request(
        client,
        "post",
        "/api/v1/generation-runs",
        headers,
        json={
            "operation": operation,
            "provider": "reference-course-test",
            "retryKey": f"{slug}-{operation}-{uuid.uuid4()}",
            "contentVersion": 1,
        },
    )
    for status in ("running", "published"):
        request(
            client,
            "patch",
            f"/api/v1/generation-runs/{run['id']}",
            headers,
            json={"status": status},
        )
    return run["id"]


def question(client, headers, course, reference, number):
    run = published_run(client, headers, "question.compose", course["slug"])
    module = course["modules"][number]
    return request(
        client,
        "post",
        "/api/v1/questions",
        headers,
        json={
            "generationRunId": run,
            "kind": "multiple_choice",
            "prompt": module["check"],
            "options": [
                {"id": "incorrect", "text": module["distractor"], "is_correct": False},
                {"id": "correct", "text": module["answer"], "is_correct": True},
            ],
            "points": 1,
            "reviewStatus": "approved",
            "rationale": module["rationale"],
            "explanation": module["feedback"],
            "difficulty": "intermediate",
            "sourceReferences": [reference],
        },
    )


def build_reference_course(client, course, headers=None):
    """Author and publish one reference course with the supplied authority.

    Tests use a fresh learner by default.  Local-stack fixtures supply a
    scoped delegation so the course is authored for an existing learner.
    """
    if headers is None:
        headers = register_author(client, course)
    reference = source_reference(client, headers, course)
    created = request(
        client,
        "post",
        "/api/v1/learning/courses",
        headers,
        json={
            "rawIntent": course["intent"],
            "idempotencyKey": f"reference-{course['slug']}-{uuid.uuid4()}",
            "brief": {
                "title": course["title"],
                "audience": course["audience"],
                "estimatedMinutes": 240,
                "prerequisites": course["prerequisites"],
                "outcomes": [module["outcome"] for module in course["modules"]],
                "modules": [module["title"] for module in course["modules"]],
            },
            "sourceReferences": [reference],
        },
    )
    journey_id = created["journeyId"]
    revision_id = created["revision"]["id"]
    objectives = []
    for module in course["modules"]:
        objectives.append(
            request(
                client,
                "post",
                f"/api/v1/learning/journeys/{journey_id}/objectives",
                headers,
                json={
                    "revisionId": revision_id,
                    "verb": "Design"
                    if "design" in module["outcome"].lower()
                    else "Explain",
                    "statement": module["outcome"],
                    "successCriteria": module["success"],
                },
            )
        )

    content_run = published_run(
        client, headers, "learning.activity.content.compose", course["slug"]
    )
    explanation_activities = []
    authored_activities = []
    for number, module in enumerate(course["modules"]):
        objective = objectives[number]
        chapter = request(
            client,
            "post",
            f"/api/v1/learning/journeys/{journey_id}/chapters",
            headers,
            json={
                "revisionId": revision_id,
                "title": module["title"],
                "summary": module["summary"],
            },
        )
        status = "ready" if number == 0 else "proposed"
        explanation = request(
            client,
            "post",
            f"/api/v1/learning/journeys/{journey_id}/activities",
            headers,
            json={
                "revisionId": revision_id,
                "chapterId": chapter["id"],
                "kind": "explanation",
                "title": f"{module['title']}: key idea",
                "payload": {},
                "objectiveIds": [objective["id"]],
                "status": status,
            },
        )
        example = request(
            client,
            "post",
            f"/api/v1/learning/journeys/{journey_id}/activities",
            headers,
            json={
                "revisionId": revision_id,
                "chapterId": chapter["id"],
                "kind": "example",
                "title": f"{module['title']}: worked example",
                "payload": {},
                "objectiveIds": [objective["id"]],
                "status": "proposed",
            },
        )
        explanation = request(
            client,
            "patch",
            f"/api/v1/learning/activities/{explanation['id']}/content",
            headers,
            json={
                "revisionId": revision_id,
                "generationRunId": content_run,
                "content": {
                    "type": "explanation",
                    "heading": f"{module['title']}: key idea",
                    "body": module["instruction"],
                    "key_points": module["key_points"],
                },
                "sourceReferences": [reference],
                "reviewStatus": "approved",
            },
        )
        request(
            client,
            "patch",
            f"/api/v1/learning/activities/{example['id']}/content",
            headers,
            json={
                "revisionId": revision_id,
                "generationRunId": content_run,
                "content": {
                    "type": "worked_example",
                    "heading": f"{module['title']}: worked example",
                    "prompt": module["example_prompt"],
                    "steps": module["example_steps"],
                    "reflection": module["reflection"],
                },
                "sourceReferences": [reference],
                "reviewStatus": "approved",
            },
        )
        formative = question(client, headers, course, reference, number)
        request(
            client,
            "post",
            "/api/v1/assessments",
            headers,
            json={
                "revisionId": revision_id,
                "activityId": explanation["id"],
                "mode": "practice",
                "status": "published",
                "items": [
                    {
                        "objectiveId": objective["id"],
                        "questionId": formative["questionId"],
                        "questionVersion": formative["version"],
                        "orderIndex": 0,
                        "points": 1,
                    }
                ],
            },
        )
        explanation_activities.append((explanation, objective))
        authored_activities.extend((explanation, example))

    mastery_activity = request(
        client,
        "post",
        f"/api/v1/learning/journeys/{journey_id}/activities",
        headers,
        json={
            "revisionId": revision_id,
            "chapterId": None,
            "kind": "practice",
            "title": f"{course['title']} mastery assessment",
            "payload": {"prompt": course["mastery_prompt"]},
            "objectiveIds": [objective["id"] for objective in objectives],
            "status": "proposed",
        },
    )
    mastery_question = question(client, headers, course, reference, 0)
    request(
        client,
        "post",
        "/api/v1/assessments",
        headers,
        json={
            "revisionId": revision_id,
            "activityId": mastery_activity["id"],
            "mode": "graded",
            "status": "published",
            "items": [
                {
                    "objectiveId": objectives[0]["id"],
                    "questionId": mastery_question["questionId"],
                    "questionVersion": mastery_question["version"],
                    "orderIndex": 0,
                    "points": 1,
                }
            ],
        },
    )
    application = request(
        client,
        "post",
        f"/api/v1/learning/journeys/{journey_id}/activities",
        headers,
        json={
            "revisionId": revision_id,
            "chapterId": None,
            "kind": "application",
            "title": course["task_title"],
            "payload": {"prompt": course["task_prompt"]},
            "objectiveIds": [objective["id"] for objective in objectives],
            "status": "proposed",
        },
    )
    rubric_run = published_run(
        client, headers, "learning.activity.rubric.compose", course["slug"]
    )
    request(
        client,
        "patch",
        f"/api/v1/learning/activities/{application['id']}/rubric",
        headers,
        json={
            "revisionId": revision_id,
            "generationRunId": rubric_run,
            "rubric": {
                "version": 1,
                "passingScore": 0.7,
                "criteria": [
                    {
                        "id": f"module-{number + 1}",
                        "objectiveId": objective["id"],
                        "description": module["success"],
                        "maxPoints": 1,
                        "required": True,
                    }
                    for number, (objective, module) in enumerate(
                        zip(objectives, course["modules"], strict=True)
                    )
                ],
            },
            "sourceReferences": [reference],
            "reviewStatus": "approved",
        },
    )
    authored_activities.extend((mastery_activity, application))

    for activity in authored_activities:
        request(
            client,
            "post",
            f"/api/v1/learning/activities/{activity['id']}/review",
            headers,
            json={"revisionId": revision_id},
        )
    validated = request(
        client,
        "post",
        f"/api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/validate",
        headers,
    )
    assert validated["validation"] == []
    request(
        client,
        "post",
        f"/api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/review",
        headers,
        json={
            "review": {
                "reviewer": "reference-course-contract",
                "decision": "approved",
                "notes": "All required lesson, check, mastery, and rubric links are present.",
            }
        },
    )
    published = request(
        client,
        "post",
        f"/api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/publish",
        headers,
    )
    assert published["status"] == "published"

    return {
        "headers": headers,
        "journeyId": journey_id,
        "revisionId": revision_id,
        "first": explanation_activities[0][0],
        "firstObjective": explanation_activities[0][1],
        "masteryActivity": mastery_activity,
        "application": application,
    }


COURSES = [
    {
        "slug": "flink-clickstream",
        "title": "Reliable clickstream aggregation with Apache Flink",
        "intent": "In four weeks, design a reliable clickstream aggregation service using Apache Flink event time, watermarks, checkpoints, and the Kubernetes Operator.",
        "audience": "Backend engineers who can study four hours each week.",
        "prerequisites": [
            "Java or Scala",
            "basic streaming concepts",
            "Kubernetes basics",
        ],
        "source": "Event-time windows order events by their timestamps. Watermarks communicate bounded event-time progress. Checkpoints create recoverable state snapshots. The Kubernetes Operator reconciles a declared Flink deployment.",
        "mastery_prompt": "Choose the failure-safe design decision for the clickstream service.",
        "task_title": "Design a late-event clickstream window",
        "task_prompt": "Describe event-time extraction, watermark policy, window state, recovery behavior, and Kubernetes Operator deployment for a clickstream aggregation service.",
        "modules": [
            {
                "title": "Event-time windows",
                "summary": "Model click arrivals by event time rather than processing delay.",
                "outcome": "Explain how event-time windows group delayed clickstream events.",
                "success": "Names event timestamp assignment and a window boundary.",
                "instruction": "Event time lets the aggregation group a click by when it occurred, even when transport delay changes arrival order.",
                "key_points": [
                    "Assign timestamps at ingestion.",
                    "Window on event time.",
                ],
                "example_prompt": "Count clicks for a five-minute event-time window when one click arrives late.",
                "example_steps": [
                    "Assign the click timestamp.",
                    "Place it in its event-time window.",
                    "Decide whether lateness policy accepts it.",
                ],
                "reflection": "Which clock did the calculation use?",
                "check": "Which timestamp should a clickstream window use to tolerate transport delay?",
                "distractor": "The time the task manager receives the record.",
                "answer": "The timestamp at which the click event occurred.",
                "rationale": "Processing time measures arrival, not the business event, so it changes with transport delay.",
                "feedback": "Correct: event time preserves the intended window even when records arrive out of order.",
            },
            {
                "title": "State and checkpoint recovery",
                "summary": "Make window state recoverable after a task failure.",
                "outcome": "Explain how a checkpoint restores consistent aggregation state after failure.",
                "success": "Connects state snapshots to restart recovery.",
                "instruction": "A successful checkpoint captures operator state so a restarted job can resume from a consistent cut rather than recounting arbitrary records.",
                "key_points": [
                    "State is operator-owned.",
                    "Recovery starts from a completed checkpoint.",
                ],
                "example_prompt": "Recover a keyed click counter after a task manager fails during a window.",
                "example_steps": [
                    "Identify the latest completed checkpoint.",
                    "Restore keyed state.",
                    "Resume input from the aligned position.",
                ],
                "reflection": "Why is an incomplete checkpoint unsuitable?",
                "check": "What makes a Flink aggregation restart from a consistent state?",
                "distractor": "Replaying records without restoring operator state.",
                "answer": "Restoring operator state from a completed checkpoint.",
                "rationale": "A completed checkpoint pairs state with a recoverable input position.",
                "feedback": "Correct: the restart uses the completed checkpoint as its durable recovery boundary.",
            },
            {
                "title": "Watermarks and late data",
                "summary": "Choose an explicit progress and lateness policy.",
                "outcome": "Design a watermark policy for delayed clickstream events.",
                "success": "States bounded out-of-orderness and a late-event decision.",
                "instruction": "A watermark states how far event time has progressed; late-event handling must be explicit so the system trades latency against completeness deliberately.",
                "key_points": [
                    "Watermarks estimate event-time progress.",
                    "Late data needs a declared path.",
                ],
                "example_prompt": "Set a two-minute out-of-orderness policy for a five-minute click window.",
                "example_steps": [
                    "Estimate normal delay.",
                    "Emit a bounded watermark.",
                    "Route records beyond allowed lateness.",
                ],
                "reflection": "What happens when delay exceeds the bound?",
                "check": "What does a watermark communicate to an event-time operator?",
                "distractor": "The exact processing time of the next record.",
                "answer": "The event-time progress the operator can use to close windows.",
                "rationale": "Watermarks are progress signals, not guarantees about wall-clock arrival.",
                "feedback": "Correct: watermark progress drives event-time completion decisions.",
            },
            {
                "title": "Kubernetes Operator deployment",
                "summary": "Operate the job through a declared deployment and recovery policy.",
                "outcome": "Design a Flink Kubernetes Operator deployment with observable recovery behavior.",
                "success": "Names the deployment declaration, checkpoint storage, and failure observation.",
                "instruction": "The Kubernetes Operator reconciles a declared Flink deployment; durable checkpoint storage and observable job state make recovery an operational design instead of an ad-hoc restart.",
                "key_points": [
                    "Declare the job deployment.",
                    "Keep checkpoints in durable storage.",
                ],
                "example_prompt": "Plan an operator-managed upgrade of a checkpointed clickstream job.",
                "example_steps": [
                    "Declare deployment settings.",
                    "Verify durable checkpoint access.",
                    "Observe reconciliation and job health.",
                ],
                "reflection": "Which signal would prove recovery completed?",
                "check": "Which deployment concern preserves recoverability across a Kubernetes restart?",
                "distractor": "Keeping checkpoints only on an ephemeral task manager disk.",
                "answer": "Using durable checkpoint storage reachable after restart.",
                "rationale": "Ephemeral local state disappears with the failed workload.",
                "feedback": "Correct: durable checkpoint storage is required for an operator-managed recovery path.",
            },
        ],
    },
    {
        "slug": "netty-tcp-service",
        "title": "Asynchronous TCP service design with Netty",
        "intent": "Over three weeks, design and debug an asynchronous TCP protocol service with Netty event loops, channel pipelines, ByteBuf ownership, backpressure, and graceful shutdown.",
        "audience": "Java service maintainers who can study thirty minutes each day.",
        "prerequisites": [
            "Java",
            "TCP basics",
            "familiarity with asynchronous callbacks",
        ],
        "source": "An event loop owns channel I/O. A channel pipeline orders handlers. ByteBuf reference counts require explicit ownership. Backpressure protects a service when writes cannot keep up. Graceful shutdown drains or closes work deliberately.",
        "mastery_prompt": "Choose the safe asynchronous TCP-service design decision.",
        "task_title": "Design a backpressured Netty pipeline",
        "task_prompt": "Describe a channel pipeline, ByteBuf ownership boundaries, backpressure behavior, event-loop work, and graceful shutdown for an asynchronous TCP protocol service.",
        "modules": [
            {
                "title": "Event loops and channel ownership",
                "summary": "Keep channel I/O and short callbacks on the owning event loop.",
                "outcome": "Explain how a Netty event loop owns channel I/O work.",
                "success": "Separates channel I/O from blocking application work.",
                "instruction": "An event loop serializes channel I/O callbacks; blocking work there delays every channel sharing that loop.",
                "key_points": [
                    "Event loops own channel callbacks.",
                    "Move blocking work away from I/O loops.",
                ],
                "example_prompt": "Handle a decoded request without blocking the channel event loop.",
                "example_steps": [
                    "Decode on the pipeline.",
                    "Offload blocking work.",
                    "Write the response on the channel context.",
                ],
                "reflection": "Which channels are delayed by a blocking handler?",
                "check": "Why should a blocking database call not run on a Netty event loop?",
                "distractor": "It makes TCP packets arrive in order.",
                "answer": "It delays I/O callbacks for channels assigned to that loop.",
                "rationale": "The event loop is responsible for progressing channel I/O, not waiting on blocking work.",
                "feedback": "Correct: keep event-loop callbacks short to preserve I/O progress.",
            },
            {
                "title": "Channel pipeline composition",
                "summary": "Order decoding, protocol validation, and business handlers explicitly.",
                "outcome": "Design a channel pipeline that separates framing, decoding, and protocol handling.",
                "success": "Names an ordered inbound path with a protocol boundary.",
                "instruction": "A channel pipeline composes handlers in order, so framing and decoding should happen before application protocol decisions.",
                "key_points": [
                    "Handlers run in pipeline order.",
                    "Decode before business logic.",
                ],
                "example_prompt": "Build an inbound pipeline for length-prefixed protocol messages.",
                "example_steps": [
                    "Frame bytes into messages.",
                    "Decode the message.",
                    "Validate and handle the protocol command.",
                ],
                "reflection": "Where should malformed frames stop?",
                "check": "Which pipeline stage should turn a byte frame into a protocol message?",
                "distractor": "The business command handler before framing.",
                "answer": "A decoder placed after framing and before the business handler.",
                "rationale": "Business logic needs a decoded message, not an arbitrary byte stream.",
                "feedback": "Correct: framing and decoding define a clean protocol boundary.",
            },
            {
                "title": "ByteBuf ownership",
                "summary": "Retain or release buffers at explicit handoff boundaries.",
                "outcome": "Explain safe ByteBuf ownership across asynchronous handler boundaries.",
                "success": "Identifies retain or release at a handoff.",
                "instruction": "ByteBuf instances are reference-counted; a handler that keeps a buffer beyond the current callback must retain it and later release it.",
                "key_points": [
                    "Ownership is reference-counted.",
                    "Async handoff needs explicit retain and release.",
                ],
                "example_prompt": "Pass a decoded ByteBuf to asynchronous work without using freed memory.",
                "example_steps": [
                    "Retain before the handoff.",
                    "Use it in asynchronous work.",
                    "Release when the consumer finishes.",
                ],
                "reflection": "Who releases the retained buffer?",
                "check": "What must a handler do before retaining a ByteBuf for asynchronous work?",
                "distractor": "Assume the pipeline will preserve it indefinitely.",
                "answer": "Retain it and define the later release owner.",
                "rationale": "Reference counting makes ownership explicit across callback lifetimes.",
                "feedback": "Correct: retaining without a release leaks, and using without retain risks invalid memory.",
            },
            {
                "title": "Backpressure and graceful shutdown",
                "summary": "Protect the service when outbound writes and shutdown race with demand.",
                "outcome": "Design backpressure and graceful shutdown behavior for an asynchronous TCP service.",
                "success": "Connects writability to demand control and names a shutdown drain decision.",
                "instruction": "Backpressure observes whether a channel can accept more writes; graceful shutdown must stop new work and drain or close outstanding work by a declared policy.",
                "key_points": [
                    "Channel writability informs demand.",
                    "Shutdown needs a drain or close policy.",
                ],
                "example_prompt": "Pause request intake when a channel becomes non-writable, then close gracefully.",
                "example_steps": [
                    "Observe writability.",
                    "Pause or limit producers.",
                    "Stop accepting new work and drain bounded writes.",
                ],
                "reflection": "What work is allowed after shutdown starts?",
                "check": "What signal should a Netty service use to reduce outbound pressure?",
                "distractor": "The number of Java source files in the service.",
                "answer": "Channel writability and bounded queued work.",
                "rationale": "Writability exposes whether writes are making progress.",
                "feedback": "Correct: backpressure must react to transport capacity, not an unrelated metric.",
            },
        ],
    },
]


@pytest.mark.parametrize("course", COURSES, ids=[course["slug"] for course in COURSES])
def test_reference_courses_publish_and_adapt_after_weak_evidence(client, course):
    built = build_reference_course(client, course)
    learner = request(
        client,
        "get",
        f"/api/v1/learning/journeys/{built['journeyId']}",
        built["headers"],
    )
    assert len(learner["chapters"]) == 4
    assert len(learner["objectives"]) == 4
    assert all(
        activity["publicationStatus"] == "published"
        for activity in learner["activities"]
    )

    session = request(
        client,
        "post",
        f"/api/v1/learning/journeys/{built['journeyId']}/activities/{built['first']['id']}/start",
        built["headers"],
    )
    formative = request(
        client,
        "get",
        "/api/v1/assessments",
        built["headers"],
        params={"activityId": built["first"]["id"]},
    )
    attempt = request(
        client,
        "post",
        f"/api/v1/assessments/{formative['id']}/attempts",
        built["headers"],
        json={"learningSessionId": session["id"]},
    )
    assert attempt["items"][0]["assessmentItemId"]
    assert attempt["items"][0]["questionVersionId"]
    assert attempt["items"][0]["evaluationStatus"] == "unanswered"
    item = formative["items"][0]
    request(
        client,
        "post",
        f"/api/v1/attempts/{attempt['id']}/answers",
        built["headers"],
        json={
            "assessmentItemId": item["id"],
            "questionVersionId": item["questionVersionId"],
            "response": {"option_id": "incorrect"},
        },
    )
    finished = request(
        client,
        "post",
        f"/api/v1/attempts/{attempt['id']}/finish",
        built["headers"],
    )
    assert finished["score"] == 0
    assert finished["items"][0]["evaluationStatus"] == "incorrect"
    assert finished["items"][0]["explanation"] == course["modules"][0]["feedback"]
    assert finished["items"][0]["rationale"] == course["modules"][0]["rationale"]
    evidence = request(
        client,
        "post",
        "/api/v1/progress/evidence",
        built["headers"],
        json={
            "journeyId": built["journeyId"],
            "objectiveId": built["firstObjective"]["id"],
            "activityId": built["first"]["id"],
            "attemptId": attempt["id"],
            "contentVersion": built["first"]["contentVersion"],
            "value": 0,
            "derivationVersion": 1,
        },
    )
    recommendation = request(
        client,
        "post",
        f"/api/v1/progress/{built['journeyId']}/recommendation",
        built["headers"],
        json={
            "objectives": [
                {
                    "objectiveId": built["firstObjective"]["id"],
                    "activityId": built["first"]["id"],
                }
            ]
        },
    )
    assert recommendation["objectiveId"] == built["firstObjective"]["id"]
    assert evidence["id"] in recommendation["evidenceIds"]

    fork = request(
        client,
        "post",
        f"/api/v1/learning/journeys/{built['journeyId']}/course-revisions/{built['revisionId']}/fork",
        built["headers"],
    )
    assert fork["status"] == "draft"
    assert fork["version"] == 2
    fork_validation = request(
        client,
        "post",
        f"/api/v1/learning/journeys/{built['journeyId']}/course-revisions/{fork['id']}/validate",
        built["headers"],
    )
    assert fork_validation["validation"] == []
    learner_after_fork = request(
        client,
        "get",
        f"/api/v1/learning/journeys/{built['journeyId']}",
        built["headers"],
    )
    assert learner_after_fork["activities"][0]["id"] == built["first"]["id"]


def test_learner_views_are_published_course_scoped_and_owner_scoped(client):
    headers = register_author(client, COURSES[0])
    flink = build_reference_course(client, COURSES[0], headers)
    netty = build_reference_course(client, COURSES[1], headers)
    draft_reference = source_reference(client, headers, COURSES[0])
    draft = request(
        client,
        "post",
        "/api/v1/learning/courses",
        headers,
        json={
            "rawIntent": "Draft course must not enter a learner library.",
            "idempotencyKey": f"draft-learner-view-{uuid.uuid4()}",
            "brief": {
                "title": "Unpublished draft course",
                "audience": "Test learner",
                "estimatedMinutes": 30,
                "prerequisites": [],
                "outcomes": ["Inspect publication boundaries"],
                "modules": ["Draft module"],
            },
            "sourceReferences": [draft_reference],
        },
    )

    library = request(client, "get", "/api/v1/learning/courses", headers)
    assert [course["journeyId"] for course in library] == [
        netty["journeyId"],
        flink["journeyId"],
    ]
    assert library[0]["title"] == COURSES[1]["title"]
    assert library[0]["progress"] == {"completed": 0, "total": 10}
    assert library[0]["next"]["activityId"] == netty["first"]["id"]

    shell = request(
        client,
        "get",
        f"/api/v1/learning/journeys/{netty['journeyId']}/learner-view",
        headers,
    )
    assert shell["publishedRevisionId"] == netty["revisionId"]
    assert shell["tabs"] == ["learn", "course", "progress", "resources"]
    assert shell["next"]["activityId"] == netty["first"]["id"]

    course = request(
        client,
        "get",
        f"/api/v1/learning/journeys/{netty['journeyId']}/course",
        headers,
    )
    assert course["title"] == COURSES[1]["title"]
    assert [module["title"] for module in course["modules"]] == [
        module["title"] for module in COURSES[1]["modules"]
    ]
    assert all(module["completion"] == {"completed": 0, "total": 2} for module in course["modules"])

    progress = request(
        client,
        "get",
        f"/api/v1/learning/journeys/{netty['journeyId']}/progress-view",
        headers,
    )
    assert progress["attempts"] == []
    assert all(outcome["evidenceStatus"] == "not_assessed_yet" for outcome in progress["outcomes"])

    resources = request(
        client,
        "get",
        f"/api/v1/learning/journeys/{netty['journeyId']}/resources",
        headers,
    )
    assert len(resources["sources"]) == 1
    assert resources["sources"][0]["locator"] == f"reference://{COURSES[1]['slug']}/course-notes.txt"
    assert "content" not in resources["sources"][0]
    assert "contentSha256" not in resources["sources"][0]

    draft_view = client.get(
        f"/api/v1/learning/journeys/{draft['journeyId']}/learner-view",
        headers=headers,
    )
    assert draft_view.status_code == 404, draft_view.text

    other_headers = register_author(client, COURSES[0])
    forbidden = client.get(
        f"/api/v1/learning/journeys/{netty['journeyId']}/learner-view",
        headers=other_headers,
    )
    assert forbidden.status_code == 404, forbidden.text
