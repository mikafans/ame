#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Reference client for the AME agent surface.

The agent surface is plain HTTP + JSON (NOT MCP) — see ``GET /llms.txt``. You do
not need this file to use it; it is a dependency-free convenience wrapper and the
canonical worked example. Copy it, import it, or run it directly:

    # create an agent via an owner token
    uv run agents/client.py --api http://localhost:28080 --token <owner-token>

    # or use it as a library
    from client import AmeAgent
    agent = AmeAgent(base_url="http://localhost:28080", api_key="<agent-key>")
    assessments = agent.get("/v1/assessments")
    print(f"Found {len(assessments['assessments'])} assessments")

Two ways to act (mirrors llms.txt):
  - reads + simple writes -> call REST directly:  agent.get/post/patch/delete(path)
  - composite / write tools -> the dispatcher:     agent.run(tool, **params)

The transport is resilient by default: every call has a timeout and retries
transient failures (HTTP 429 / 5xx and network errors) with backoff, honouring a
``Retry-After`` header when the server sends one. Tune via ``timeout`` and
``max_retries`` on the constructor.
"""

from __future__ import annotations

import json
import time
import urllib.error
import urllib.request
from urllib.parse import urlencode

DEFAULT_TIMEOUT = 30.0  # seconds, per request
DEFAULT_RETRIES = 3  # transient-failure retries (429 / 5xx / network)

# Responses worth retrying: rate-limit + transient server errors.
_RETRY_STATUS = frozenset({429, 500, 502, 503, 504})


class AgentError(RuntimeError):
    """A non-2xx response, a network failure, or a /run call that returned ok=false."""

    def __init__(self, message: str, *, status: int | None = None, body=None):
        super().__init__(message)
        self.status = status
        self.body = body


class AmeAgent:
    """A thin authenticated client for one AME agent key."""

    def __init__(
        self,
        base_url: str,
        api_key: str | None = None,
        *,
        timeout: float = DEFAULT_TIMEOUT,
        max_retries: int = DEFAULT_RETRIES,
    ):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout
        self.max_retries = max_retries
        self.agent_id: str | None = None

    # ── bootstrap ────────────────────────────────────────────────────────────
    @classmethod
    def create(
        cls,
        base_url: str,
        owner_token: str,
        label: str = "agent",
        scopes: list[str] | None = None,
        **kwargs,
    ) -> "AmeAgent":
        """POST /v1/me/agents as an owner and return a ready-to-use client.

        Extra keyword args (``timeout``, ``max_retries``) are forwarded to the
        minted client. The new ``agentId`` is available as ``.agent_id``.
        """
        if scopes is None:
            scopes = ["assessment.read", "assessment.write"]

        owner = cls(base_url, owner_token, **kwargs)
        resp = owner.post("/v1/me/agents", {"label": label, "scopes": scopes})

        agent = cls(base_url, resp["apiKey"], **kwargs)
        agent.agent_id = resp["id"]
        return agent

    # ── verbs ────────────────────────────────────────────────────────────────
    def run(self, tool: str, **params):
        """Invoke a runnable tool via POST /v1/agents/run.

        Raises AgentError if the dispatcher reports ok=false.
        """
        resp = self.post("/v1/agents/run", {"tool": tool, "params": params})
        if not resp.get("ok"):
            raise AgentError(f"tool {tool} failed: {resp.get('error')}", body=resp)
        return resp.get("result")

    def get(self, path: str, **query):
        """GET a read endpoint (e.g. /v1/assessments). Read tools are direct REST."""
        if query:
            path = f"{path}?{urlencode(query)}"
        return self.request("GET", path)

    def post(self, path: str, body=None):
        """POST a write endpoint (e.g. /v1/me/agents)."""
        return self.request("POST", path, body)

    def patch(self, path: str, body=None):
        """PATCH a write endpoint (e.g. /v1/assessments/{id})."""
        return self.request("PATCH", path, body)

    def delete(self, path: str):
        """DELETE a resource (e.g. /v1/me/agents/{id})."""
        return self.request("DELETE", path)

    # ── transport ──────────────────────────────────────────────────────────--
    def request(self, method: str, path: str, body=None):
        """Low-level escape hatch: any method/path with auth, JSON, and retries."""
        url = f"{self.base_url}{path}"
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("content-type", "application/json")
        if self.api_key:
            req.add_header("authorization", f"Bearer {self.api_key}")

        for attempt in range(self.max_retries + 1):
            try:
                with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                    raw = resp.read()
                break
            except urllib.error.HTTPError as e:
                if e.code in _RETRY_STATUS and attempt < self.max_retries:
                    time.sleep(_retry_delay(attempt, e))
                    continue
                raise _http_error(method, path, e) from None
            except (urllib.error.URLError, TimeoutError) as e:
                if attempt < self.max_retries:
                    time.sleep(_retry_delay(attempt, None))
                    continue
                reason = getattr(e, "reason", e)
                raise AgentError(f"{method} {path} -> network error: {reason}") from None

        if not raw:
            return None
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return raw.decode(errors="replace")


def _http_error(method: str, path: str, e: urllib.error.HTTPError) -> AgentError:
    """Turn an HTTPError into an AgentError, parsing the JSON body when present."""
    raw = e.read()
    try:
        parsed = json.loads(raw) if raw else None
    except json.JSONDecodeError:
        parsed = raw.decode(errors="replace")
    return AgentError(f"{method} {path} -> {e.code}", status=e.code, body=parsed)


def _retry_delay(attempt: int, err: urllib.error.HTTPError | None) -> float:
    """Backoff seconds for a retry, honouring Retry-After when the server sends it."""
    if err is not None and err.headers:
        retry_after = err.headers.get("Retry-After")
        if retry_after and retry_after.isdigit():
            return float(retry_after)
    return min(2.0**attempt, 8.0)  # 1, 2, 4, 8s cap


# ── runnable demo / smoke test ────────────────────────────────────────────────
def _demo(api: str, token: str, title: str) -> None:
    """Create an agent and author an assessment end to end."""
    print(f"create agent @ {api}")
    agent = AmeAgent.create(
        api, token, label="client-demo", scopes=["assessment.read", "assessment.write"]
    )
    print(f"  agentId={agent.agent_id}")

    print("building question bank...")
    questions = [
        {
            "kind": "mc",
            "prompt": "What is the smallest deployable unit in Kubernetes?",
            "points": 1,
            "explanation": "A Pod wraps one or more containers.",
            "payload": {"options": ["Container", "Pod", "Node", "Service"], "correct_index": 1},
        },
        {
            "kind": "tf",
            "prompt": "A Deployment manages ReplicaSets.",
            "points": 1,
            "payload": {"correct": True},
        },
        {
            "kind": "short",
            "prompt": "Which kubectl subcommand lists pods? (one word)",
            "points": 1,
            "payload": {"accepted": ["get"], "normalize": "exact", "judge": "exact"},
        },
    ]

    created = agent.run("question.create", questions=questions)
    qids = [q["id"] for q in created["questions"]]
    print(f"  created {len(qids)} questions")

    print(f"creating assessment: {title}")
    assessment = agent.run(
        "assessment.create",
        title=title,
        mode="graded",
        objectives=["Describe pods and deployments", "Recall kubectl basics"],
        method="agent",
    )
    aid = assessment["id"]
    print(f"  assessmentId={aid} (draft)")

    print("attaching questions...")
    for qid in qids:
        agent.post(f"/v1/assessments/{aid}/questions", {"questionId": qid})
    print(f"  attached {len(qids)} questions")

    print("publishing...")
    published = agent.run("assessment.update", id=aid, status="active")
    print(f"  status={published['status']}")

    listed = agent.get("/v1/assessments")["assessments"]
    match = next((a for a in listed if a["id"] == aid), None)
    assert match, f"assessment {aid} not visible in /v1/assessments"
    print(f"  library shows: {match['title']!r} ({match['status']})")
    print("\nRESULT: OK")


if __name__ == "__main__":
    import argparse

    ap = argparse.ArgumentParser(description="AME agent reference client demo")
    ap.add_argument("--api", default="http://localhost:28080")
    ap.add_argument("--token", help="Owner token to create the agent")
    ap.add_argument("--title", default="Kubernetes Basics (Phase 2 demo)")
    args = ap.parse_args()

    if not args.token:
        ap.error("--token <owner-token> is required to run the demo")

    try:
        _demo(args.api, args.token, args.title)
    except AgentError as e:
        print(f"Error: {e}")
        print(f"Body: {e.body}")
        exit(1)
