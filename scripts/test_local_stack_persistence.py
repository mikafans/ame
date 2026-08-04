"""Rehearse learner persistence across a non-destructive local-stack restart."""

from __future__ import annotations

import subprocess
import uuid
from pathlib import Path

import httpx

ROOT = Path(__file__).resolve().parents[1]
BASE_URL = "http://localhost:28800"


def run_stack(command: str) -> None:
    subprocess.run(
        ["uv", "run", "scripts/local_stack.py", command],
        cwd=ROOT,
        check=True,
    )


def main() -> None:
    email = f"restart-{uuid.uuid4()}@example.test"
    password = f"restart-{uuid.uuid4()}"
    prompt = "I would like to understand a subject across restarts"

    with httpx.Client(base_url=BASE_URL, timeout=30.0) as client:
        registration = client.post(
            "/public/v1/auth/register",
            json={"email": email, "name": "Restart Learner", "password": password},
        )
        registration.raise_for_status()
        token = registration.json()["token"]
        headers = {"Authorization": f"Bearer {token}"}
        started = client.post(
            "/public/v1/onboarding/start",
            headers=headers,
            json={
                "email": email,
                "displayName": "Restart Learner",
                "prompt": prompt,
                "idempotencyKey": f"restart-{uuid.uuid4()}",
            },
        )
        started.raise_for_status()
        journey_id = started.json()["journeyId"]

        journey = client.get(f"/api/v1/learning/journeys/{journey_id}", headers=headers)
        journey.raise_for_status()
        journey_body = journey.json()
        assert journey_body["goal"]["rawIntent"] == prompt
        recommendation = journey_body["recommendation"]
        assert recommendation is not None
        candidates = [
            {"objectiveId": objective_id, "activityId": activity["id"]}
            for activity in journey_body["activities"]
            if activity["status"] in {"ready", "in_progress"}
            for objective_id in activity["objectiveIds"]
        ]
        recommendation_read = client.post(
            f"/api/v1/progress/{journey_id}/recommendation",
            headers=headers,
            json={"objectives": candidates},
        )
        recommendation_read.raise_for_status()
        recommendation_body = recommendation_read.json()
        assert recommendation_body["activityId"] == recommendation["activityId"]
        assert recommendation_body["objectiveId"] == recommendation["objectiveId"]
        assert recommendation_body["evidenceIds"] == recommendation["evidenceIds"]

        session_start = client.post(
            f"/api/v1/learning/journeys/{journey_id}/activities/{recommendation['activityId']}/start",
            headers=headers,
        )
        session_start.raise_for_status()
        session_id = session_start.json()["id"]
        session_before = client.get(f"/api/v1/learning/sessions/{session_id}", headers=headers)
        session_before.raise_for_status()
        assert session_before.json()["status"] == "in_progress"

    run_stack("down")
    run_stack("up")

    with httpx.Client(base_url=BASE_URL, timeout=30.0) as client:
        login = client.post(
            "/public/v1/auth/login",
            json={"email": email, "password": password},
        )
        login.raise_for_status()
        token = login.json()["token"]
        journey = client.get(
            f"/api/v1/learning/journeys/{journey_id}",
            headers={"Authorization": f"Bearer {token}"},
        )
        journey.raise_for_status()
        journey_body = journey.json()
        assert journey_body["id"] == journey_id
        assert journey_body["goal"]["rawIntent"] == prompt
        recommendation = journey_body["recommendation"]
        assert recommendation is not None
        candidates = [
            {"objectiveId": objective_id, "activityId": activity["id"]}
            for activity in journey_body["activities"]
            if activity["status"] in {"ready", "in_progress"}
            for objective_id in activity["objectiveIds"]
        ]
        recommendation_read = client.post(
            f"/api/v1/progress/{journey_id}/recommendation",
            headers={"Authorization": f"Bearer {token}"},
            json={"objectives": candidates},
        )
        recommendation_read.raise_for_status()
        recommendation_body = recommendation_read.json()
        assert recommendation_body["activityId"] == recommendation["activityId"]
        assert recommendation_body["objectiveId"] == recommendation["objectiveId"]
        assert recommendation_body["evidenceIds"] == recommendation["evidenceIds"]

        resumed_session = client.get(
            f"/api/v1/learning/sessions/{session_id}",
            headers={"Authorization": f"Bearer {token}"},
        )
        resumed_session.raise_for_status()
        assert resumed_session.json()["id"] == session_id
        assert resumed_session.json()["journeyId"] == journey_id
        assert resumed_session.json()["activityId"] == recommendation["activityId"]
        assert resumed_session.json()["status"] == "in_progress"


if __name__ == "__main__":
    main()
