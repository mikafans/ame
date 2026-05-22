# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27", "rich>=13"]
# ///
"""
Seed the ame database with demo users, tags, questions, quizzes, and exams.

Usage:
    uv run scripts/seed.py
    uv run scripts/seed.py --api http://localhost:8080
    uv run scripts/seed.py --wipe   # drop existing seed data first (re-seeds)
"""

import argparse
import sys
from dataclasses import dataclass, field
from typing import Any

import httpx
from rich.console import Console
from rich.table import Table

console = Console()

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

DEFAULT_API = "http://localhost:8080"

USERS = [
    {"email": "learner@example.com", "name": "Alice Learner", "password": "password123", "role": "learner"},
    {"email": "instructor@example.com", "name": "Bob Instructor", "password": "password123", "role": "instructor"},
    {"email": "admin@example.com", "name": "Carol Admin", "password": "password123", "role": "admin"},
]

TAGS = [
    {"name": "algorithms", "description": "Algorithm design and analysis"},
    {"name": "data-structures", "description": "Arrays, trees, graphs, etc."},
    {"name": "sorting", "description": "Sorting algorithms and complexity"},
    {"name": "graphs", "description": "Graph traversal and shortest paths"},
    {"name": "dynamic-programming", "description": "Optimal substructure, memoization"},
    {"name": "math", "description": "Mathematics and discrete math"},
    {"name": "python", "description": "Python language and stdlib"},
]

QUESTIONS = [
    # Multiple choice
    {
        "kind": "mc",
        "prompt": "What is the time complexity of binary search on a sorted array?",
        "payload": {
            "options": ["O(n)", "O(log n)", "O(n log n)", "O(1)"],
            "correct_index": 1,
        },
        "explanation": "Binary search halves the search space each step, giving O(log n).",
        "tags": ["algorithms", "sorting"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which data structure is used to implement a breadth-first search?",
        "payload": {
            "options": ["Stack", "Queue", "Heap", "Hash map"],
            "correct_index": 1,
        },
        "explanation": "BFS uses a queue (FIFO) to visit nodes level by level.",
        "tags": ["graphs", "data-structures"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What is the worst-case time complexity of quicksort?",
        "payload": {
            "options": ["O(n log n)", "O(n)", "O(n²)", "O(log n)"],
            "correct_index": 2,
        },
        "explanation": "Quicksort degrades to O(n²) when the pivot is always the smallest or largest element.",
        "tags": ["sorting", "algorithms"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "In dynamic programming, what property ensures a problem can be solved optimally by combining solutions to subproblems?",
        "payload": {
            "options": ["Greedy choice", "Optimal substructure", "Divide and conquer", "Memoization"],
            "correct_index": 1,
        },
        "explanation": "Optimal substructure means the optimal solution to the whole problem contains optimal solutions to subproblems.",
        "tags": ["dynamic-programming", "algorithms"],
        "points": 2,
    },
    # True / False
    {
        "kind": "tf",
        "prompt": "A binary search tree guarantees O(log n) search time in all cases.",
        "payload": {"correct": False},
        "explanation": "An unbalanced BST can degrade to O(n) in the worst case (e.g., inserting a sorted sequence).",
        "tags": ["data-structures", "algorithms"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "Merge sort is a stable sorting algorithm.",
        "payload": {"correct": True},
        "explanation": "Merge sort preserves the relative order of equal elements, making it stable.",
        "tags": ["sorting"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "Dijkstra's algorithm can handle graphs with negative edge weights.",
        "payload": {"correct": False},
        "explanation": "Dijkstra's requires non-negative weights. Use Bellman-Ford for negative edges.",
        "tags": ["graphs"],
        "points": 1,
    },
    # Short answer
    {
        "kind": "short",
        "prompt": "Name the data structure that follows Last-In, First-Out (LIFO) ordering.",
        "payload": {
            "accepted": ["stack", "Stack", "STACK"],
            "normalize": "case_insensitive_strip_accents",
            "judge": "exact",
        },
        "explanation": "A stack processes the most recently added item first.",
        "tags": ["data-structures"],
        "points": 1,
    },
    {
        "kind": "short",
        "prompt": "What Python built-in function returns the length of a sequence?",
        "payload": {
            "accepted": ["len", "len()", "len()"],
            "normalize": "exact",
            "judge": "exact",
        },
        "explanation": "len() returns the number of items in an object.",
        "tags": ["python"],
        "points": 1,
    },
    # Essay
    {
        "kind": "essay",
        "prompt": "Compare depth-first search and breadth-first search. When would you choose each?",
        "payload": {
            "min_words": 60,
            "rubric": "Award marks for: correct traversal description (2), time/space complexity comparison (2), appropriate use-case examples (2).",
            "judge": "manual",
        },
        "explanation": "DFS uses O(depth) space and is good for cycle detection/topological sort. BFS uses O(width) space and finds shortest paths in unweighted graphs.",
        "tags": ["graphs", "algorithms"],
        "points": 6,
    },
    {
        "kind": "essay",
        "prompt": "Explain how memoization differs from tabulation in dynamic programming, with an example for each.",
        "payload": {
            "min_words": 80,
            "rubric": "Award marks for: correct definition of memoization (2), correct definition of tabulation (2), valid example for each (2), space/time trade-off discussion (2).",
            "judge": "manual",
        },
        "tags": ["dynamic-programming"],
        "points": 8,
    },
]

QUIZ = {
    "title": "Algorithms and Data Structures — Fundamentals",
    "objectives": [
        "Understand time complexity of common algorithms",
        "Distinguish between core data structures",
        "Apply sorting and graph algorithms",
    ],
}

EXAM_BLUEPRINT = {
    "name": "CS Fundamentals Midterm",
    "description": "Covers algorithms, data structures, and complexity analysis.",
    "duration": 60,
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

@dataclass
class SeedResult:
    users: list[dict] = field(default_factory=list)
    tags: list[dict] = field(default_factory=list)
    questions: list[dict] = field(default_factory=list)
    quiz_id: str | None = None
    exam_id: str | None = None
    cohort_id: str | None = None


def post(client: httpx.Client, path: str, body: dict, auth: dict | None = None, label: str = "") -> dict:
    headers = auth or {}
    resp = client.post(path, json=body, headers=headers)
    if not resp.is_success:
        console.print(f"[red]FAIL[/red] POST {path} → {resp.status_code}: {resp.text[:200]}")
        if label:
            console.print(f"  ({label})")
    return resp.json() if resp.is_success else {}


def register_or_login(client: httpx.Client, user: dict) -> dict | None:
    resp = client.post("/v1/auth/register", json=user)
    if resp.status_code == 201:
        return resp.json()
    if resp.status_code in (400, 422):
        # Email already registered — log in instead
        resp2 = client.post("/v1/auth/login", json={"email": user["email"], "password": user["password"]})
        if resp2.is_success:
            return resp2.json()
    console.print(f"[red]Could not register/login {user['email']}:[/red] {resp.text[:200]}")
    return None


# ---------------------------------------------------------------------------
# Seed steps
# ---------------------------------------------------------------------------

def seed_users(client: httpx.Client, result: SeedResult) -> None:
    console.rule("[bold]Users")
    for u in USERS:
        data = register_or_login(client, u)
        if data:
            result.users.append({**u, "token": data["token"], "id": data["user"]["id"]})
            console.print(f"  [green]✓[/green] {u['role']:12} {u['email']}")


def seed_tags(client: httpx.Client, auth: dict, result: SeedResult) -> None:
    console.rule("[bold]Tags")
    for tag in TAGS:
        resp = client.post("/v1/tags", json=tag, headers=auth)
        if resp.status_code in (200, 201):
            result.tags.append(resp.json())
            console.print(f"  [green]✓[/green] #{tag['name']}")
        else:
            console.print(f"  [yellow]skip[/yellow] #{tag['name']} ({resp.status_code})")


def seed_questions(client: httpx.Client, auth: dict, result: SeedResult) -> None:
    console.rule("[bold]Questions")
    resp = client.post("/v1/questions", json={"questions": QUESTIONS}, headers=auth)
    if resp.status_code != 201:
        console.print(f"[red]Failed to create questions:[/red] {resp.text[:400]}")
        return

    created = resp.json()["questions"]
    result.questions = created
    console.print(f"  Created {len(created)} questions (draft)")

    # Promote all to live
    promoted = 0
    for q in created:
        r = client.post(f"/v1/questions/{q['id']}/promote", headers=auth)
        if r.is_success:
            promoted += 1
    console.print(f"  Promoted {promoted}/{len(created)} questions to live")


def seed_quiz(client: httpx.Client, auth: dict, result: SeedResult) -> None:
    console.rule("[bold]Quiz")
    resp = client.post("/v1/quizzes", json=QUIZ, headers=auth)
    if not resp.is_success:
        console.print(f"[red]Failed to create quiz:[/red] {resp.text[:200]}")
        return

    quiz_id = resp.json()["quiz"]["id"]
    result.quiz_id = quiz_id
    console.print(f"  Created quiz {quiz_id}")

    # Add live bank questions to the quiz
    added = 0
    for q in result.questions:
        r = client.post(
            f"/v1/quizzes/{quiz_id}/questions",
            json={"questionId": q["id"]},
            headers=auth,
        )
        if r.is_success:
            added += 1
    console.print(f"  Linked {added}/{len(result.questions)} questions to quiz")

    # Publish
    resp2 = client.patch(f"/v1/quizzes/{quiz_id}", json={"status": "active"}, headers=auth)
    if resp2.is_success:
        console.print("  Published quiz → active")
    else:
        console.print(f"  [yellow]Could not publish:[/yellow] {resp2.text[:200]}")


def seed_exam(client: httpx.Client, auth: dict, result: SeedResult) -> None:
    console.rule("[bold]Exam")
    if not result.questions:
        console.print("  [yellow]No questions available — skipping exam[/yellow]")
        return

    mc_ids = [q["id"] for q in result.questions if q["kind"] == "mc"]
    essay_ids = [q["id"] for q in result.questions if q["kind"] == "essay"]

    if not mc_ids:
        console.print("  [yellow]No multiple-choice questions — skipping exam[/yellow]")
        return

    sections = [{"title": "Multiple Choice", "questionIds": mc_ids, "weight": 0.6}]
    if essay_ids:
        sections.append({"title": "Essay Questions", "questionIds": essay_ids, "weight": 0.4})

    resp = client.post("/v1/exams", json={**EXAM_BLUEPRINT, "sections": sections}, headers=auth)
    if resp.is_success:
        exam_id = resp.json().get("id") or resp.json().get("exam_id")
        result.exam_id = exam_id
        console.print(f"  Created exam {exam_id} with {len(sections)} sections")
    else:
        console.print(f"  [yellow]Could not create exam (non-fatal):[/yellow] {resp.status_code} {resp.text[:200]}")


def seed_cohort(client: httpx.Client, auth: dict, result: SeedResult) -> None:
    console.rule("[bold]Cohort")
    resp = client.post(
        "/v1/cohorts",
        json={"name": "CS '27", "description": "Computer Science 2027 cohort"},
        headers=auth,
    )
    if not resp.is_success:
        console.print(f"  [yellow]Could not create cohort (non-fatal):[/yellow] {resp.status_code} {resp.text[:200]}")
        return

    cohort_id = resp.json().get("id")
    result.cohort_id = cohort_id
    console.print(f"  Created cohort {cohort_id}")

    learners = [u for u in result.users if u["role"] == "learner"]
    enrolled = 0
    for u in learners:
        r = client.post(
            f"/v1/cohorts/{cohort_id}/members",
            json={"userId": u["id"]},
            headers=auth,
        )
        if r.is_success:
            enrolled += 1
    console.print(f"  Enrolled {enrolled}/{len(learners)} learners")


def print_summary(result: SeedResult) -> None:
    console.rule("[bold green]Seed complete")
    table = Table(show_header=True, header_style="bold")
    table.add_column("Resource")
    table.add_column("Count / ID")
    table.add_row("Users", str(len(result.users)))
    table.add_row("Tags", str(len(result.tags)))
    table.add_row("Questions (live)", str(len(result.questions)))
    table.add_row("Quiz", result.quiz_id or "—")
    table.add_row("Exam", result.exam_id or "—")
    table.add_row("Cohort", result.cohort_id or "—")
    console.print(table)

    console.rule("[bold]Credentials")
    for u in result.users:
        console.print(f"  [cyan]{u['role']:12}[/cyan]  {u['email']}  /  {u['password']}")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Seed demo data into a running ame API.")
    parser.add_argument("--api", default=DEFAULT_API, help="Base URL of the API (default: %(default)s)")
    args = parser.parse_args()

    console.print(f"\n[bold]ame seed[/bold] → {args.api}\n")

    result = SeedResult()

    with httpx.Client(base_url=args.api, timeout=15.0) as client:
        # Health check
        health = client.get("/healthz")
        if not health.is_success:
            console.print(f"[red]API not reachable at {args.api}[/red]")
            sys.exit(1)

        seed_users(client, result)

        instructor = next((u for u in result.users if u["role"] == "instructor"), None)
        if not instructor:
            console.print("[red]No instructor user — cannot seed content[/red]")
            sys.exit(1)
        auth = {"Authorization": f"Bearer {instructor['token']}"}

        admin = next((u for u in result.users if u["role"] == "admin"), None)
        admin_auth = {"Authorization": f"Bearer {admin['token']}"} if admin else auth

        seed_tags(client, auth, result)
        seed_questions(client, auth, result)
        seed_quiz(client, auth, result)
        seed_exam(client, auth, result)
        seed_cohort(client, admin_auth, result)

    print_summary(result)


if __name__ == "__main__":
    main()
