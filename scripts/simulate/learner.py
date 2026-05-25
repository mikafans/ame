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


def main() -> None:
    print("=== Learner simulation ===")
    c = httpx.Client(base_url=BASE, timeout=15)

    # 1. Register
    r = c.post(
        "/v1/auth/register",
        json={"email": EMAIL, "password": PASSWORD, "displayName": "Sim Learner"},
    )
    step("register", r.status_code == 201, str(r.status_code))
    token = r.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # 2. List quizzes
    r = c.get("/v1/quizzes", params={"status": "active"}, headers=headers)
    step("list quizzes", r.status_code == 200)
    quizzes = r.json().get("quizzes", [])
    step("at least one active quiz", len(quizzes) > 0, f"found {len(quizzes)}")
    quiz_id = quizzes[0]["id"]

    # 3. Start quiz session
    r = c.post("/v1/sessions", json={"quizId": quiz_id}, headers=headers)
    step("start quiz session", r.status_code == 200, str(r.status_code))
    session_id = r.json()["sessionId"]
    questions = r.json().get("questions", [])
    step("session has questions", len(questions) > 0, f"{len(questions)} questions")

    # 4. Answer all questions
    for q in questions:
        kind = q["kind"]
        if kind in ("mc", "mcq"):
            response = 0
        elif kind == "tf":
            response = True
        else:
            response = "simulation answer"
        r = c.post(
            f"/v1/sessions/{session_id}/answer",
            json={"questionId": q["questionId"], "response": response},
            headers=headers,
        )
        step(f"answer {kind} question", r.status_code == 200, str(r.status_code))

    # 5. Finish quiz session
    r = c.post(f"/v1/sessions/{session_id}/finish", headers=headers)
    step("finish quiz session", r.status_code == 200, str(r.status_code))

    # 6. Fetch session results
    r = c.get(f"/v1/sessions/{session_id}", headers=headers)
    step("fetch session results", r.status_code == 200)
    result = r.json().get("session", {}).get("result", {})
    step("result has score", "points_awarded" in result, str(result.get("points_awarded")))

    # 7. List exams and start one
    r = c.get("/v1/exams", headers=headers)
    step("list exams", r.status_code == 200)
    exams = r.json().get("exams", [])
    step("at least one exam exists", len(exams) > 0, f"found {len(exams)}")
    exam_id = next((e["id"] for e in exams if e.get("status") == "published"), None)
    step("published exam found", exam_id is not None)

    r = c.post("/v1/sessions", json={"examId": exam_id}, headers=headers)
    step("start exam session", r.status_code == 200, str(r.status_code))
    exam_session_id = r.json()["sessionId"]
    exam_questions = r.json().get("questions", [])
    step(
        "exam session has questions",
        len(exam_questions) > 0,
        f"{len(exam_questions)} questions",
    )

    # 8. Answer exam questions and finish
    for q in exam_questions:
        kind = q["kind"]
        if kind in ("mc", "mcq"):
            response = 0
        elif kind == "tf":
            response = True
        else:
            response = "simulation answer"
        r = c.post(
            f"/v1/sessions/{exam_session_id}/answer",
            json={"questionId": q["questionId"], "response": response},
            headers=headers,
        )
        step(f"answer exam {kind}", r.status_code == 200, str(r.status_code))

    r = c.post(f"/v1/sessions/{exam_session_id}/finish", headers=headers)
    step("finish exam session", r.status_code == 200, str(r.status_code))

    # 9. Personal stats
    r = c.get("/v1/me/stats", headers=headers)
    step("fetch personal stats", r.status_code == 200)
    stats = r.json()
    step("stats has avg_score", "avg_score" in stats, str(stats.get("avg_score")))

    print("\n✓ Learner simulation complete")


if __name__ == "__main__":
    main()
