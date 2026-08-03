"""Small, reusable Python client for the public AME learning contract.

The same file is downloadable from ``/public/sdk/python/ame.py`` so an agent
can run it without cloning AME.  For a packaged install, use the adjacent
``pyproject.toml``.  The generic ``request`` method intentionally remains
available for every operation advertised by ``/public/skill.json``.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import uuid
from dataclasses import dataclass
from typing import Any, Mapping
from urllib import error as urlerror
from urllib import request as urlrequest


@dataclass
class AmeError(RuntimeError):
    """An API error with the server status and structured error payload."""

    status_code: int
    message: str
    code: str | None = None
    details: Any = None
    retry_after: str | None = None

    def __post_init__(self) -> None:
        RuntimeError.__init__(self, self.message)

    def __str__(self) -> str:
        suffix = f"; retry after {self.retry_after}s" if self.retry_after else ""
        return f"AME HTTP {self.status_code}: {self.message}{suffix}"


class AmeClient:
    """HTTP client for one AME public origin and optional bearer capability."""

    def __init__(
        self,
        base_url: str,
        token: str | None = None,
        *,
        timeout: float = 30.0,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        if not self.base_url.startswith(("http://", "https://")):
            raise ValueError("base_url must start with http:// or https://")
        self.token = token
        self.timeout = timeout

    @classmethod
    def from_handoff(cls, handoff: str, **kwargs: Any) -> "AmeClient":
        """Build a client from the one-time handoff shown by the web app."""

        token_match = re.search(r"Authorization:\s*Bearer\s+(\S+)", handoff)
        origin_match = re.search(r"Read\s+(https?://[^/\s]+)", handoff)
        if not token_match:
            raise ValueError("handoff does not contain an Authorization bearer token")
        if not origin_match:
            raise ValueError("handoff does not contain a public AME origin")
        return cls(origin_match.group(1), token_match.group(1), **kwargs)

    def close(self) -> None:
        return None

    def __enter__(self) -> "AmeClient":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    def request(
        self,
        method: str,
        path: str,
        *,
        json_body: Mapping[str, Any] | None = None,
        params: Mapping[str, Any] | None = None,
    ) -> Any:
        """Call any documented AME path and return its decoded response."""

        headers = {"Accept": "application/json"}
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        request = urlrequest.Request(
            self._url(path, params),
            data=json.dumps(json_body).encode("utf-8") if json_body is not None else None,
            headers={**headers, "Content-Type": "application/json"},
            method=method.upper(),
        )
        try:
            response = urlrequest.urlopen(request, timeout=self.timeout)
        except urlerror.HTTPError as response:
            self._raise_error(response)
        except urlerror.URLError as error:
            raise AmeError(0, f"could not reach AME: {error}") from error
        content = response.read()
        if response.status == 204 or not content:
            return None
        content_type = response.headers.get("content-type", "")
        text = content.decode("utf-8")
        return json.loads(text) if "json" in content_type else text

    def discover(self) -> dict[str, Any]:
        return self.request("GET", "/public/skill.json")

    def learning_contract(self) -> dict[str, Any]:
        return self.request("GET", "/public/learning-contract.json")

    def register(self, email: str, name: str, password: str) -> dict[str, Any]:
        response = self.request(
            "POST",
            "/public/v1/auth/register",
            json_body={"email": email, "name": name, "password": password},
        )
        self.token = response["token"]
        return response

    def login(self, email: str, password: str) -> dict[str, Any]:
        response = self.request(
            "POST",
            "/public/v1/auth/login",
            json_body={"email": email, "password": password},
        )
        self.token = response["token"]
        return response

    def start_journey(
        self,
        email: str,
        display_name: str,
        prompt: str,
        *,
        catalog_id: str | None = None,
        idempotency_key: str | None = None,
    ) -> dict[str, Any]:
        body: dict[str, Any] = {
            "email": email,
            "displayName": display_name,
            "prompt": prompt,
            "idempotencyKey": idempotency_key or f"ame-{uuid.uuid4()}",
        }
        if catalog_id:
            body["catalogId"] = catalog_id
        return self.request("POST", "/public/v1/onboarding/start", json_body=body)

    def rate_limit_status(self) -> dict[str, Any]:
        return self.request("GET", "/api/v1/me/rate-limit")

    def _url(self, path: str, params: Mapping[str, Any] | None = None) -> str:
        url = path if path.startswith(("http://", "https://")) else f"{self.base_url}/{path.lstrip('/')}"
        if params:
            from urllib.parse import urlencode

            url = f"{url}?{urlencode(params)}"
        return url

    @staticmethod
    def _raise_error(response: Any) -> None:
        try:
            payload = json.loads(response.read().decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            payload = {}
        envelope = payload.get("error", {}) if isinstance(payload, dict) else {}
        if isinstance(envelope, str):
            message, code, details = envelope, None, None
        else:
            message = envelope.get("message") or payload.get("message") or getattr(response, "reason", "request failed")
            code = envelope.get("code")
            details = envelope.get("details")
        raise AmeError(
            response.code,
            message,
            code,
            details,
            response.headers.get("retry-after"),
        )


def _main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Call the AME public learning contract")
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--token")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("discover")
    sub.add_parser("contract")
    sub.add_parser("rate-limit")
    request = sub.add_parser("request")
    request.add_argument("method")
    request.add_argument("path")
    request.add_argument("--json", dest="json_text")
    args = parser.parse_args(argv)
    try:
        with AmeClient(args.base_url, args.token) as client:
            if args.command == "discover":
                result = client.discover()
            elif args.command == "contract":
                result = client.learning_contract()
            elif args.command == "rate-limit":
                result = client.rate_limit_status()
            else:
                result = client.request(
                    args.method,
                    args.path,
                    json_body=json.loads(args.json_text) if args.json_text else None,
                )
        print(json.dumps(result, indent=2, ensure_ascii=False))
        return 0
    except (AmeError, ValueError, json.JSONDecodeError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(_main(sys.argv[1:]))
