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
import sys
from dataclasses import dataclass, field

import httpx
from rich.console import Console
from rich.table import Table

console = Console()

DEFAULT_API = "http://localhost:8080"

# ---------------------------------------------------------------------------
# Users
# ---------------------------------------------------------------------------

USERS = [
    {"email": "learner@example.com", "name": "Alice Learner", "password": "password123", "role": "learner"},
    {"email": "instructor@example.com", "name": "Bob Instructor", "password": "password123", "role": "instructor"},
    {"email": "admin@example.com", "name": "Carol Admin", "password": "password123", "role": "admin"},
]

# ---------------------------------------------------------------------------
# Tags
# ---------------------------------------------------------------------------

TAGS = [
    {"name": "algorithms", "description": "Algorithm design and analysis"},
    {"name": "data-structures", "description": "Arrays, trees, graphs, etc."},
    {"name": "sorting", "description": "Sorting algorithms and complexity"},
    {"name": "graphs", "description": "Graph traversal and shortest paths"},
    {"name": "dynamic-programming", "description": "Optimal substructure, memoization"},
    {"name": "math", "description": "Mathematics and discrete math"},
    {"name": "python", "description": "Python language and stdlib"},
    {"name": "rust", "description": "Rust language and ecosystem"},
    {"name": "ownership", "description": "Rust ownership, borrowing, lifetimes"},
    {"name": "concurrency", "description": "Concurrent and parallel programming"},
    {"name": "zig", "description": "Zig language and ecosystem"},
    {"name": "systems", "description": "Systems programming concepts"},
    {"name": "memory", "description": "Memory management and layout"},
    {"name": "os", "description": "Operating systems concepts"},
    {"name": "networking", "description": "Networking and protocols"},
    {"name": "complexity", "description": "Time and space complexity"},
]

# ---------------------------------------------------------------------------
# Questions — grouped by quiz
# ---------------------------------------------------------------------------

QUESTIONS_ALGO = [
    {
        "kind": "mc",
        "prompt": "What is the time complexity of binary search on a sorted array?",
        "payload": {"options": ["O(n)", "O(log n)", "O(n log n)", "O(1)"], "correct_index": 1},
        "explanation": "Binary search halves the search space each step: O(log n).",
        "tags": ["algorithms", "complexity"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which data structure is used to implement breadth-first search?",
        "payload": {"options": ["Stack", "Queue", "Heap", "Hash map"], "correct_index": 1},
        "explanation": "BFS uses a FIFO queue to visit nodes level by level.",
        "tags": ["graphs", "data-structures"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What is the worst-case time complexity of quicksort?",
        "payload": {"options": ["O(n log n)", "O(n)", "O(n²)", "O(log n)"], "correct_index": 2},
        "explanation": "Quicksort degrades to O(n²) when the pivot is always the min or max.",
        "tags": ["sorting", "complexity"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "A binary search tree guarantees O(log n) search time in all cases.",
        "payload": {"correct": False},
        "explanation": "An unbalanced BST degrades to O(n) — e.g., inserting a sorted sequence.",
        "tags": ["data-structures", "complexity"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "Merge sort is a stable sorting algorithm.",
        "payload": {"correct": True},
        "explanation": "Merge sort preserves relative order of equal elements.",
        "tags": ["sorting"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which property ensures a problem can be solved optimally by combining solutions to subproblems?",
        "payload": {"options": ["Greedy choice", "Optimal substructure", "Divide and conquer", "Memoization"], "correct_index": 1},
        "explanation": "Optimal substructure is the key property enabling dynamic programming.",
        "tags": ["dynamic-programming", "algorithms"],
        "points": 2,
    },
    {
        "kind": "short",
        "prompt": "Name the data structure that follows Last-In, First-Out (LIFO) ordering.",
        "payload": {"accepted": ["stack", "Stack", "STACK"], "normalize": "case_insensitive_strip_accents", "judge": "exact"},
        "explanation": "A stack processes the most recently added item first.",
        "tags": ["data-structures"],
        "points": 1,
    },
    {
        "kind": "essay",
        "prompt": "Compare depth-first search and breadth-first search. When would you choose each?",
        "payload": {"min_words": 60, "rubric": "Traversal description (2), complexity comparison (2), use-case examples (2).", "judge": "manual"},
        "explanation": "DFS uses O(depth) space; BFS uses O(width) space and finds shortest paths in unweighted graphs.",
        "tags": ["graphs", "algorithms"],
        "points": 6,
    },
]

QUESTIONS_RUST = [
    {
        "kind": "mc",
        "prompt": "What does Rust's ownership system eliminate at compile time?",
        "payload": {"options": ["Stack overflows", "Data races and use-after-free", "Integer overflow", "Deadlocks"], "correct_index": 1},
        "explanation": "Rust's ownership system statically prevents data races and use-after-free without a GC.",
        "tags": ["rust", "ownership", "memory"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "How many immutable references to a value can coexist at the same time in Rust?",
        "payload": {"options": ["Only one", "Exactly two", "Any number", "Zero — you must use a mutex"], "correct_index": 2},
        "explanation": "Rust allows any number of simultaneous immutable (&T) references, but only one mutable (&mut T).",
        "tags": ["rust", "ownership"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "In Rust, a value can have multiple mutable references at the same time.",
        "payload": {"correct": False},
        "explanation": "Rust enforces exclusive access: at most one &mut T reference at a time.",
        "tags": ["rust", "ownership"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which Rust type represents an owned, heap-allocated string?",
        "payload": {"options": ["&str", "str", "String", "Box<str>"], "correct_index": 2},
        "explanation": "String is the growable, heap-allocated string type. &str is a borrowed string slice.",
        "tags": ["rust"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What does the ? operator do in a function returning Result?",
        "payload": {"options": ["Panics on error", "Returns None", "Propagates the error to the caller", "Logs the error"], "correct_index": 2},
        "explanation": "? unwraps Ok values and returns Err early, propagating the error up the call stack.",
        "tags": ["rust"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "Rust's async functions return a Future that runs on a background thread immediately.",
        "payload": {"correct": False},
        "explanation": "Rust Futures are lazy — they do nothing until polled by an executor.",
        "tags": ["rust", "concurrency"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which trait must a type implement to be sent across thread boundaries in Rust?",
        "payload": {"options": ["Copy", "Clone", "Send", "Sync"], "correct_index": 2},
        "explanation": "Send marks types safe to transfer ownership across threads. Sync marks types safe to share references.",
        "tags": ["rust", "concurrency", "ownership"],
        "points": 2,
    },
    {
        "kind": "short",
        "prompt": "What keyword is used to define a Rust trait?",
        "payload": {"accepted": ["trait"], "normalize": "exact", "judge": "exact"},
        "explanation": "The trait keyword defines a shared interface, similar to interfaces in other languages.",
        "tags": ["rust"],
        "points": 1,
    },
    {
        "kind": "essay",
        "prompt": "Explain Rust's ownership model and how it differs from garbage collection. Give an example of a bug it prevents.",
        "payload": {"min_words": 80, "rubric": "Ownership explanation (2), GC comparison (2), concrete bug example (2).", "judge": "manual"},
        "tags": ["rust", "ownership", "memory"],
        "points": 6,
    },
]

QUESTIONS_ZIG = [
    {
        "kind": "mc",
        "prompt": "How does Zig handle memory allocation differently from C?",
        "payload": {"options": [
            "Zig uses a garbage collector",
            "Zig requires an explicit allocator to be passed to functions that allocate",
            "Zig allocates all memory on the stack",
            "Zig forbids heap allocation entirely",
        ], "correct_index": 1},
        "explanation": "Zig requires explicit allocator arguments, making allocation visible and testable.",
        "tags": ["zig", "memory", "systems"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What is comptime in Zig?",
        "payload": {"options": [
            "A runtime optimization hint",
            "A way to mark functions as async",
            "Compile-time code execution for generics and meta-programming",
            "A debugging mode",
        ], "correct_index": 2},
        "explanation": "comptime allows arbitrary code to run at compile time, enabling generics without templates.",
        "tags": ["zig"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "Zig has hidden control flow such as exceptions or operator overloading.",
        "payload": {"correct": False},
        "explanation": "Zig explicitly avoids hidden control flow — no exceptions, no operator overloading, no implicit allocations.",
        "tags": ["zig", "systems"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which Zig type is used to represent a value that may or may not be present?",
        "payload": {"options": ["Result", "Option", "?T (optional)", "Maybe"], "correct_index": 2},
        "explanation": "Zig uses ?T for optionals — null is only allowed for optional types.",
        "tags": ["zig"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What does defer do in Zig?",
        "payload": {"options": [
            "Delays a function until the next event loop tick",
            "Runs a statement when the current scope exits",
            "Marks a function as asynchronous",
            "Allocates memory that is freed at program exit",
        ], "correct_index": 1},
        "explanation": "defer schedules a statement to run at the end of the enclosing scope — useful for cleanup.",
        "tags": ["zig", "systems"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "Zig can call C functions directly without any FFI layer or wrapper.",
        "payload": {"correct": True},
        "explanation": "Zig has first-class C interop — you can @cImport and call C functions directly.",
        "tags": ["zig", "systems"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "Which Zig error handling mechanism propagates errors without exceptions?",
        "payload": {"options": ["try/catch blocks", "error unions with try and catch", "error unions with try (propagates) and catch (handles)", "panic()"], "correct_index": 2},
        "explanation": "Zig error unions (T!E) combined with try propagate errors up the call stack explicitly.",
        "tags": ["zig"],
        "points": 2,
    },
    {
        "kind": "essay",
        "prompt": "Compare Zig's approach to error handling with Rust's Result type. What are the trade-offs?",
        "payload": {"min_words": 70, "rubric": "Zig error unions (2), Rust Result (2), trade-off analysis (2).", "judge": "manual"},
        "tags": ["zig", "rust", "systems"],
        "points": 6,
    },
]

QUESTIONS_CS = [
    {
        "kind": "mc",
        "prompt": "What does a process context switch involve?",
        "payload": {"options": [
            "Only saving the program counter",
            "Saving CPU registers, stack pointer, and page table reference for the current process",
            "Flushing the disk cache",
            "Deallocating all heap memory",
        ], "correct_index": 1},
        "explanation": "A context switch saves the full CPU state of the current process and restores the next one.",
        "tags": ["os", "systems"],
        "points": 2,
    },
    {
        "kind": "tf",
        "prompt": "TCP guarantees that data arrives in the same order it was sent.",
        "payload": {"correct": True},
        "explanation": "TCP is a reliable, ordered byte stream — it reorders segments and retransmits as needed.",
        "tags": ["networking"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What is a TLB in the context of virtual memory?",
        "payload": {"options": [
            "A type of disk cache",
            "A hardware cache that speeds up virtual-to-physical address translation",
            "A network routing table",
            "A thread-local buffer",
        ], "correct_index": 1},
        "explanation": "The Translation Lookaside Buffer caches recent page table entries to avoid expensive memory lookups.",
        "tags": ["os", "memory"],
        "points": 2,
    },
    {
        "kind": "mc",
        "prompt": "Which HTTP status code indicates that a resource has permanently moved to a new URL?",
        "payload": {"options": ["200", "301", "404", "500"], "correct_index": 1},
        "explanation": "301 Moved Permanently tells clients and search engines the resource has a new permanent URL.",
        "tags": ["networking"],
        "points": 1,
    },
    {
        "kind": "tf",
        "prompt": "A mutex and a semaphore are functionally identical.",
        "payload": {"correct": False},
        "explanation": "A mutex is a binary lock owned by a thread. A semaphore is a counter that need not be owned.",
        "tags": ["concurrency", "os"],
        "points": 1,
    },
    {
        "kind": "mc",
        "prompt": "What does the CAP theorem state?",
        "payload": {"options": [
            "A distributed system can guarantee all of consistency, availability, and partition tolerance",
            "A distributed system can guarantee at most two of consistency, availability, and partition tolerance",
            "Partition tolerance can always be sacrificed for consistency",
            "Availability is always more important than consistency",
        ], "correct_index": 1},
        "explanation": "CAP: in the presence of a network partition, you must choose between consistency and availability.",
        "tags": ["systems", "networking"],
        "points": 2,
    },
    {
        "kind": "short",
        "prompt": "What does POSIX stand for?",
        "payload": {"accepted": ["Portable Operating System Interface", "portable operating system interface"], "normalize": "case_insensitive_strip_accents", "judge": "exact"},
        "explanation": "POSIX defines a standard API for Unix-compatible operating systems.",
        "tags": ["os", "systems"],
        "points": 1,
    },
    {
        "kind": "essay",
        "prompt": "Explain the difference between a process and a thread. When would you use one over the other?",
        "payload": {"min_words": 70, "rubric": "Process definition (2), thread definition (2), comparison with examples (2).", "judge": "manual"},
        "tags": ["os", "concurrency"],
        "points": 6,
    },
]

# Quiz definitions: title, objectives, question list key
QUIZZES = [
    {
        "title": "Algorithms & Data Structures — Fundamentals",
        "objectives": [
            "Understand time complexity of common algorithms",
            "Distinguish between core data structures",
            "Apply sorting and graph algorithms",
        ],
        "questions": QUESTIONS_ALGO,
    },
    {
        "title": "Rust Essentials",
        "objectives": [
            "Understand Rust's ownership and borrowing model",
            "Use Result and Option for error handling",
            "Reason about Rust concurrency primitives",
        ],
        "questions": QUESTIONS_RUST,
    },
    {
        "title": "Zig Fundamentals",
        "objectives": [
            "Understand Zig's comptime and allocator model",
            "Use Zig error unions and defer",
            "Compare Zig with C and Rust",
        ],
        "questions": QUESTIONS_ZIG,
    },
    {
        "title": "General CS — Systems & Networking",
        "objectives": [
            "Understand OS concepts: processes, threads, virtual memory",
            "Reason about networking protocols",
            "Apply concurrency primitives correctly",
        ],
        "questions": QUESTIONS_CS,
    },
]

EXAM = {
    "name": "Systems & Languages Midterm",
    "description": "Covers Rust, Zig, algorithms, OS, and networking fundamentals.",
    "duration": 90,
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

@dataclass
class SeedResult:
    users: list[dict] = field(default_factory=list)
    tags: list[dict] = field(default_factory=list)
    quizzes: list[dict] = field(default_factory=list)
    exam_id: str | None = None


def post(client: httpx.Client, path: str, body: dict, auth: dict | None = None) -> dict:
    resp = client.post(path, json=body, headers=auth or {})
    if not resp.is_success:
        console.print(f"[red]FAIL[/red] POST {path} → {resp.status_code}: {resp.text[:300]}")
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


def seed_tags(client: httpx.Client, auth: dict) -> None:
    console.rule("[bold]Tags")
    for tag in TAGS:
        resp = client.post("/v1/tags", json=tag, headers=auth)
        status = "[green]✓[/green]" if resp.status_code in (200, 201) else "[yellow]skip[/yellow]"
        console.print(f"  {status} #{tag['name']}")


def seed_quiz(client: httpx.Client, auth: dict, quiz_def: dict, result: SeedResult) -> None:
    # Create questions
    resp = client.post("/v1/questions", json={"questions": quiz_def["questions"]}, headers=auth)
    if resp.status_code != 201:
        console.print(f"  [red]Failed to create questions:[/red] {resp.text[:300]}")
        return
    created_qs = resp.json()["questions"]

    # Promote to live
    for q in created_qs:
        client.post(f"/v1/questions/{q['id']}/promote", headers=auth)

    # Create quiz
    resp2 = client.post("/v1/quizzes", json={
        "title": quiz_def["title"],
        "objectives": quiz_def["objectives"],
    }, headers=auth)
    if not resp2.is_success:
        console.print(f"  [red]Failed to create quiz:[/red] {resp2.text[:200]}")
        return
    quiz_id = resp2.json()["quiz"]["id"]

    # Link questions
    added = 0
    for q in created_qs:
        r = client.post(f"/v1/quizzes/{quiz_id}/questions", json={"questionId": q["id"]}, headers=auth)
        if r.is_success:
            added += 1

    # Publish
    client.patch(f"/v1/quizzes/{quiz_id}", json={"status": "active"}, headers=auth)

    result.quizzes.append({"id": quiz_id, "title": quiz_def["title"], "questions": created_qs})
    console.print(f"  [green]✓[/green] {quiz_def['title']} — {len(created_qs)} questions, {added} linked")


def seed_exam(client: httpx.Client, auth: dict, result: SeedResult) -> None:
    console.rule("[bold]Exam")
    if len(result.quizzes) < 2:
        console.print("  [yellow]Need at least 2 quizzes — skipping exam[/yellow]")
        return

    # Build one section per quiz, picking up to 10 MC/TF questions per section
    sections = []
    total = 0
    for quiz in result.quizzes:
        mc_tf_ids = [
            q["id"] for q in quiz["questions"]
            if q["kind"] in ("mc", "tf") and total < 30
        ][:3]  # up to 3 per quiz section → ~10-12 total across 4 quizzes
        if not mc_tf_ids:
            continue
        sections.append({
            "title": quiz["title"],
            "questionIds": mc_tf_ids,
            "weight": 1.0,
        })
        total += len(mc_tf_ids)

    if not sections:
        console.print("  [yellow]No MC/TF questions found — skipping exam[/yellow]")
        return

    resp = client.post("/v1/exams", json={**EXAM, "sections": sections}, headers=auth)
    if resp.is_success:
        body = resp.json()
        exam_id = body.get("id") or (body.get("exam") or {}).get("id")
        result.exam_id = exam_id
        console.print(f"  [green]✓[/green] {EXAM['name']} — {len(sections)} sections, {total} questions")
    else:
        console.print(f"  [yellow]Could not create exam:[/yellow] {resp.status_code} {resp.text[:200]}")


def print_summary(result: SeedResult) -> None:
    console.rule("[bold green]Seed complete")
    table = Table(show_header=True, header_style="bold")
    table.add_column("Resource")
    table.add_column("Count / ID")
    table.add_row("Users", str(len(result.users)))
    table.add_row("Tags", str(len(TAGS)))
    table.add_row("Quizzes", str(len(result.quizzes)))
    table.add_row("Exam", result.exam_id or "—")
    console.print(table)

    console.rule("[bold]Credentials")
    for u in result.users:
        console.print(f"  [cyan]{u['role']:12}[/cyan]  {u['email']}  /  {u['password']}")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Seed demo data into a running ame API.")
    parser.add_argument("--api", default=DEFAULT_API)
    args = parser.parse_args()

    console.print(f"\n[bold]ame seed[/bold] → {args.api}\n")
    result = SeedResult()

    with httpx.Client(base_url=args.api, timeout=15.0) as client:
        if not client.get("/healthz").is_success:
            console.print(f"[red]API not reachable at {args.api}[/red]")
            sys.exit(1)

        seed_users(client, result)

        instructor = next((u for u in result.users if u["role"] == "instructor"), None)
        if not instructor:
            console.print("[red]No instructor user — cannot seed content[/red]")
            sys.exit(1)

        auth = {"Authorization": f"Bearer {instructor['token']}"}

        seed_tags(client, auth)

        console.rule("[bold]Quizzes")
        for quiz_def in QUIZZES:
            seed_quiz(client, auth, quiz_def, result)

        seed_exam(client, auth, result)

    print_summary(result)


if __name__ == "__main__":
    main()
