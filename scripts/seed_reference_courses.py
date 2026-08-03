#!/usr/bin/env python3
"""Create delegated Flink and Netty reference courses for one local learner.

This is a local-stack quality fixture, not a database seed. It registers (or
logs in) a learner, creates one scoped `dlg_*` capability per course, and uses
the same public authoring workflow exercised by ``api_tests/test_reference_courses.py``.
Only non-secret learner/course metadata is written to ``.tmp``.
"""

import argparse
import json
import os
import sys
import uuid
from pathlib import Path

import httpx


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from api_tests.test_reference_courses import COURSES, build_reference_course  # noqa: E402


DEFAULT_EMAIL = "haru-reference@example.test"
DEFAULT_NAME = "Haru"
BASE_URL = os.getenv("AME_API_URL", "http://localhost:28800")


def require_ok(response: httpx.Response) -> dict:
    if not response.is_success:
        raise RuntimeError(
            f"{response.request.method} {response.request.url.path}: "
            f"{response.status_code} {response.text}"
        )
    return response.json()


def learner_headers(
    client: httpx.Client, email: str, name: str, password: str
) -> dict[str, str]:
    registered = client.post(
        "/public/v1/auth/register",
        json={"email": email, "name": name, "password": password},
    )
    if registered.status_code == 201:
        payload = registered.json()
    elif registered.status_code == 422:
        payload = require_ok(
            client.post(
                "/public/v1/auth/login", json={"email": email, "password": password}
            )
        )
    else:
        payload = require_ok(registered)
    return {"Authorization": f"Bearer {payload['token']}"}


def delegation_token(
    client: httpx.Client, headers: dict[str, str], goal: str
) -> tuple[dict, dict[str, str]]:
    payload = require_ok(
        client.post(
            "/api/v1/agent-delegations",
            headers=headers,
            json={"goal": goal, "expiresInMinutes": 120},
        )
    )
    authorization = next(
        (
            line
            for line in payload["handoff"].splitlines()
            if line.startswith("Authorization: Bearer dlg_")
        ),
        None,
    )
    if authorization is None:
        raise RuntimeError(
            "delegation response did not contain a dlg_* authorization line"
        )
    return payload["delegation"], {
        "Authorization": authorization.removeprefix("Authorization: ")
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--email", default=os.getenv("HARU_REFERENCE_EMAIL", DEFAULT_EMAIL)
    )
    parser.add_argument(
        "--name", default=os.getenv("HARU_REFERENCE_NAME", DEFAULT_NAME)
    )
    parser.add_argument("--password", default=os.getenv("HARU_REFERENCE_PASSWORD"))
    args = parser.parse_args()
    if not args.password or len(args.password) < 8:
        raise SystemExit(
            "Set HARU_REFERENCE_PASSWORD (at least 8 characters); it is never printed or saved."
        )

    run_id = str(uuid.uuid4())
    with httpx.Client(base_url=BASE_URL, timeout=60.0) as client:
        owner_headers = learner_headers(client, args.email, args.name, args.password)
        courses = []
        for course in COURSES:
            delegation, author_headers = delegation_token(
                client, owner_headers, course["intent"]
            )
            built = build_reference_course(client, course, headers=author_headers)
            courses.append(
                {
                    "slug": course["slug"],
                    "title": course["title"],
                    "journeyId": built["journeyId"],
                    "journeyUrl": f"{BASE_URL}/learning/journeys/{built['journeyId']}",
                    "delegation": {
                        "id": str(delegation["id"]),
                        "scope": delegation["scope"],
                        "expiresAt": delegation["expiresAt"],
                    },
                }
            )
        owned_journey_ids = {
            journey["id"]
            for journey in require_ok(
                client.get("/api/v1/learning/journeys", headers=owner_headers)
            )
        }
        missing = [
            course["journeyId"]
            for course in courses
            if course["journeyId"] not in owned_journey_ids
        ]
        if missing:
            raise RuntimeError(
                "delegated course did not appear in the learner's owned journey list: "
                + ", ".join(missing)
            )

    handoff = {
        "kind": "local-delegated-reference-courses",
        "runId": run_id,
        "email": args.email,
        "learningUrl": f"{BASE_URL}/learning",
        "courses": courses,
        "completion": "not simulated; these are active published courses",
    }
    handoff_dir = ROOT / ".tmp"
    handoff_dir.mkdir(exist_ok=True)
    handoff_path = handoff_dir / f"haru-reference-courses-{run_id}.json"
    handoff_path.write_text(json.dumps(handoff, indent=2) + "\n")
    print(f"Delegated reference courses ready: {handoff['learningUrl']}")
    print(f"Login email: {args.email}")
    print(
        "Use the HARU_REFERENCE_PASSWORD value you supplied; it is never printed or saved."
    )
    print(f"Non-secret handoff: {handoff_path}")


if __name__ == "__main__":
    try:
        main()
    except (httpx.HTTPError, RuntimeError) as error:
        print(f"Reference course fixture failed: {error}", file=sys.stderr)
        raise SystemExit(1)
