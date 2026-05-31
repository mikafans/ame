# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx"]
# ///
"""Agent role simulation — registers an agent, manages knowledge base, generates assessment, fetches stats, creates plan."""

import sys
import time
import httpx

BASE = "http://localhost:8080"
INSTRUCTOR_EMAIL = "instructor@example.com"
INSTRUCTOR_PASSWORD = "password123"


def step(label: str, ok: bool, detail: str = "") -> None:
    status = "✓" if ok else "✗"
    print(f"  {status} {label}" + (f": {detail}" if detail else ""))
    if not ok:
        sys.exit(1)


def main() -> None:
    print("=== Agent simulation ===")
    c = httpx.Client(base_url=BASE, timeout=15)

    # Bootstrap: instructor token to register the agent
    r = c.post(
        "/v1/auth/login",
        json={"email": INSTRUCTOR_EMAIL, "password": INSTRUCTOR_PASSWORD},
    )
    step("bootstrap instructor login", r.status_code == 200)
    inst_token = r.json()["token"]
    inst_headers = {"Authorization": f"Bearer {inst_token}"}

    # 1. Register agent
    r = c.post(
        "/v1/agents/register",
        json={
            "label": f"sim-agent-{int(time.time())}",
            "scopes": [
                "assessment.read",
                "assessment.write",
                "stats.read",
                "plan.write",
                "plan.read",
            ],
        },
        headers=inst_headers,
    )
    step("register agent", r.status_code in (200, 201), str(r.status_code))
    agent_key = r.json()["apiKey"]  # API returns "apiKey"
    step("received API key", bool(agent_key))
    agent_headers = {"Authorization": f"Bearer {agent_key}"}

    # 2. Import questions
    r = c.post(
        "/v1/questions",
        json={
            "questions": [
                {
                    "kind": "mc",
                    "prompt": "Which Rust keyword declares a module?",
                    "payload": {
                        "options": [
                            {"text": "let"},
                            {"text": "fn"},
                            {"text": "mod"},
                            {"text": "use"},
                        ],
                        "correct_index": 2,
                    },
                    "explanation": "`mod` declares a module, creating a new scope.",
                    "tags": ["rust", "modules"],
                },
                {
                    "kind": "tf",
                    "prompt": "Rust guarantees memory safety at compile time.",
                    "payload": {"correct": True},
                    "explanation": "The borrow checker enforces this at compile time.",
                    "tags": ["rust", "memory"],
                },
                {
                    "kind": "short",
                    "prompt": "What does `&mut` mean in Rust?",
                    "payload": {
                        "accepted": ["mutable reference", "mutable borrow"],
                        "normalize": "case_insensitive_strip_accents",
                    },
                    "tags": ["rust", "borrowing"],
                },
            ]
        },
        headers=agent_headers,
    )
    step("import questions", r.status_code in (200, 201), str(r.status_code))
    q_ids = [q["id"] for q in r.json()["questions"]]
    step("3 questions created", len(q_ids) == 3, str(len(q_ids)))

    # 3. Promote questions to live
    for qid in q_ids:
        r = c.post(f"/v1/questions/{qid}/promote", headers=agent_headers)
        step("promote question", r.status_code in (200, 204), str(r.status_code))

    # 4. List live questions
    r = c.get(
        "/v1/questions",
        params={"status": "live", "tag": "rust"},
        headers=agent_headers,
    )
    step("list live rust questions", r.status_code == 200)
    count = len(r.json()["questions"])
    step("at least 3 questions in bank", count >= 3, str(count))

    # 5. Generate assessment from source text (endpoint may be stub)
    r = c.post(
        "/v1/assessments/generate",
        json={
            "source": "Rust is a systems programming language that prevents segfaults and guarantees thread safety through its ownership system.",
            "questionCount": 3,
            "types": ["mc", "tf"],
            "difficulty": "inter",
        },
        headers=agent_headers,
    )
    step("generate assessment", r.status_code in (200, 201), str(r.status_code))
    gen_assessment_id = r.json().get("id") or r.json().get("assessmentId")
    is_stub = any("not yet implemented" in w for w in r.json().get("warnings", []))
    step("assessment id returned (or stub)", bool(gen_assessment_id) or is_stub, "stub" if is_stub else str(gen_assessment_id))

    # 6. Fetch user stats (instructor has attempt history)
    r = c.get("/v1/me/stats", headers=inst_headers)
    step("fetch user stats", r.status_code == 200)
    step("stats has avg_score field", "avg_score" in r.json())

    # 7. Create study plan
    r = c.post(
        "/v1/plans",
        json={
            "goal": "Master Rust ownership and borrowing in 4 weeks",
            "lookbackDays": 30,
        },
        headers=inst_headers,
    )
    step("create study plan", r.status_code in (200, 201), str(r.status_code))
    plan_id = r.json()["id"]
    step("plan id returned", bool(plan_id))

    # 8. Fetch study plan
    r = c.get(f"/v1/plans/{plan_id}", headers=inst_headers)
    step("fetch study plan", r.status_code == 200)
    weeks = r.json().get("weeks", [])
    step("plan has weeks", len(weeks) > 0, f"{len(weeks)} weeks")

    # 9. Agent activity log
    r = c.get("/v1/agents/activity", headers=agent_headers)
    step("fetch agent activity log", r.status_code == 200)

    print("\n✓ Agent simulation complete")


if __name__ == "__main__":
    main()
