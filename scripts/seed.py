# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27", "rich>=13"]
# ///
"""
Seed the ame database with demo users, tags, questions, quizzes, and exams.

Usage:
    uv run scripts/seed.py
    uv run scripts/seed.py --api http://localhost:8080
"""

import argparse
import os
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

DEFAULT_API = os.environ.get("AME_API_URL", "http://localhost:8080")

USERS = [
    {"email": "learner@example.com", "name": "Alice Learner", "password": "password123", "role": "user"},
    {"email": "instructor@example.com", "name": "Bob Instructor", "password": "password123", "role": "user"},
    
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
    # Multiple choice — options are bare strings matching McPayload.options: Vec<String>
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
            "accepted": ["len", "len()"],
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
    # Code — uses exemplar for auto-grading; falls back to pending_manual without one
    {
        "kind": "code",
        "prompt": "Write a Python function `triangle_sum(n)` that returns the sum 1 + 2 + ... + n.",
        "payload": {
            "language": "python",
            "starter": "def triangle_sum(n):\n    pass",
            "tests": [
                {"name": "triangle_sum(1) == 1", "body": "assert triangle_sum(1) == 1"},
                {"name": "triangle_sum(5) == 15", "body": "assert triangle_sum(5) == 15"},
                {"name": "triangle_sum(10) == 55", "body": "assert triangle_sum(10) == 55"},
            ],
            "exemplar": "def triangle_sum(n):\n    return n * (n + 1) // 2",
        },
        "explanation": "The closed-form formula n*(n+1)//2 computes the sum in O(1) time.",
        "tags": ["math", "python"],
        "points": 3,
    },
]

QUIZ = {
    "title": "Algorithms and Data Structures — Fundamentals",
    "course": "Computer Science",
    "visibility": "public",
    "objectives": [
        "Understand time complexity of common algorithms",
        "Distinguish between core data structures",
        "Apply sorting and graph algorithms",
    ],
}

QUIZ_PYTHON = {
    "title": "Python Essentials",
    "course": "Programming",
    "visibility": "public",
    "difficulty": "beginner",
    "objectives": [
        "Use Python built-in functions correctly",
        "Write small functions with correct logic",
    ],
}

QUIZ_DRAFT = {
    "title": "Algorithmic Complexity & Graph Theory (Draft)",
    "course": "Computer Science",
    "visibility": "private",
    "objectives": [
        "Analyze recurrence relations",
        "Implement DFS and BFS graph traversals",
    ],
}

EXAM_BLUEPRINT = {
    "title": "CS Fundamentals Midterm",
    "description": "Covers algorithms, data structures, and complexity analysis.",
    "objectives": [
        "Analyze time and space complexity using Big O notation",
        "Implement basic data structures and algorithms",
    ],
    "duration": 60,
    "passing_points": 10,
}

# Answers to submit when seeding learner attempts.
# Each entry maps to the corresponding QUESTIONS item by index.
# MC: selected_position that maps to the correct option (position == correct_index
#     when option_order is identity, which it may not be — we derive it at runtime).
# TF/Short: hardcoded correct values.
# Essay: sample body submitted for manual grading queue.
# Code: the exemplar solution.
QUESTION_ANSWERS = [
    {"kind": "mc", "correct_index": 1},                      # binary search
    {"kind": "mc", "correct_index": 1},                      # BFS
    {"kind": "mc", "correct_index": 2},                      # quicksort
    {"kind": "mc", "correct_index": 1},                      # DP substructure
    {"kind": "tf", "answer": False},                         # BST guarantee
    {"kind": "tf", "answer": True},                          # merge sort stable
    {"kind": "tf", "answer": False},                         # Dijkstra negative
    {"kind": "short", "answer": "stack"},                    # LIFO
    {"kind": "short", "answer": "len"},                      # Python len
    {                                                         # DFS vs BFS essay
        "kind": "essay",
        "body": (
            "Depth-first search (DFS) explores as far as possible along each branch before "
            "backtracking. It uses a stack (explicit or call stack) and requires O(depth) space. "
            "DFS is well-suited for cycle detection, topological sorting, and exhaustive path "
            "enumeration. Breadth-first search (BFS) visits all neighbors at the current depth "
            "before moving deeper, using a queue (O(width) space). BFS guarantees the shortest "
            "path in unweighted graphs and is ideal for level-order traversal or finding the "
            "minimum number of hops between nodes."
        ),
    },
    {                                                         # memoization vs tabulation
        "kind": "essay",
        "body": (
            "Memoization is a top-down approach: a recursive function caches results for "
            "subproblems as they are computed, avoiding redundant work. Example: Fibonacci "
            "with a dict cache. Tabulation is bottom-up: fill a table from base cases up to "
            "the target. Example: a loop building fib[0..n]. Memoization is more natural for "
            "problems with complex recursion; tabulation avoids call-stack overhead and is "
            "often faster in practice. Both have the same asymptotic complexity for well-defined "
            "DP problems."
        ),
    },
    {                                                         # code: triangle_sum
        "kind": "code",
        "source": "def triangle_sum(n):\n    return n * (n + 1) // 2",
        "language": "python",
    },
]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

@dataclass
class SeedResult:
    users: list[dict] = field(default_factory=list)
    tags: list[dict] = field(default_factory=list)
    questions: list[dict] = field(default_factory=list)
    quiz_id: str | None = None
    quiz_python_id: str | None = None
    exam_id: str | None = None
    cohort_id: str | None = None
    attempts_seeded: int = 0


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
    by_kind = {}
    for q in created:
        by_kind[q["kind"]] = by_kind.get(q["kind"], 0) + 1
    summary = " · ".join(f"{n} {k}" for k, n in sorted(by_kind.items()))
    console.print(f"  Created {len(created)} questions ({summary})")

    promoted = 0
    for q in created:
        r = client.post(f"/v1/questions/{q['id']}/promote", headers=auth)
        if r.is_success:
            promoted += 1
    console.print(f"  Promoted {promoted}/{len(created)} questions to live")


def _find_assessment_by_title(client: httpx.Client, auth: dict, title: str) -> str | None:
    """Return the id of an existing active assessment with this title, if any.

    Keeps the seed idempotent so repeated runs don't pile up duplicates.
    """
    resp = client.get("/v1/assessments", params={"status": "active"}, headers=auth)
    if not resp.is_success:
        return None
    for a in resp.json() if isinstance(resp.json(), list) else []:
        if a.get("title") == title:
            return a["id"]
    return None


def _create_and_publish_assessment(
    client: httpx.Client, auth: dict, meta: dict, questions: list[dict], result: SeedResult
) -> str | None:
    existing = _find_assessment_by_title(client, auth, meta["title"])
    if existing:
        console.print(f"  Reusing assessment {existing[:8]}…  '{meta['title']}' (already seeded)")
        return existing

    body = {
        "title": meta["title"],
        "description": meta.get("description"),
        "mode": "practice",
        "objectives": meta.get("objectives", []),
        "course": meta.get("course"),
        "durationMin": meta.get("duration"),
        "timeLimitSeconds": None,
        "passingPoints": None,
        "showResultsDuring": False,
        "affectsRating": True,
        "visibility": meta.get("visibility", "private"),
        "method": "manual",
    }
    resp = client.post("/v1/assessments", json=body, headers=auth)
    if not resp.is_success:
        console.print(f"[red]Failed to create assessment '{meta['title']}':[/red] {resp.text[:200]}")
        return None

    assessment_id = resp.json()["id"]
    console.print(f"  Created assessment {assessment_id[:8]}…  '{meta['title']}'")

    added = sum(
        1 for q in questions
        if client.post(
            f"/v1/assessments/{assessment_id}/questions",
            json={"questionId": q["id"]},
            headers=auth,
        ).is_success
    )
    console.print(f"  Linked {added}/{len(questions)} questions")

    pub = client.patch(
        f"/v1/assessments/{assessment_id}",
        json={"status": "active", "visibility": meta.get("visibility", "private")},
        headers=auth,
    )
    if pub.is_success:
        console.print(f"  Published → active  (visibility={meta.get('visibility', 'private')})")
    else:
        console.print(f"  [yellow]Could not publish:[/yellow] {pub.text[:200]}")
    return assessment_id


def seed_quizzes(client: httpx.Client, auth: dict, result: SeedResult) -> None:
    console.rule("[bold]Assessments")
    if not result.questions:
        console.print("  [yellow]No questions — skipping[/yellow]")
        return

    # Main assessment: all question kinds, public
    result.quiz_id = _create_and_publish_assessment(client, auth, QUIZ, result.questions, result)

    # Python assessment: python + math questions only (short + code), public
    python_qs = [q for q in result.questions if q["kind"] in ("short", "code")]
    if python_qs:
        result.quiz_python_id = _create_and_publish_assessment(client, auth, QUIZ_PYTHON, python_qs, result)

    # Draft assessment (private) for Author studio
    draft_body = {
        "title": QUIZ_DRAFT["title"],
        "description": None,
        "mode": "practice",
        "objectives": QUIZ_DRAFT.get("objectives", []),
        "course": QUIZ_DRAFT.get("course"),
        "durationMin": None,
        "timeLimitSeconds": None,
        "passingPoints": None,
        "showResultsDuring": False,
        "affectsRating": True,
        "visibility": "private",
        "method": "manual",
    }
    draft_resp = client.post("/v1/assessments", json=draft_body, headers=auth)
    if draft_resp.is_success:
        console.print(f"  Created private draft assessment: '{QUIZ_DRAFT['title']}'")
    else:
        console.print(f"  [yellow]Failed to create draft assessment:[/yellow] {draft_resp.text[:200]}")


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

    body = {
        **EXAM_BLUEPRINT,
        "mode": "graded",
        "visibility": "public",
        "method": "manual",
    }
    resp = client.post("/v1/assessments", json=body, headers=auth)
    if resp.is_success:
        exam_id = resp.json().get("id")
        result.exam_id = exam_id
        console.print(f"  Created exam assessment {exam_id}")
        
        # Link questions to the default section
        for qid in mc_ids + essay_ids:
            client.post(f"/v1/assessments/{exam_id}/questions", json={"questionId": qid}, headers=auth)
            
        pub = client.patch(f"/v1/assessments/{exam_id}", json={"status": "active"}, headers=auth)
        if pub.is_success:
            console.print("  Published exam → active")
        else:
            console.print(f"  [yellow]Could not publish exam:[/yellow] {pub.status_code} {pub.text[:120]}")
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

    learners = [u for u in result.users if u["email"] == "learner@example.com"]
    enrolled = sum(
        1 for u in learners
        if client.post(f"/v1/cohorts/{cohort_id}/members", json={"userId": u["id"]}, headers=auth).is_success
    )
    console.print(f"  Enrolled {enrolled}/{len(learners)} learners")


def seed_attempts(client: httpx.Client, result: SeedResult) -> None:
    """Have the learner take the main quiz so grading/results/progress pages show data."""
    console.rule("[bold]Attempts")
    if not result.quiz_id:
        console.print("  [yellow]No quiz — skipping attempts[/yellow]")
        return

    learner = next((u for u in result.users if u["email"] == "learner@example.com"), None)
    if not learner:
        console.print("  [yellow]No learner user — skipping attempts[/yellow]")
        return

    auth = {"Authorization": f"Bearer {learner['token']}"}

    # Build question-id → answer lookup from the questions we created (same order as QUESTIONS)
    id_to_answer: dict[str, dict] = {}
    for q, ans in zip(result.questions, QUESTION_ANSWERS):
        id_to_answer[q["id"]] = ans

    # Start a session
    resp = client.post("/v1/sessions", json={"assessmentId": result.quiz_id}, headers=auth)
    if not resp.is_success:
        console.print(f"  [yellow]Could not start session:[/yellow] {resp.status_code} {resp.text[:200]}")
        return

    body = resp.json()
    session_id = (
        body.get("sessionId")
        or body.get("session_id")
        or body.get("session", {}).get("id")
    )
    if not session_id:
        console.print("  [yellow]No session ID in response — skipping[/yellow]")
        return

    # Fetch full session to get option_order per question
    get_resp = client.get(f"/v1/sessions/{session_id}", headers=auth)
    if not get_resp.is_success:
        console.print(f"  [yellow]Could not fetch session: {get_resp.status_code}[/yellow]")
        return

    session_data = get_resp.json()
    questions = session_data.get("questions", [])

    answered = 0
    for q in questions:
        qid = str(q.get("questionId") or q.get("question_id", ""))
        kind = q.get("kind", "")
        option_order: list[int] = q.get("optionOrder") or q.get("option_order") or []
        ans = id_to_answer.get(qid)

        if kind == "mc":
            correct_index = (ans or {}).get("correct_index", 0)
            # Find which shuffled position maps to the correct canonical index.
            # Falls back to position 0 if option_order is unavailable.
            selected_position = next(
                (pos for pos, canon in enumerate(option_order) if canon == correct_index),
                0,
            )
            response = {"selected_position": selected_position}
        elif kind == "tf":
            response = {"answer": (ans or {}).get("answer", True)}
        elif kind == "short":
            response = {"answer": (ans or {}).get("answer", "stack")}
        elif kind == "essay":
            body_text = (ans or {}).get("body", "Sample essay response for grading.")
            response = {"body": body_text, "word_count": len(body_text.split())}
        elif kind == "code":
            source = (ans or {}).get("source", "pass")
            language = (ans or {}).get("language", "python")
            response = {"source": source, "language": language}
        else:
            continue

        answer_resp = client.post(
            f"/v1/sessions/{session_id}/answer",
            json={"questionId": qid, "response": response},
            headers=auth,
        )
        if answer_resp.is_success:
            answered += 1
            result.attempts_seeded += 1
        else:
            console.print(f"  [yellow]Answer failed for {kind} {qid[:8]}:[/yellow] {answer_resp.status_code} {answer_resp.text[:120]}")

    console.print(f"  Submitted {answered}/{len(questions)} answers for learner session {session_id[:8]}…")

    # Finish the session to complete it
    finish_resp = client.post(f"/v1/sessions/{session_id}/finish", headers=auth)
    if finish_resp.is_success:
        console.print(f"  [green]Finished learner session {session_id[:8]}[/green]")
    else:
        console.print(f"  [yellow]Failed to finish session {session_id[:8]}:[/yellow] {finish_resp.status_code}")


def seed_agents(client: httpx.Client, result: SeedResult) -> None:
    console.rule("[bold]Agents")
    admin = next((u for u in result.users if u["email"] == "admin@example.com"), None)
    if not admin:
        console.print("  [yellow]No admin user — skipping agents[/yellow]")
        return
    admin_auth = {"Authorization": f"Bearer {admin['token']}"}

    # 1. Upgrade learner and instructor to premium
    for u in result.users:
        if u["role"] == "user":
            r = client.patch(f"/v1/admin/users/{u['id']}", json={"plan": "premium"}, headers=admin_auth)
            if r.is_success:
                console.print(f"  Upgraded {u['email']} to PREMIUM")
            else:
                console.print(f"  [yellow]Failed to upgrade {u['email']}:[/yellow] {r.status_code}")

    # 2. Create agent for learner
    learner = next((u for u in result.users if u["email"] == "learner@example.com"), None)
    if learner:
        learner_auth = {"Authorization": f"Bearer {learner['token']}"}
        agent_resp = client.post(
            "/v1/me/agents",
            json={
                "label": "Alice's Study Assistant",
                "scopes": ["assessment.read", "assessment.write", "attempt.read", "attempt.write", "stats.read", "plan.read", "plan.write"],
                "focusTags": ["algorithms", "python"],
            },
            headers=learner_auth,
        )
        if agent_resp.is_success:
            agent_data = agent_resp.json()
            console.print(f"  [green]✓[/green] Created agent for Learner: [cyan]{agent_data.get('id')}[/cyan]")
            console.print(f"    Key: {agent_data.get('secret')}")
        else:
            console.print(f"  [yellow]Failed to create learner agent:[/yellow] {agent_resp.status_code} {agent_resp.text}")

    # 3. Create agent for instructor
    instructor = next((u for u in result.users if u["email"] == "instructor@example.com"), None)
    if instructor:
        instructor_auth = {"Authorization": f"Bearer {instructor['token']}"}
        agent_resp = client.post(
            "/v1/me/agents",
            json={
                "label": "Bob's Content Generator",
                "scopes": ["assessment.read", "assessment.write", "attempt.read", "attempt.write", "stats.read", "plan.read", "plan.write", "public.publish"],
                "focusTags": ["data-structures", "sorting"],
            },
            headers=instructor_auth,
        )
        if agent_resp.is_success:
            agent_data = agent_resp.json()
            console.print(f"  [green]✓[/green] Created agent for Instructor: [cyan]{agent_data.get('id')}[/cyan]")
            console.print(f"    Key: {agent_data.get('secret')}")
        else:
            console.print(f"  [yellow]Failed to create instructor agent:[/yellow] {agent_resp.status_code} {agent_resp.text}")


def print_summary(result: SeedResult) -> None:
    console.rule("[bold green]Seed complete")
    table = Table(show_header=True, header_style="bold")
    table.add_column("Resource")
    table.add_column("Count / ID")
    table.add_row("Users", str(len(result.users)))
    table.add_row("Tags", str(len(result.tags)))
    table.add_row("Questions (live)", str(len(result.questions)))
    table.add_row("Quiz (main)", result.quiz_id or "—")
    table.add_row("Quiz (python)", result.quiz_python_id or "—")
    table.add_row("Exam", result.exam_id or "—")
    table.add_row("Cohort", result.cohort_id or "—")
    table.add_row("Attempts seeded", str(result.attempts_seeded))
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
        health = client.get("/healthz")
        if not health.is_success:
            console.print(f"[red]API not reachable at {args.api}[/red]")
            sys.exit(1)

        seed_users(client, result)

        instructor = next((u for u in result.users if u["email"] == "instructor@example.com"), None)
        if not instructor:
            console.print("[red]No instructor user — cannot seed content[/red]")
            sys.exit(1)
        auth = {"Authorization": f"Bearer {instructor['token']}"}

        admin = next((u for u in result.users if u["role"] == "admin"), None)
        admin_auth = {"Authorization": f"Bearer {admin['token']}"} if admin else auth

        seed_tags(client, auth, result)
        seed_questions(client, auth, result)
        seed_quizzes(client, auth, result)
        seed_exam(client, auth, result)
        seed_cohort(client, admin_auth, result)
        seed_attempts(client, result)
        seed_agents(client, result)

    print_summary(result)


if __name__ == "__main__":
    main()
