#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Reference client for the AME agent surface.

The agent surface is plain HTTP + JSON (NOT MCP) — see ``GET /llms.txt``. You do
not need this file to use it; it is a dependency-free convenience wrapper and the
canonical worked example. Copy it, import it, or run it directly:

    # run the end-to-end author demo against a dev stack
    uv run agents/client.py --api http://localhost:28080 --code devsmoke

    # or use it as a library
    from client import AmeAgent
    agent = AmeAgent.register("http://localhost:28080", ["quiz.read", "quiz.write"], "devsmoke")
    quiz = agent.run("quiz.import", source=json.dumps({...}))
    agent.run("quiz.update", id=quiz["quizId"], status="active")
    quizzes = agent.get("/v1/quizzes")["quizzes"]

Two ways to act (mirrors llms.txt):
  - reads + simple writes -> call REST directly:  agent.get(path) / agent.request(...)
  - composite / write tools -> the dispatcher:     agent.run(tool, **params)
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request


class AgentError(RuntimeError):
    """A non-2xx response, or a /run call that returned ok=false."""

    def __init__(self, message: str, *, status: int | None = None, body=None):
        super().__init__(message)
        self.status = status
        self.body = body


class AmeAgent:
    """A thin authenticated client for one AME agent key."""

    def __init__(self, base_url: str, api_key: str | None = None):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key

    # ── bootstrap ────────────────────────────────────────────────────────────
    @classmethod
    def register(
        cls,
        base_url: str,
        scopes: list[str],
        access_code: str,
        label: str = "agent",
    ) -> "AmeAgent":
        """POST /v1/agents/register and return a ready-to-use client.

        The minted ``userId`` is available as ``.user_id`` afterwards.
        """
        agent = cls(base_url)
        body = {"label": label, "scopes": scopes, "accessCode": access_code}
        resp = agent.request("POST", "/v1/agents/register", body)
        agent.api_key = resp["apiKey"]
        agent.user_id = resp["userId"]
        return agent

    # ── verbs ────────────────────────────────────────────────────────────────
    def run(self, tool: str, **params):
        """Invoke a runnable write/composite tool via POST /v1/agents/run.

        Raises AgentError if the dispatcher reports ok=false.
        """
        resp = self.request("POST", "/v1/agents/run", {"tool": tool, "params": params})
        if not resp.get("ok"):
            raise AgentError(
                f"tool {tool} failed: {resp.get('error')}", body=resp
            )
        return resp.get("result")

    def get(self, path: str, **query):
        """GET a read endpoint (e.g. /v1/quizzes). Read tools are direct REST."""
        if query:
            from urllib.parse import urlencode

            path = f"{path}?{urlencode(query)}"
        return self.request("GET", path)

    # ── transport ──────────────────────────────────────────────────────────--
    def request(self, method: str, path: str, body=None):
        """Low-level escape hatch: any method/path with auth + JSON handling."""
        url = f"{self.base_url}{path}"
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("content-type", "application/json")
        if self.api_key:
            req.add_header("authorization", f"Bearer {self.api_key}")
        try:
            with urllib.request.urlopen(req) as resp:
                raw = resp.read()
        except urllib.error.HTTPError as e:
            raw = e.read()
            try:
                parsed = json.loads(raw) if raw else None
            except json.JSONDecodeError:
                parsed = raw.decode(errors="replace")
            raise AgentError(
                f"{method} {path} -> {e.code}", status=e.code, body=parsed
            ) from None
        if not raw:
            return None
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return raw.decode(errors="replace")


# ── runnable demo / smoke test ────────────────────────────────────────────────
def _demo(api: str, code: str, title: str) -> None:
    """Author a quiz end to end and confirm it lands in the library."""
    print(f"register write agent @ {api}")
    agent = AmeAgent.register(api, ["quiz.read", "quiz.write"], code, label="client-demo")
    print(f"  userId={agent.user_id}")

    source = json.dumps(
        {
            "title": title,
            "course": "Platform Eng",
            "objectives": ["Describe pods and deployments", "Recall kubectl basics"],
            "questions": [
                {
                    "kind": "mc",
                    "prompt": "What is the smallest deployable unit in Kubernetes?",
                    "points": 1,
                    "explanation": "A Pod wraps one or more containers.",
                    "payload": {"options": ["Container", "Pod", "Node", "Service"], "correct_index": 1},
                },
                {"kind": "tf", "prompt": "A Deployment manages ReplicaSets.", "points": 1,
                 "payload": {"correct": True}},
                {"kind": "short", "prompt": "Which kubectl subcommand lists pods? (one word)", "points": 1,
                 "payload": {"accepted": ["get"], "normalize": "exact", "judge": "exact"}},
            ],
        }
    )
    imported = agent.run("quiz.import", source=source)
    qid = imported["quizId"]
    print(f"  imported quizId={qid} questions={imported['questionsCreated']} (draft)")

    published = agent.run("quiz.update", id=qid, status="active")
    print(f"  published -> status={published['status']}")

    listed = agent.get("/v1/quizzes")["quizzes"]
    match = next((q for q in listed if q["id"] == qid), None)
    assert match, f"quiz {qid} not visible in /v1/quizzes"
    print(f"  library shows: {match['title']!r} ({match['status']}, {match['questionCount']} questions)")
    print(f"\nPUBLISHED_QUIZ_ID={qid}\nRESULT: OK")


if __name__ == "__main__":
    import argparse

    ap = argparse.ArgumentParser(description="AME agent reference client demo")
    ap.add_argument("--api", default="http://localhost:28080")
    ap.add_argument("--code", default="devsmoke", help="AME_AGENT_ACCESS_CODE")
    ap.add_argument("--title", default="Kubernetes Basics (client demo)")
    args = ap.parse_args()
    _demo(args.api, args.code, args.title)
