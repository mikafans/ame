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


def test_agent_first_learning_loop_happy_evil_and_edge_paths(client):
    prompt = "I want to understand stream processing and deploy a Flink operator"

    preview = client.post("/public/v1/onboarding/preview", json={"prompt": prompt})
    assert preview.status_code == 200, preview.text
    assert preview.json()["objectives"]
    assert preview.json()["firstActivity"]["title"]

    empty_preview = client.post("/public/v1/onboarding/preview", json={"prompt": " "})
    assert empty_preview.status_code == 422

    started, headers = _start_learner(client, prompt)
    journey_id = started["journeyId"]

    journey = client.get(f"/api/v1/learning/journeys/{journey_id}", headers=headers)
    assert journey.status_code == 200, journey.text
    journey_body = journey.json()
    objective_id = journey_body["objectives"][0]["id"]
    activity = journey_body["activities"][0]
    activity_id = activity["id"]

    journeys = client.get("/api/v1/learning/journeys", headers=headers)
    assert journeys.status_code == 200, journeys.text
    assert any(item["id"] == journey_id for item in journeys.json())
    session_response = client.post(
        f"/api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
        headers=headers,
    )
    assert session_response.status_code == 201, session_response.text
    session_id = session_response.json()["id"]

    question_response = client.post(
        "/api/v1/questions",
        headers=headers,
        json={
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
            "sourceReferences": ["https://nightlies.apache.org/flink/"],
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
    assert client.get(
        f"/api/v1/assessments/{assessment['id']}", headers=headers
    ).status_code == 200

    attempt_response = client.post(
        f"/api/v1/assessments/{assessment['id']}/attempts",
        headers=headers,
        json={"learningSessionId": session_id},
    )
    assert attempt_response.status_code == 200, attempt_response.text
    attempt = attempt_response.json()
    item_id = assessment["items"][0]["id"]
    version_id = assessment["items"][0]["questionVersionId"]

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

    finished = client.post(
        f"/api/v1/attempts/{attempt['id']}/finish", headers=headers
    )
    assert finished.status_code == 200, finished.text
    assert finished.json()["score"] == 1
    assert finished.json()["items"][0]["evaluationStatus"] == "correct"
    assert finished.json()["items"][0]["awardedPoints"] == 1
    persisted_attempt = client.get(
        f"/api/v1/attempts/{attempt['id']}", headers=headers
    )
    assert persisted_attempt.status_code == 200, persisted_attempt.text
    assert persisted_attempt.json()["score"] == 1
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

    evidence_response = client.post(
        "/api/v1/progress/evidence",
        headers=headers,
        json={
            "journeyId": journey_id,
            "objectiveId": objective_id,
            "activityId": activity_id,
            "attemptId": attempt["id"],
            "value": 1,
            "derivationVersion": 1,
        },
    )
    assert evidence_response.status_code == 200, evidence_response.text
    evidence_id = evidence_response.json()["id"]

    snapshot = client.get(
        f"/api/v1/progress/{journey_id}/objectives/{objective_id}", headers=headers
    )
    assert snapshot.status_code == 200, snapshot.text
    assert snapshot.json()["evidenceCount"] == 1

    recommendation = client.post(
        f"/api/v1/progress/{journey_id}/recommendation",
        headers=headers,
        json={
            "objectives": [
                {"objectiveId": objective_id, "activityId": activity_id}
            ]
        },
    )
    assert recommendation.status_code == 200, recommendation.text

    streak_body = {
        "journeyId": journey_id,
        "activityId": activity_id,
        "qualifyingEventKey": f"attempt:{attempt['id']}",
        "learnerTimezone": "Asia/Tokyo",
        "qualifyingDay": "2026-07-19",
    }
    streak = client.post(
        "/api/v1/progress/streaks", headers=headers, json=streak_body
    )
    assert streak.status_code == 200, streak.text
    duplicate_streak = client.post(
        "/api/v1/progress/streaks", headers=headers, json=streak_body
    )
    assert duplicate_streak.status_code == 200, duplicate_streak.text
    assert duplicate_streak.json()["id"] == streak.json()["id"]
    streaks = client.get(
        f"/api/v1/progress/{journey_id}/streaks", headers=headers
    )
    assert streaks.status_code == 200, streaks.text
    assert len(streaks.json()) == 1

    deep_dive_response = client.post(
        "/api/v1/deep-dives",
        headers=headers,
        json={
            "journeyId": journey_id,
            "activityId": activity_id,
            "objectiveId": objective_id,
            "triggeringEvidenceId": evidence_id,
            "title": "Flink operator scheduling",
            "body": "The JobManager coordinates the deployment lifecycle.",
            "example": "Inspect the JobManager and TaskManager roles.",
            "caveats": ["Deployment behavior depends on the configured operator version."],
            "sourceReferences": ["https://nightlies.apache.org/flink/"],
            "applicationTask": "Explain the scheduling path in your own words.",
            "reviewStatus": "approved",
        },
    )
    assert deep_dive_response.status_code == 200, deep_dive_response.text
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

    finished_session = client.post(
        f"/api/v1/learning/sessions/{session_id}/finish",
        headers=headers,
        json={"completed": True, "responses": [{"id": "q1", "value": "right"}]},
    )
    assert finished_session.status_code == 200, finished_session.text
    timeline = client.get(
        f"/api/v1/progress/{journey_id}/timeline", headers=headers
    )
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
    assert next(
        activity
        for activity in after_abandon.json()["activities"]
        if activity["id"] == next_activity["id"]
    )["status"] == "ready"

    _, other_headers = _start_learner(client, "I want to learn a different subject")
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

    other, other_headers = _start_learner(client, "I want to understand another provider")
    assert other["userId"] != started["userId"]
    cross_owner = client.get(
        f"/api/v1/generation-runs/{run_id}", headers=other_headers
    )
    assert cross_owner.status_code == 404, cross_owner.text
