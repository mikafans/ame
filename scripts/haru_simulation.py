#!/usr/bin/env python3
"""Create a disposable, browser-visible three-round Haru learning journey.

This is intentionally an API-only learner-origin simulation. Fixture journeys
cannot create progress by contract, and this script never writes to Postgres.
Set HARU_SIM_PASSWORD yourself, then open the printed journey URL and log in
with the printed email and that same password.
"""

import json
import os
import sys
import uuid
from pathlib import Path

import httpx

ROOT = Path(__file__).resolve().parents[1]
BASE_URL = os.getenv("AME_API_URL", "http://localhost:28800")
PASSWORD = os.getenv("HARU_SIM_PASSWORD")


def require_ok(response: httpx.Response) -> dict:
    if not response.is_success:
        raise RuntimeError(
            f"{response.request.method} {response.url.path}: {response.status_code} {response.text}"
        )
    return response.json()


def main() -> None:
    if not PASSWORD or len(PASSWORD) < 8:
        raise SystemExit("Set HARU_SIM_PASSWORD to a value with at least 8 characters.")

    run_id = str(uuid.uuid4())
    email = f"haru-simulation-{run_id}@example.test"
    with httpx.Client(base_url=BASE_URL, timeout=30.0) as client:
        registered = require_ok(
            client.post(
                "/public/v1/auth/register",
                json={"email": email, "name": "Haru Simulation", "password": PASSWORD},
            )
        )
        headers = {"Authorization": f"Bearer {registered['token']}"}
        started = require_ok(
            client.post(
                "/public/v1/onboarding/start",
                headers=headers,
                json={
                    "email": email,
                    "displayName": "Haru Simulation",
                    "prompt": "I want to understand distributed systems and design a small event-processing service",
                    "idempotencyKey": f"haru-{run_id}",
                },
            )
        )
        journey_id = started["journeyId"]

        completed = []
        for round_number in range(1, 4):
            journey = require_ok(
                client.get(f"/api/v1/learning/journeys/{journey_id}", headers=headers)
            )
            activity = next(item for item in journey["activities"] if item["status"] == "ready")
            session = require_ok(
                client.post(
                    f"/api/v1/learning/journeys/{journey_id}/activities/{activity['id']}/start",
                    headers=headers,
                )
            )
            responses = []
            questions = activity.get("payload", {}).get("content", {}).get("questions", [])
            for question in questions:
                responses.append({"id": question["id"], "value": "haru-simulation"})
            require_ok(
                client.post(
                    f"/api/v1/learning/sessions/{session['id']}/finish",
                    headers=headers,
                    json={"completed": True, "responses": responses},
                )
            )
            completed.append(
                {"round": round_number, "title": activity["title"], "activityId": activity["id"]}
            )

    handoff = {
        "kind": "local-disposable-haru-simulation",
        "runId": run_id,
        "email": email,
        "journeyId": journey_id,
        "journeyUrl": f"{BASE_URL}/learning/journeys/{journey_id}",
        "completedRounds": completed,
    }
    handoff_dir = ROOT / ".tmp"
    handoff_dir.mkdir(exist_ok=True)
    handoff_path = handoff_dir / f"haru-simulation-{run_id}.json"
    handoff_path.write_text(json.dumps(handoff, indent=2) + "\n")
    print(f"Haru simulation ready: {handoff['journeyUrl']}")
    print(f"Login email: {email}")
    print("Use the HARU_SIM_PASSWORD value you supplied; it is never printed or saved.")
    print(f"Non-secret handoff: {handoff_path}")


if __name__ == "__main__":
    try:
        main()
    except (httpx.HTTPError, RuntimeError) as error:
        print(f"Haru simulation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
