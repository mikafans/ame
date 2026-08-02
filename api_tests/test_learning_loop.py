from datetime import datetime, timedelta
from zoneinfo import ZoneInfo
import uuid


def _start_learner(client, prompt):
    email = f"loop-{uuid.uuid4()}@example.test"
    response = client.post(
        "/public/v1/onboarding/start",
        headers={"Cookie": ""},
        json={
            "email": email,
            "displayName": "Learning Loop",
            "prompt": prompt,
            "idempotencyKey": f"loop-{uuid.uuid4()}",
        },
    )
    assert response.status_code == 201, response.text
    body = response.json()
    return body, {"Authorization": f"Bearer {body['token']}"}


def _published_generation_run(client, headers, operation):
    response = client.post(
        "/api/v1/generation-runs",
        headers=headers,
        json={
            "operation": operation,
            "provider": "test-provider",
            "retryKey": f"{operation}-{uuid.uuid4()}",
            "contentVersion": 1,
        },
    )
    assert response.status_code == 201, response.text
    run_id = response.json()["id"]
    running = client.patch(
        f"/api/v1/generation-runs/{run_id}",
        headers=headers,
        json={"status": "running"},
    )
    assert running.status_code == 200, running.text
    published = client.patch(
        f"/api/v1/generation-runs/{run_id}",
        headers=headers,
        json={"status": "published"},
    )
    assert published.status_code == 200, published.text
    return run_id


def _certified_reference(client, headers):
    content = "A supported source statement."
    source = client.post(
        "/api/v1/sources/imports",
        headers=headers,
        json={
            "kind": "document",
            "locator": "test://grounding/source.txt",
            "mediaType": "text/plain",
            "content": content,
            "retryKey": f"citation-source-{uuid.uuid4()}",
        },
    )
    assert source.status_code == 200, source.text
    citation = client.post(
        "/api/v1/citations",
        headers=headers,
        json={
            "snapshotId": source.json()["id"],
            "startByte": 0,
            "endByte": len(content),
            "quote": content,
            "extractionMethod": "exact_quote",
            "groundingStatus": "supported",
            "groundingNote": "The exact immutable source text supports this test content.",
            "licenseStatus": "allowed",
            "licenseName": "CC BY 4.0",
            "licenseUrl": "https://creativecommons.org/licenses/by/4.0/",
        },
    )
    assert citation.status_code == 200, citation.text
    return [citation.json()["id"]]


def _recommendation_candidates(journey_body):
    return [
        {"objectiveId": objective_id, "activityId": activity["id"]}
        for activity in journey_body["activities"]
        if activity["status"] in {"ready", "in_progress"}
        for objective_id in activity["objectiveIds"]
    ]


def test_agent_authored_activity_stays_private_until_review_and_publish(client):
    started, headers = _start_learner(client, "I want to learn a useful subject")
    journey_id = started["journeyId"]
    journey = client.get(f"/api/v1/learning/journeys/{journey_id}", headers=headers)
    assert journey.status_code == 200, journey.text
    objective_id = journey.json()["objectives"][0]["id"]

    created = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities",
        headers=headers,
        json={
            "chapterId": None,
            "kind": "practice",
            "title": "Draft-only agent activity",
            "payload": {"prompt": "A draft activity must not reach the learner."},
            "objectiveIds": [objective_id],
            "status": "ready",
        },
    )
    assert created.status_code == 200, created.text
    activity = created.json()
    assert activity["publicationStatus"] == "draft"

    blocked = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities/{activity['id']}/start",
        headers=headers,
        json={"questionPlan": {}},
    )
    assert blocked.status_code == 422, blocked.text

    invalid_publish = client.post(
        f"/api/v1/learning/activities/{activity['id']}/publish", headers=headers
    )
    assert invalid_publish.status_code == 409, invalid_publish.text

    review = client.post(
        f"/api/v1/learning/activities/{activity['id']}/review", headers=headers
    )
    assert review.status_code == 200, review.text
    assert review.json()["publicationStatus"] == "review"

    blocked_course_publish = client.post(
        f"/api/v1/learning/activities/{activity['id']}/publish", headers=headers
    )
    assert blocked_course_publish.status_code == 409, blocked_course_publish.text
    assert blocked_course_publish.json()["error"]["code"] == "course_publication_required"

    still_blocked = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities/{activity['id']}/start",
        headers=headers,
        json={"questionPlan": {}},
    )
    assert still_blocked.status_code == 422, still_blocked.text


def test_course_revision_returns_server_side_publish_diagnostics(client):
    started, headers = _start_learner(client, "I want to learn a useful subject")
    references = _certified_reference(client, headers)
    journey_id = started["journeyId"]
    created = client.post(
        f"/api/v1/learning/journeys/{journey_id}/course-revisions",
        headers=headers,
        json={
            "brief": {
                "title": "A source-grounded course",
                "audience": "A learner with a concrete goal",
                "estimatedMinutes": 60,
                "prerequisites": ["None beyond the stated learner level."],
                "outcomes": ["Explain one verifiable course outcome."],
                "modules": ["Foundations", "Practice"],
            },
            "sourceReferences": references,
        },
    )
    assert created.status_code == 201, created.text
    revision = created.json()
    assert revision["status"] == "draft"

    validated = client.post(
        f"/api/v1/learning/journeys/{journey_id}/course-revisions/{revision['id']}/validate",
        headers=headers,
    )
    assert validated.status_code == 200, validated.text
    issue_codes = {issue["code"] for issue in validated.json()["validation"]}
    assert "activity_not_reviewed" in issue_codes
    assert "instruction_missing_formative_check" in issue_codes
    assert "outcome_missing_mastery" in issue_codes

    publish = client.post(
        f"/api/v1/learning/journeys/{journey_id}/course-revisions/{revision['id']}/publish",
        headers=headers,
    )
    assert publish.status_code == 409, publish.text


def test_agent_can_start_an_empty_course_without_generic_template_material(client):
    started, headers = _start_learner(client, "I want to learn a useful subject")
    references = _certified_reference(client, headers)
    created = client.post(
        "/api/v1/learning/courses",
        headers=headers,
        json={
            "rawIntent": "Design a reliable asynchronous TCP service with Netty.",
            "idempotencyKey": f"agent-course-{uuid.uuid4()}",
            "brief": {
                "title": "Netty service design",
                "audience": "A Java maintainer",
                "estimatedMinutes": 90,
                "prerequisites": ["Basic Java concurrency"],
                "outcomes": ["Explain a Netty pipeline design"],
                "modules": ["Event loops"],
            },
            "sourceReferences": references,
        },
    )
    assert created.status_code == 201, created.text
    course = created.json()
    assert course["journeyId"] != started["journeyId"]
    assert course["revision"]["status"] == "draft"

    journey = client.get(
        f"/api/v1/learning/journeys/{course['journeyId']}", headers=headers
    )
    assert journey.status_code == 200, journey.text
    assert journey.json()["activities"] == []
    assert journey.json()["objectives"] == []


def test_agent_can_ground_first_package_activity_and_cannot_forge_it(client):
    started, headers = _start_learner(client, "I want to learn a useful subject")
    references = _certified_reference(client, headers)
    journey_id = started["journeyId"]
    journey = client.get(f"/api/v1/learning/journeys/{journey_id}", headers=headers)
    assert journey.status_code == 200, journey.text
    activities = journey.json()["activities"]
    explanation = next(
        activity
        for activity in activities
        if activity["payload"]["content"]["type"] == "explanation"
    )
    worked_example = next(
        activity
        for activity in activities
        if activity["payload"]["content"]["type"] == "worked_example"
    )
    generation_run_id = _published_generation_run(
        client, headers, "learning.activity.content.compose"
    )
    worked_example_content = {
        "type": "worked_example",
        "heading": "A bounded worked example",
        "prompt": "Apply one idea to one observable situation.",
        "steps": ["Name the situation.", "Apply the idea.", "Inspect the result."],
        "reflection": "What would you try next?",
    }
    explanation_kind_mismatch = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": worked_example_content,
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert explanation_kind_mismatch.status_code == 422, explanation_kind_mismatch.text
    worked_example_kind_mismatch = client.patch(
        f"/api/v1/learning/activities/{worked_example['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": {
                "type": "explanation",
                "heading": "Wrong kind",
                "body": "This belongs to an explanation activity.",
                "key_points": ["It must not be stored here."],
            },
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert worked_example_kind_mismatch.status_code == 422, (
        worked_example_kind_mismatch.text
    )
    non_string_field = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": {
                "type": "explanation",
                "heading": "Invalid field",
                "body": 42,
                "key_points": ["This is not enough."],
            },
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert non_string_field.status_code == 422, non_string_field.text
    empty_list = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": {
                "type": "explanation",
                "heading": "Invalid list",
                "body": "The list is empty.",
                "key_points": [],
            },
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert empty_list.status_code == 422, empty_list.text
    unapproved = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": {
                "type": "explanation",
                "heading": "Unapproved",
                "body": "This must not become learner-visible.",
                "key_points": ["Review is mandatory."],
            },
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "draft",
        },
    )
    assert unapproved.status_code == 422, unapproved.text
    content = {
        "type": "explanation",
        "heading": "A grounded starting model",
        "body": "This explanation was authored from the learner's actual sources.",
        "key_points": ["Start with one observable mechanism."],
    }
    grounded = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": content,
            "sourceReferences": references,
            "reviewStatus": "approved",
        },
    )
    assert grounded.status_code == 200, grounded.text
    assert grounded.json()["payload"]["content"] == content
    assert grounded.json()["payload"]["contentProvenance"] == {
        "generationRunId": generation_run_id,
        "reviewStatus": "approved",
        "sourceReferences": references,
    }
    worked_grounded = client.patch(
        f"/api/v1/learning/activities/{worked_example['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": worked_example_content,
            "sourceReferences": references,
            "reviewStatus": "approved",
        },
    )
    assert worked_grounded.status_code == 200, worked_grounded.text
    assert worked_grounded.json()["payload"]["content"] == worked_example_content
    assert worked_grounded.json()["payload"]["contentProvenance"][
        "sourceReferences"
    ] == references

    invalid_content = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": {"type": "worked_example", "heading": "wrong"},
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert invalid_content.status_code == 422, invalid_content.text

    missing_sources = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": content,
            "sourceReferences": [],
            "reviewStatus": "approved",
        },
    )
    assert missing_sources.status_code == 422, missing_sources.text

    unpublished_run_response = client.post(
        "/api/v1/generation-runs",
        headers=headers,
        json={
            "operation": "learning.activity.content.compose",
            "provider": "test-provider",
            "retryKey": f"unpublished-{uuid.uuid4()}",
            "contentVersion": 1,
        },
    )
    assert unpublished_run_response.status_code == 201
    unpublished = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": unpublished_run_response.json()["id"],
            "content": content,
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert unpublished.status_code == 409, unpublished.text

    wrong_operation_run_id = _published_generation_run(
        client, headers, "question.compose"
    )
    wrong_operation = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": wrong_operation_run_id,
            "content": content,
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert wrong_operation.status_code == 422, wrong_operation.text

    _, other_headers = _start_learner(client, "I want a different subject")
    cross_owner = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=other_headers,
        json={
            "generationRunId": generation_run_id,
            "content": content,
            "sourceReferences": ["https://example.test/subject/intro"],
            "reviewStatus": "approved",
        },
    )
    assert cross_owner.status_code == 404, cross_owner.text

    starter = activities[0]
    starter_session = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities/{starter['id']}/start",
        headers=headers,
    )
    assert starter_session.status_code == 201, starter_session.text
    finish_starter = client.post(
        f"/api/v1/learning/sessions/{starter_session.json()['id']}/finish",
        headers=headers,
        json={"completed": True, "responses": []},
    )
    assert finish_starter.status_code == 200, finish_starter.text
    explanation_session = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities/{explanation['id']}/start",
        headers=headers,
    )
    assert explanation_session.status_code == 201, explanation_session.text
    finish_explanation = client.post(
        f"/api/v1/learning/sessions/{explanation_session.json()['id']}/finish",
        headers=headers,
        json={"completed": True, "responses": []},
    )
    assert finish_explanation.status_code == 200, finish_explanation.text
    completed = client.patch(
        f"/api/v1/learning/activities/{explanation['id']}/content",
        headers=headers,
        json={
            "generationRunId": generation_run_id,
            "content": content,
            "sourceReferences": references,
            "reviewStatus": "approved",
        },
    )
    assert completed.status_code == 409, completed.text


def test_agent_first_learning_loop_happy_evil_and_edge_paths(client):
    prompt = "I want to understand stream processing and deploy a Flink operator"

    preview = client.post("/public/v1/onboarding/preview", json={"prompt": prompt})
    assert preview.status_code == 200, preview.text
    assert preview.json()["objectives"]
    assert preview.json()["firstActivity"]["title"]

    empty_preview = client.post("/public/v1/onboarding/preview", json={"prompt": " "})
    assert empty_preview.status_code == 422

    started, headers = _start_learner(client, prompt)
    references = _certified_reference(client, headers)
    journey_id = started["journeyId"]

    journey = client.get(f"/api/v1/learning/journeys/{journey_id}", headers=headers)
    assert journey.status_code == 200, journey.text
    journey_body = journey.json()
    objective_id = journey_body["objectives"][0]["id"]
    activity = journey_body["activities"][0]
    activity_id = activity["id"]
    initial_recommendation = journey_body["recommendation"]
    initial_progress_recommendation = client.post(
        f"/api/v1/progress/{journey_id}/recommendation",
        headers=headers,
        json={"objectives": _recommendation_candidates(journey_body)},
    )
    assert initial_progress_recommendation.status_code == 200, (
        initial_progress_recommendation.text
    )
    assert (
        initial_recommendation["objectiveId"]
        == initial_progress_recommendation.json()["objectiveId"]
    )
    assert (
        initial_recommendation["activityId"]
        == initial_progress_recommendation.json()["activityId"]
    )
    assert (
        initial_recommendation["evidenceIds"]
        == initial_progress_recommendation.json()["evidenceIds"]
    )

    journeys = client.get("/api/v1/learning/journeys", headers=headers)
    assert journeys.status_code == 200, journeys.text
    assert any(item["id"] == journey_id for item in journeys.json())
    session_response = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
        headers=headers,
    )
    assert session_response.status_code == 201, session_response.text
    session_id = session_response.json()["id"]
    question_generation_run_id = _published_generation_run(
        client, headers, "question.compose"
    )

    question_response = client.post(
        "/api/v1/questions",
        headers=headers,
        json={
            "generationRunId": question_generation_run_id,
            "kind": "multiple_choice",
            "prompt": "Which Flink component schedules a deployed job?",
            "options": [
                {"id": "wrong", "text": "The client only", "is_correct": False},
                {"id": "right", "text": "The JobManager", "is_correct": True},
            ],
            "points": 1,
            "reviewStatus": "approved",
            "explanation": "The JobManager coordinates scheduling and execution.",
            "difficulty": "introductory",
            "sourceReferences": references,
        },
    )
    assert question_response.status_code == 200, question_response.text
    question = question_response.json()

    draft_activity = journey_body["activities"][1]
    draft_assessment_response = client.post(
        "/api/v1/assessments",
        headers=headers,
        json={
            "activityId": draft_activity["id"],
            "mode": "practice",
            "status": "draft",
            "items": [
                {
                    "objectiveId": objective_id,
                    "questionId": question["questionId"],
                    "questionVersion": question["version"],
                    "orderIndex": 0,
                    "points": 1,
                }
            ],
        },
    )
    assert draft_assessment_response.status_code == 200, draft_assessment_response.text
    assert (
        client.get(
            "/api/v1/assessments",
            params={"activityId": draft_activity["id"]},
            headers=headers,
        ).status_code
        == 404
    )

    assessment_response = client.post(
        "/api/v1/assessments",
        headers=headers,
        json={
            "activityId": activity_id,
            "mode": "practice",
            "status": "published",
            "items": [
                {
                    "objectiveId": objective_id,
                    "questionId": question["questionId"],
                    "questionVersion": question["version"],
                    "orderIndex": 0,
                    "points": 1,
                }
            ],
        },
    )
    assert assessment_response.status_code == 200, assessment_response.text
    assessment = assessment_response.json()
    discovered_assessment = client.get(
        "/api/v1/assessments",
        params={"activityId": activity_id},
        headers=headers,
    )
    assert discovered_assessment.status_code == 200, discovered_assessment.text
    assert discovered_assessment.json()["id"] == assessment["id"]
    projected_question = assessment["items"][0]["question"]
    assert projected_question["prompt"]
    assert projected_question["difficulty"] == "introductory"
    assert projected_question["options"]
    assert "is_correct" not in projected_question["options"][0]
    assert (
        client.get(
            f"/api/v1/assessments/{assessment['id']}", headers=headers
        ).status_code
        == 200
    )

    attempt_response = client.post(
        f"/api/v1/assessments/{assessment['id']}/attempts",
        headers=headers,
        json={"learningSessionId": session_id},
    )
    assert attempt_response.status_code == 200, attempt_response.text
    attempt = attempt_response.json()
    assert attempt["assessmentMode"] == "practice"
    assert attempt["reviewStatus"] == "not_required"
    item_id = assessment["items"][0]["id"]
    version_id = assessment["items"][0]["questionVersionId"]

    unfinished_evidence = client.post(
        "/api/v1/progress/evidence",
        headers=headers,
        json={
            "journeyId": journey_id,
            "objectiveId": objective_id,
            "activityId": activity_id,
            "attemptId": attempt["id"],
            "contentVersion": 1,
            "value": 1,
            "derivationVersion": 1,
        },
    )
    assert unfinished_evidence.status_code == 404, unfinished_evidence.text

    stale_answer = client.post(
        f"/api/v1/attempts/{attempt['id']}/answers",
        headers=headers,
        json={
            "assessmentItemId": item_id,
            "questionVersionId": str(uuid.uuid4()),
            "response": {"option_id": "right"},
        },
    )
    assert stale_answer.status_code == 422, stale_answer.text

    answer = client.post(
        f"/api/v1/attempts/{attempt['id']}/answers",
        headers=headers,
        json={
            "assessmentItemId": item_id,
            "questionVersionId": version_id,
            "response": {"option_id": "right"},
        },
    )
    assert answer.status_code == 200, answer.text

    finished = client.post(f"/api/v1/attempts/{attempt['id']}/finish", headers=headers)
    assert finished.status_code == 200, finished.text
    assert finished.json()["score"] == 1
    assert finished.json()["assessmentMode"] == "practice"
    assert finished.json()["reviewStatus"] == "complete"
    assert finished.json()["items"][0]["evaluationStatus"] == "correct"
    assert finished.json()["items"][0]["awardedPoints"] == 1
    persisted_attempt = client.get(f"/api/v1/attempts/{attempt['id']}", headers=headers)
    assert persisted_attempt.status_code == 200, persisted_attempt.text
    assert persisted_attempt.json()["score"] == 1
    assert persisted_attempt.json()["assessmentMode"] == "practice"
    assert persisted_attempt.json()["reviewStatus"] == "complete"
    assert persisted_attempt.json()["items"][0]["evaluationStatus"] == "correct"
    attempt_history = client.get(
        f"/api/v1/learning/journeys/{journey_id}/attempts", headers=headers
    )
    assert attempt_history.status_code == 200, attempt_history.text
    assert [item["id"] for item in attempt_history.json()] == [attempt["id"]]
    repeated_finish = client.post(
        f"/api/v1/attempts/{attempt['id']}/finish", headers=headers
    )
    assert repeated_finish.status_code == 409, repeated_finish.text

    forged_evidence = client.post(
        "/api/v1/progress/evidence",
        headers=headers,
        json={
            "journeyId": journey_id,
            "objectiveId": objective_id,
            "activityId": activity_id,
            "contentVersion": 1,
            "value": 1,
            "derivationVersion": 1,
        },
    )
    assert forged_evidence.status_code == 422, forged_evidence.text

    evidence_response = client.post(
        "/api/v1/progress/evidence",
        headers=headers,
        json={
            "journeyId": journey_id,
            "objectiveId": objective_id,
            "activityId": activity_id,
            "attemptId": attempt["id"],
            "contentVersion": 1,
            "value": 1,
            "derivationVersion": 1,
        },
    )
    assert evidence_response.status_code == 200, evidence_response.text
    evidence_id = evidence_response.json()["id"]

    _, other_headers = _start_learner(
        client, "I want to understand a different subject"
    )
    cross_owner_evidence = client.post(
        "/api/v1/progress/evidence",
        headers=other_headers,
        json={
            "journeyId": journey_id,
            "objectiveId": objective_id,
            "activityId": activity_id,
            "attemptId": attempt["id"],
            "contentVersion": 1,
            "value": 0.25,
            "derivationVersion": 1,
        },
    )
    assert cross_owner_evidence.status_code == 404, cross_owner_evidence.text
    cross_owner_recommendation = client.post(
        f"/api/v1/progress/{journey_id}/recommendation",
        headers=other_headers,
        json={"objectives": [{"objectiveId": objective_id, "activityId": activity_id}]},
    )
    assert cross_owner_recommendation.status_code == 404, (
        cross_owner_recommendation.text
    )

    snapshot = client.get(
        f"/api/v1/progress/{journey_id}/objectives/{objective_id}", headers=headers
    )
    assert snapshot.status_code == 200, snapshot.text
    assert snapshot.json()["evidenceCount"] == 1
    snapshot_again = client.get(
        f"/api/v1/progress/{journey_id}/objectives/{objective_id}", headers=headers
    )
    assert snapshot_again.status_code == 200, snapshot_again.text
    assert snapshot_again.json()["calculatedAt"] == snapshot.json()["calculatedAt"]

    recommendation = client.post(
        f"/api/v1/progress/{journey_id}/recommendation",
        headers=headers,
        json={"objectives": [{"objectiveId": objective_id, "activityId": activity_id}]},
    )
    assert recommendation.status_code == 200, recommendation.text

    learner_day = datetime.now(ZoneInfo("Asia/Tokyo")).date()
    streak_body = {
        "journeyId": journey_id,
        "activityId": activity_id,
        "qualifyingEventKey": f"attempt:{attempt['id']}",
        "learnerTimezone": "Asia/Tokyo",
        "qualifyingDay": learner_day.isoformat(),
    }
    unverified_streak_body = {**streak_body, "qualifyingEventKey": "agent-asserted"}
    unverified_streak = client.post(
        "/api/v1/progress/streaks", headers=headers, json=unverified_streak_body
    )
    assert unverified_streak.status_code == 422, unverified_streak.text
    mismatched_streak = client.post(
        "/api/v1/progress/streaks",
        headers=headers,
        json={
            **streak_body,
            "qualifyingDay": (learner_day - timedelta(days=1)).isoformat(),
        },
    )
    assert mismatched_streak.status_code == 422, mismatched_streak.text
    invalid_timezone_streak = client.post(
        "/api/v1/progress/streaks",
        headers=headers,
        json={**streak_body, "learnerTimezone": "Not/A-Timezone"},
    )
    assert invalid_timezone_streak.status_code == 422, invalid_timezone_streak.text
    cross_owner_streak = client.post(
        "/api/v1/progress/streaks", headers=other_headers, json=streak_body
    )
    assert cross_owner_streak.status_code == 404, cross_owner_streak.text
    streak = client.post("/api/v1/progress/streaks", headers=headers, json=streak_body)
    assert streak.status_code == 200, streak.text
    duplicate_streak = client.post(
        "/api/v1/progress/streaks", headers=headers, json=streak_body
    )
    assert duplicate_streak.status_code == 200, duplicate_streak.text
    assert duplicate_streak.json()["id"] == streak.json()["id"]
    streaks = client.get(f"/api/v1/progress/{journey_id}/streaks", headers=headers)
    assert streaks.status_code == 200, streaks.text
    assert len(streaks.json()) == 1

    deep_dive_generation_run_id = _published_generation_run(
        client, headers, "deep_dive.create"
    )
    deep_dive_response = client.post(
        "/api/v1/deep-dives",
        headers=headers,
        json={
            "generationRunId": deep_dive_generation_run_id,
            "journeyId": journey_id,
            "activityId": activity_id,
            "objectiveId": objective_id,
            "triggeringEvidenceId": evidence_id,
            "title": "Flink operator scheduling",
            "body": "The JobManager coordinates the deployment lifecycle.",
            "example": "Inspect the JobManager and TaskManager roles.",
            "caveats": [
                "Deployment behavior depends on the configured operator version."
            ],
            "sourceReferences": references,
            "applicationTask": "Explain the scheduling path in your own words.",
            "reviewStatus": "approved",
        },
    )
    assert deep_dive_response.status_code == 200, deep_dive_response.text
    assert deep_dive_response.json()["reviewStatus"] == "approved"
    deep_dive_id = deep_dive_response.json()["id"]
    discovered_deep_dive = client.get(
        "/api/v1/deep-dives",
        params={"activityId": activity_id},
        headers=headers,
    )
    assert discovered_deep_dive.status_code == 200, discovered_deep_dive.text
    assert discovered_deep_dive.json()["id"] == deep_dive_id
    deep_dive = client.get(f"/api/v1/deep-dives/{deep_dive_id}", headers=headers)
    assert deep_dive.status_code == 200, deep_dive.text
    assert deep_dive.json()["reviewStatus"] == "approved"

    forged_deep_dive = client.post(
        "/api/v1/deep-dives",
        headers=headers,
        json={
            "journeyId": journey_id,
            "activityId": activity_id,
            "objectiveId": objective_id,
            "triggeringEvidenceId": str(uuid.uuid4()),
            "generationRunId": deep_dive_generation_run_id,
            "title": "Forged evidence",
            "body": "This must not be stored.",
            "example": "No example.",
            "caveats": ["This is a negative-path test."],
            "sourceReferences": ["https://example.test/forged"],
            "applicationTask": "Reject this content.",
        },
    )
    assert forged_deep_dive.status_code == 404, forged_deep_dive.text

    finished_session = client.post(
        f"/api/v1/learning/sessions/{session_id}/finish",
        headers=headers,
        json={"completed": True, "responses": [{"id": "q1", "value": "right"}]},
    )
    assert finished_session.status_code == 200, finished_session.text
    timeline = client.get(f"/api/v1/progress/{journey_id}/timeline", headers=headers)
    assert timeline.status_code == 200, timeline.text
    timeline_body = timeline.json()
    timeline_kinds = [event["kind"] for event in timeline_body]
    assert "activity_started" in timeline_kinds
    assert "activity_completed" in timeline_kinds
    assert "assessment_submitted" in timeline_kinds
    assert "evidence_recorded" in timeline_kinds
    assert "deep_dive_created" in timeline_kinds
    assert "streak_recorded" in timeline_kinds
    assert [event["occurredAt"] for event in timeline_body] == sorted(
        event["occurredAt"] for event in timeline_body
    )

    resumed_journey = client.get(
        f"/api/v1/learning/journeys/{journey_id}", headers=headers
    )
    assert resumed_journey.status_code == 200, resumed_journey.text
    resumed_journey_body = resumed_journey.json()
    resumed_progress_recommendation = client.post(
        f"/api/v1/progress/{journey_id}/recommendation",
        headers=headers,
        json={"objectives": _recommendation_candidates(resumed_journey_body)},
    )
    assert resumed_progress_recommendation.status_code == 200, (
        resumed_progress_recommendation.text
    )
    assert resumed_journey_body["recommendation"] == {
        "activityId": resumed_progress_recommendation.json()["activityId"],
        "objectiveId": resumed_progress_recommendation.json()["objectiveId"],
        "title": resumed_journey_body["recommendation"]["title"],
        "objectiveIds": resumed_journey_body["recommendation"]["objectiveIds"],
        "evidenceIds": resumed_progress_recommendation.json()["evidenceIds"],
        "rationale": resumed_progress_recommendation.json()["reason"],
        "basedOnSessionId": resumed_journey_body["recommendation"]["basedOnSessionId"],
    }
    next_activity = next(
        activity
        for activity in resumed_journey.json()["activities"]
        if activity["status"] == "ready"
    )
    abandoned_session = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities/{next_activity['id']}/start",
        headers=headers,
    )
    assert abandoned_session.status_code == 201, abandoned_session.text
    abandoned = client.post(
        f"/api/v1/learning/sessions/{abandoned_session.json()['id']}/finish",
        headers=headers,
        json={"completed": False, "responses": []},
    )
    assert abandoned.status_code == 200, abandoned.text
    assert abandoned.json()["status"] == "abandoned"
    after_abandon = client.get(
        f"/api/v1/learning/journeys/{journey_id}", headers=headers
    )
    assert after_abandon.status_code == 200, after_abandon.text
    assert (
        next(
            activity
            for activity in after_abandon.json()["activities"]
            if activity["id"] == next_activity["id"]
        )["status"]
        == "ready"
    )

    other_started, other_headers = _start_learner(
        client, "I want to learn a different subject"
    )
    other_journey = client.get(
        f"/api/v1/learning/journeys/{other_started['journeyId']}",
        headers=other_headers,
    )
    assert other_journey.status_code == 200, other_journey.text
    other_journey_body = other_journey.json()
    other_activity = other_journey_body["activities"][0]
    other_deep_dive_generation_run_id = _published_generation_run(
        client, other_headers, "deep_dive.create"
    )
    cross_owner_deep_dive = client.post(
        "/api/v1/deep-dives",
        headers=other_headers,
        json={
            "journeyId": other_started["journeyId"],
            "activityId": other_activity["id"],
            "objectiveId": other_journey_body["objectives"][0]["id"],
            "triggeringEvidenceId": evidence_id,
            "generationRunId": other_deep_dive_generation_run_id,
            "title": "Cross-owner evidence",
            "body": "This must not be stored.",
            "example": "No example.",
            "caveats": ["This is a negative-path test."],
            "sourceReferences": ["https://example.test/cross-owner"],
            "applicationTask": "Reject this content.",
        },
    )
    assert cross_owner_deep_dive.status_code == 404, cross_owner_deep_dive.text
    other_streaks = client.get(
        f"/api/v1/progress/{journey_id}/streaks", headers=other_headers
    )
    assert other_streaks.status_code == 404, other_streaks.text
    other_attempt_history = client.get(
        f"/api/v1/learning/journeys/{journey_id}/attempts", headers=other_headers
    )
    assert other_attempt_history.status_code == 404, other_attempt_history.text
    other_timeline = client.get(
        f"/api/v1/progress/{journey_id}/timeline", headers=other_headers
    )
    assert other_timeline.status_code == 404, other_timeline.text
    forbidden_journey = client.get(
        f"/api/v1/learning/journeys/{journey_id}", headers=other_headers
    )
    assert forbidden_journey.status_code == 404, forbidden_journey.text


def test_generation_runs_are_retryable_failure_safe_and_owner_scoped(client):
    started, headers = _start_learner(client, "I want to understand provider retries")
    retry_key = f"generation-{uuid.uuid4()}"
    payload = {
        "operation": "question.compose",
        "provider": "external-agent",
        "retryKey": retry_key,
        "contentVersion": 1,
    }

    first = client.post("/api/v1/generation-runs", headers=headers, json=payload)
    assert first.status_code == 201, first.text
    first_body = first.json()
    assert first_body["status"] == "requested"

    retry = client.post("/api/v1/generation-runs", headers=headers, json=payload)
    assert retry.status_code == 201, retry.text
    assert retry.json()["id"] == first_body["id"]

    run_id = first_body["id"]
    running = client.patch(
        f"/api/v1/generation-runs/{run_id}",
        headers=headers,
        json={"status": "running"},
    )
    assert running.status_code == 200, running.text

    failed = client.patch(
        f"/api/v1/generation-runs/{run_id}",
        headers=headers,
        json={"status": "failed", "error": {"code": "provider_unavailable"}},
    )
    assert failed.status_code == 200, failed.text
    assert failed.json()["status"] == "failed"

    false_publish = client.patch(
        f"/api/v1/generation-runs/{run_id}",
        headers=headers,
        json={"status": "published"},
    )
    assert false_publish.status_code == 409, false_publish.text

    other, other_headers = _start_learner(
        client, "I want to understand another provider"
    )
    assert other["userId"] != started["userId"]
    cross_owner = client.get(f"/api/v1/generation-runs/{run_id}", headers=other_headers)
    assert cross_owner.status_code == 404, cross_owner.text


def test_generated_content_requires_successful_owner_generation_run(client):
    started, headers = _start_learner(client, "I want a generated practice set")
    question = {
        "kind": "multiple_choice",
        "prompt": "Which component coordinates a deployed job?",
        "options": [
            {"id": "wrong", "text": "The browser", "is_correct": False},
            {"id": "right", "text": "The JobManager", "is_correct": True},
        ],
        "points": 1,
        "reviewStatus": "draft",
        "sourceReferences": ["https://nightlies.apache.org/flink/"],
    }

    missing_provenance = client.post(
        "/api/v1/questions", headers=headers, json=question
    )
    assert missing_provenance.status_code == 422, missing_provenance.text

    failed_run = client.post(
        "/api/v1/generation-runs",
        headers=headers,
        json={
            "operation": "question.compose",
            "provider": "test-provider",
            "retryKey": f"failed-question-{uuid.uuid4()}",
            "contentVersion": 1,
        },
    )
    assert failed_run.status_code == 201, failed_run.text
    failed_run_id = failed_run.json()["id"]
    assert (
        client.patch(
            f"/api/v1/generation-runs/{failed_run_id}",
            headers=headers,
            json={"status": "running"},
        ).status_code
        == 200
    )
    assert (
        client.patch(
            f"/api/v1/generation-runs/{failed_run_id}",
            headers=headers,
            json={"status": "failed", "error": {"code": "provider_unavailable"}},
        ).status_code
        == 200
    )
    failed_content = client.post(
        "/api/v1/questions",
        headers=headers,
        json={**question, "generationRunId": failed_run_id},
    )
    assert failed_content.status_code == 409, failed_content.text

    _, other_headers = _start_learner(client, "I want another generated practice set")
    other_run = client.post(
        "/api/v1/generation-runs",
        headers=other_headers,
        json={
            "operation": "question.compose",
            "provider": "test-provider",
            "retryKey": f"other-question-{uuid.uuid4()}",
            "contentVersion": 1,
        },
    )
    assert other_run.status_code == 201, other_run.text
    cross_owner_content = client.post(
        "/api/v1/questions",
        headers=headers,
        json={**question, "generationRunId": other_run.json()["id"]},
    )
    assert cross_owner_content.status_code == 404, cross_owner_content.text

    deep_dive_missing_provenance = client.post(
        "/api/v1/deep-dives",
        headers=headers,
        json={
            "journeyId": started["journeyId"],
            "activityId": str(uuid.uuid4()),
            "objectiveId": str(uuid.uuid4()),
            "triggeringEvidenceId": str(uuid.uuid4()),
            "title": "Generated explanation",
            "body": "A grounded explanation.",
            "example": "A small example.",
            "caveats": [],
            "sourceReferences": ["https://example.test/source"],
            "applicationTask": "Apply the idea.",
        },
    )
    assert deep_dive_missing_provenance.status_code == 422, (
        deep_dive_missing_provenance.text
    )
