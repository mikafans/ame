"""Black-box scope coverage for delegated course-author capabilities."""

import uuid


def require_ok(response):
    assert response.is_success, response.text
    return response.json()


def test_delegated_author_can_finish_draft_content_but_cannot_read_learner_profile(
    client,
):
    run_id = str(uuid.uuid4())
    owner = require_ok(
        client.post(
            "/public/v1/auth/register",
            json={
                "email": f"delegated-scope-{run_id}@example.test",
                "name": "Delegated Scope Learner",
                "password": "delegated-scope-2026",
            },
        )
    )
    owner_headers = {"Authorization": f"Bearer {owner['token']}"}
    delegation = require_ok(
        client.post(
            "/api/v1/agent-delegations",
            headers=owner_headers,
            json={"goal": "Learn safe course authoring", "expiresInMinutes": 60},
        )
    )
    authorization = next(
        line for line in delegation["handoff"].splitlines() if line.startswith("Authorization: ")
    )
    author_headers = {"Authorization": authorization.removeprefix("Authorization: ")}

    forbidden = client.get("/api/v1/me", headers=author_headers)
    assert forbidden.status_code == 403, forbidden.text

    source_text = "A reviewed source supports the lesson and its rubric."
    snapshot = require_ok(
        client.post(
            "/api/v1/sources/imports",
            headers=author_headers,
            json={
                "kind": "document",
                "locator": f"reference://delegated-scope/{run_id}.txt",
                "mediaType": "text/plain",
                "content": source_text,
                "retryKey": f"delegated-source-{run_id}",
            },
        )
    )
    citation = require_ok(
        client.post(
            "/api/v1/citations",
            headers=author_headers,
            json={
                "snapshotId": snapshot["id"],
                "startByte": 0,
                "endByte": len(source_text.encode()),
                "quote": source_text,
                "extractionMethod": "exact_quote",
                "groundingStatus": "supported",
                "groundingNote": "Scope test source.",
                "licenseStatus": "allowed",
                "licenseName": "Scope test material",
            },
        )
    )
    course = require_ok(
        client.post(
            "/api/v1/learning/courses",
            headers=author_headers,
            json={
                "rawIntent": "Learn scoped authoring.",
                "idempotencyKey": f"delegated-course-{run_id}",
                "brief": {
                    "title": "Scoped authoring",
                    "audience": "Course authors",
                    "estimatedMinutes": 30,
                    "prerequisites": [],
                    "outcomes": ["Explain scoped authoring."],
                    "modules": ["Scope"],
                },
                "sourceReferences": [citation["id"]],
            },
        )
    )
    journey_id = course["journeyId"]
    revision_id = course["revision"]["id"]
    objective = require_ok(
        client.post(
            f"/api/v1/learning/journeys/{journey_id}/objectives",
            headers=author_headers,
            json={
                "revisionId": revision_id,
                "verb": "Explain",
                "statement": "Explain scoped authoring.",
                "successCriteria": "Names the capability boundary.",
            },
        )
    )
    chapter = require_ok(
        client.post(
            f"/api/v1/learning/journeys/{journey_id}/chapters",
            headers=author_headers,
            json={
                "revisionId": revision_id,
                "title": "Scope",
                "summary": "Author safely.",
            },
        )
    )
    explanation = require_ok(
        client.post(
            f"/api/v1/learning/journeys/{journey_id}/activities",
            headers=author_headers,
            json={
                "revisionId": revision_id,
                "chapterId": chapter["id"],
                "kind": "explanation",
                "title": "Capability boundary",
                "payload": {},
                "objectiveIds": [objective["id"]],
                "status": "ready",
            },
        )
    )
    application = require_ok(
        client.post(
            f"/api/v1/learning/journeys/{journey_id}/activities",
            headers=author_headers,
            json={
                "revisionId": revision_id,
                "chapterId": chapter["id"],
                "kind": "application",
                "title": "Apply the boundary",
                "payload": {},
                "objectiveIds": [objective["id"]],
                "status": "proposed",
            },
        )
    )
    run = require_ok(
        client.post(
            "/api/v1/generation-runs",
            headers=author_headers,
            json={
                "operation": "learning.activity.content.compose",
                "provider": "delegated-scope-test",
                "retryKey": f"delegated-content-{run_id}",
                "contentVersion": 1,
            },
        )
    )
    for status in ("running", "published"):
        require_ok(
            client.patch(
                f"/api/v1/generation-runs/{run['id']}",
                headers=author_headers,
                json={"status": status},
            )
        )
    content = require_ok(
        client.patch(
            f"/api/v1/learning/activities/{explanation['id']}/content",
            headers=author_headers,
            json={
                "revisionId": revision_id,
                "generationRunId": run["id"],
                "content": {
                    "type": "explanation",
                    "heading": "Capability boundary",
                    "body": "The delegated token is limited to course authoring.",
                    "key_points": ["Scope is enforced by the API."],
                },
                "sourceReferences": [citation["id"]],
                "reviewStatus": "approved",
            },
        )
    )
    assert content["id"] == explanation["id"]
    rubric_run = require_ok(
        client.post(
            "/api/v1/generation-runs",
            headers=author_headers,
            json={
                "operation": "learning.activity.rubric.compose",
                "provider": "delegated-scope-test",
                "retryKey": f"delegated-rubric-{run_id}",
                "contentVersion": 1,
            },
        )
    )
    for status in ("running", "published"):
        require_ok(
            client.patch(
                f"/api/v1/generation-runs/{rubric_run['id']}",
                headers=author_headers,
                json={"status": status},
            )
        )
    rubric = require_ok(
        client.patch(
            f"/api/v1/learning/activities/{application['id']}/rubric",
            headers=author_headers,
            json={
                "revisionId": revision_id,
                "generationRunId": rubric_run["id"],
                "rubric": {
                    "version": 1,
                    "passingScore": 0.7,
                    "criteria": [
                        {
                            "id": "scope",
                            "objectiveId": objective["id"],
                            "description": "Explains the capability boundary.",
                            "maxPoints": 1,
                            "required": True,
                        }
                    ],
                },
                "sourceReferences": [citation["id"]],
                "reviewStatus": "approved",
            },
        )
    )
    assert rubric["id"] == application["id"]
    reviewed = require_ok(
        client.post(
            f"/api/v1/learning/activities/{explanation['id']}/review",
            headers=author_headers,
            json={"revisionId": revision_id},
        )
    )
    assert reviewed["publicationStatus"] == "review"
