# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx"]
# ///
"""Learner role simulation — walks the full learner flow against a live API."""

import sys
import time
import httpx

BASE = "http://localhost:8080"
EMAIL = f"sim-learner-{int(time.time())}@example.com"
PASSWORD = "sim-password-123"


def step(label: str, ok: bool, detail: str = "") -> None:
    status = "✓" if ok else "✗"
    print(f"  {status} {label}" + (f": {detail}" if detail else ""))
    if not ok:
        sys.exit(1)


def make_response(kind: str) -> dict:
    if kind in ("mc", "mcq"):
        return {"selected_position": 0}
    if kind == "tf":
        return {"answer": True}
    if kind == "short":
        return {"answer": "simulation answer"}
    if kind == "essay":
        return {"body": "This is a simulation answer for the essay question.", "word_count": 9}
    if kind == "code":
        return {"source": "print('hello')", "language": "python"}
    return {"answer": "simulation answer"}


def answer_all(c: httpx.Client, session_id: str, questions: list, headers: dict, label: str) -> None:
    for q in questions:
        kind = q["kind"]
        r = c.post(
            f"/v1/sessions/{session_id}/answer",
            json={"questionId": q["id"], "response": make_response(kind)},
            headers=headers,
        )
        step(f"answer {label} {kind}", r.status_code == 200, str(r.status_code))


def main() -> None:
    print("=== Learner simulation ===")
    c = httpx.Client(base_url=BASE, timeout=15)

    # 1. Register
    r = c.post(
        "/v1/auth/register",
        json={"email": EMAIL, "password": PASSWORD, "name": "Sim Learner", "role": "learner"},
    )
    step("register", r.status_code == 201, str(r.status_code))
    token = r.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # 2. List assessments
    r = c.get("/v1/assessments", params={"status": "active"}, headers=headers)
    step("list assessments", r.status_code == 200)
    assessments = r.json().get("assessments", [])
    step("at least one active assessment", len(assessments) > 0, f"found {len(assessments)}")
    assessment_id = assessments[0]["id"]

    # 3. Start assessment session
    r = c.post("/v1/sessions", json={"assessmentId": assessment_id}, headers=headers)
    step("start assessment session", r.status_code == 201, str(r.status_code))
    session_id = r.json()["sessionId"]
    questions = r.json().get("questions", [])
    step("session has questions", len(questions) > 0, f"{len(questions)} questions")

    # 4. Answer all questions and finish
    answer_all(c, session_id, questions, headers, "assessment")
    r = c.post(f"/v1/sessions/{session_id}/finish", headers=headers)
    step("finish assessment session", r.status_code == 200, str(r.status_code))

    # 5. Fetch session results
    r = c.get(f"/v1/sessions/{session_id}", headers=headers)
    step("fetch session results", r.status_code == 200)
    result = r.json().get("session", {}).get("result", {})
    step("result has score", "points_awarded" in result, str(result.get("points_awarded")))

    # 6. Find a published exam (instructor must have published one)
    r = c.get("/v1/exams", headers=headers)
    step("list exams", r.status_code == 200)
    exams = r.json().get("exams", [])
    exam_id = next((e["id"] for e in exams if e.get("status") == "published"), None)
    step("published exam available", exam_id is not None, f"{len(exams)} total exams")

    # 7. Start exam session
    r = c.post("/v1/sessions", json={"examId": exam_id}, headers=headers)
    step("start exam session", r.status_code == 201, str(r.status_code))
    exam_session_id = r.json()["sessionId"]
    exam_questions = r.json().get("questions", [])
    step("exam session has questions", len(exam_questions) > 0, f"{len(exam_questions)} questions")

    # 8. Answer exam questions and finish
    answer_all(c, exam_session_id, exam_questions, headers, "exam")
    r = c.post(f"/v1/sessions/{exam_session_id}/finish", headers=headers)
    step("finish exam session", r.status_code == 200, str(r.status_code))

    # 9. Personal stats
    r = c.get("/v1/me/stats", headers=headers)
    step("fetch personal stats", r.status_code == 200)
    step("stats has avg_score", "avg_score" in r.json(), str(r.json().get("avg_score")))

    print("\n✓ Learner simulation complete")


if __name__ == "__main__":
    main()
