# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx"]
# ///
"""Instructor role simulation — creates content, grades essays, checks stats."""

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
    print("=== Instructor simulation ===")
    c = httpx.Client(base_url=BASE, timeout=15)

    # 1. Login
    r = c.post(
        "/v1/auth/login",
        json={"email": INSTRUCTOR_EMAIL, "password": INSTRUCTOR_PASSWORD},
    )
    step("login as instructor", r.status_code == 200, str(r.status_code))
    token = r.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # 2. Create a quiz
    r = c.post(
        "/v1/quizzes",
        json={"title": f"Sim Quiz {int(time.time())}", "status": "draft"},
        headers=headers,
    )
    step("create quiz", r.status_code in (200, 201), str(r.status_code))
    quiz_id = r.json()["quizId"]

    # 3. Create questions in the bank
    r = c.post(
        "/v1/questions",
        json={
            "questions": [
                {
                    "kind": "mc",
                    "prompt": "Sim MC question?",
                    "payload": {
                        "options": ["A", "B", "C"],
                        "correct_index": 0,
                    },
                    "explanation": "A is correct.",
                    "tags": ["algorithms"],
                },
                {
                    "kind": "tf",
                    "prompt": "Is simulation useful?",
                    "payload": {"correct": True},
                    "tags": ["algorithms"],
                },
                {
                    "kind": "essay",
                    "prompt": "Describe simulation in one sentence.",
                    "payload": {"min_words": 5, "judge": "manual"},
                    "tags": ["algorithms"],
                },
            ]
        },
        headers=headers,
    )
    step("create questions", r.status_code in (200, 201), str(r.status_code))
    q_ids = [q["id"] for q in r.json()["questions"]]

    # 3b. Promote questions to live (required before quiz can be published)
    for qid in q_ids:
        r = c.post(f"/v1/questions/{qid}/promote", headers=headers)
        step("promote question to live", r.status_code == 200, str(r.status_code))

    # 4. Add questions to quiz
    for qid in q_ids:
        r = c.post(
            f"/v1/quizzes/{quiz_id}/questions",
            json={"questionId": qid},
            headers=headers,
        )
        step("add question to quiz", r.status_code in (200, 201, 204), str(r.status_code))

    # 5. Publish quiz
    r = c.patch(f"/v1/quizzes/{quiz_id}", json={"status": "active"}, headers=headers)
    step("publish quiz", r.status_code in (200, 204), str(r.status_code))

    # 6. Compose exam from the quiz
    r = c.post(
        "/v1/exams",
        json={
            "name": f"Sim Exam {int(time.time())}",
            "durationMin": 30,
            "sections": [
                {"title": "Section 1", "weight": 1, "questionIds": q_ids}
            ],
        },
        headers=headers,
    )
    step("compose exam", r.status_code in (200, 201), str(r.status_code))
    exam_id = r.json()["examId"]

    # 7. Publish exam
    r = c.patch(
        f"/v1/exams/{exam_id}", json={"status": "published"}, headers=headers
    )
    step("publish exam", r.status_code in (200, 204), str(r.status_code))

    # 8. Quiz stats
    r = c.get(f"/v1/quizzes/{quiz_id}/stats", headers=headers)
    step("fetch quiz stats", r.status_code == 200, str(r.status_code))

    # 9. Pending essay queue
    r = c.get("/v1/attempts/pending", headers=headers)
    step("list pending essays", r.status_code == 200, f"{len(r.json())} pending")

    print("\n✓ Instructor simulation complete")


if __name__ == "__main__":
    main()
