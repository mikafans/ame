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
        assert journey.json()["id"] == journey_id
        assert journey.json()["goal"]["rawIntent"] == prompt


if __name__ == "__main__":
    main()
