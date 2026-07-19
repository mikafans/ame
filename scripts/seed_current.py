"""Seed the clean-slate local stack through the current learner contract."""

from __future__ import annotations

import argparse
import json
import urllib.error
import urllib.request
import uuid


def request(api: str, method: str, path: str, body: dict, token: str | None = None) -> tuple[int, dict]:
    payload = json.dumps(body).encode()
    req = urllib.request.Request(
        f"{api.rstrip('/')}{path}",
        data=payload,
        method=method,
        headers={
            "content-type": "application/json",
            **({"authorization": f"Bearer {token}"} if token else {}),
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            return response.status, json.loads(response.read())
    except urllib.error.HTTPError as error:
        raw = error.read()
        try:
            detail = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            detail = {"message": raw.decode(errors="replace")}
        return error.code, detail


def learner_token(api: str, email: str, password: str, name: str) -> str:
    status, response = request(
        api,
        "POST",
        "/v1/auth/register",
        {"email": email, "name": name, "password": password},
    )
    if status == 201:
        return response["token"]
    if status != 422:
        raise SystemExit(f"registration failed ({status}): {response}")

    status, response = request(
        api,
        "POST",
        "/v1/auth/login",
        {"email": email, "password": password},
    )
    if status != 200:
        raise SystemExit(f"login failed ({status}): {response}")
    return response["token"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api", default="http://localhost:28800")
    parser.add_argument("--email", default="haru@example.com")
    parser.add_argument("--name", default="haru")
    parser.add_argument("--password", default="password123")
    parser.add_argument(
        "--prompt",
        default="I would like to learn a new subject",
    )
    args = parser.parse_args()

    token = learner_token(args.api, args.email, args.password, args.name)
    status, response = request(
        args.api,
        "POST",
        "/v1/onboarding/start",
        {
            "email": args.email,
            "displayName": args.name,
            "prompt": args.prompt,
            "idempotencyKey": f"local-{uuid.uuid5(uuid.NAMESPACE_URL, args.email + args.prompt)}",
        },
        token,
    )
    if status not in (200, 201):
        raise SystemExit(f"onboarding failed ({status}): {response}")

    print(f"current learner ready → {args.email} / {args.password}")
    print(f"journey → {response['journeyId']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
