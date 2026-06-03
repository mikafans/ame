# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27", "rich>=13"]
# ///
"""
Bulk-mint assessments and exams using the agent surface tools.
Uses the optimized 'assessment.create' with nested questions for speed.

By default it mints 10k practice assessments + 1k graded exams (5-10 questions
each), split evenly across the seeded users (Ada + Mira), via each user's agent.

Usage:
    uv run scripts/mint_bulk.py
    uv run scripts/mint_bulk.py --api http://localhost:28080 --assessments 10000 --exams 1000
    uv run scripts/mint_bulk.py --users ada@example.com --assessments 200 --exams 20
"""

import argparse
import os
import random
import sys
import time

import httpx
from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn, TaskProgressColumn, TimeRemainingColumn

console = Console()

DEFAULT_API = os.environ.get("AME_API_URL", "http://localhost:28080")

TOPICS = [
    "rust", "zig", "postgres", "axum", "tokio", "docker", "linux",
    "networking", "tls", "ownership", "lifetimes", "async", "borrow", "index",
    "query", "schema", "migration", "pod", "deployment", "service",
    "algorithms", "data-structures", "sorting", "graphs", "dp", "math"
]

def make_question(i):
    topic = random.choice(TOPICS)
    secondary = random.choice([t for t in TOPICS if t != topic])
    kinds = ["mc", "tf", "short"]
    kind = random.choice(kinds)
    prompt = f"[{i}] Optimized bulk question about {topic} and {secondary}."

    if kind == "mc":
        payload = {"correct_index": 1, "options": ["alpha", "beta", "gamma", "delta"]}
    elif kind == "tf":
        payload = {"correct": random.choice([True, False])}
    else:
        payload = {"accepted": [topic], "judge": "exact", "normalize": "exact"}

    return {
        "kind": kind,
        "prompt": prompt,
        "payload": payload,
        "points": 1,
        "tags": [topic, secondary]
    }

def request_with_retry(client, method, url, **kwargs):
    while True:
        try:
            resp = client.request(method, url, **kwargs)
            if resp.status_code == 429:
                retry_after = 1.0
                try:
                    h = resp.headers.get("Retry-After")
                    if h:
                        retry_after = float(h)
                except Exception:
                    pass
                time.sleep(max(0.2, retry_after))
                continue
            return resp
        except httpx.RequestError as e:
            console.print(f"[yellow]Request error, retrying in 2s: {e}[/yellow]")
            time.sleep(2)
            continue

def mint_for_user(client, email, password, n_assessments, n_exams, q_min, q_max, progress):
    """Mint n_assessments practice + n_exams graded items for one user via their agent.

    Returns the number of items successfully created.
    """
    login_resp = request_with_retry(client, "POST", "/v1/auth/login", json={
        "email": email,
        "password": password,
    })
    if not login_resp.is_success:
        console.print(f"[red]Login failed for {email}. Make sure 'make db-seed' was run.[/red]")
        return 0
    headers = {"Authorization": f"Bearer {login_resp.json()['token']}"}

    def create_agent():
        return request_with_retry(client, "POST", "/v1/me/agents", json={
            "label": f"Mega Minter {random.randint(1000, 9999)}",
            "scopes": ["assessment.read", "assessment.write"],
            "focusTags": TOPICS[:10],
        }, headers=headers)

    # If the per-account agent quota is exhausted (429), free existing agents and
    # retry once so reruns stay idempotent.
    agent_resp = create_agent()
    if agent_resp.status_code == 429:
        listed = client.get("/v1/me/agents", headers=headers)
        if listed.is_success:
            for a in listed.json().get("agents", []):
                client.delete(f"/v1/me/agents/{a['id']}", headers=headers)
        agent_resp = create_agent()
    if not agent_resp.is_success:
        console.print(f"[red]Failed to create agent for {email}: {agent_resp.text}[/red]")
        return 0
    agent_headers = {"Authorization": f"Bearer {agent_resp.json()['apiKey']}"}

    task = progress.add_task(f"[cyan]{email}", total=n_assessments + n_exams)
    minted = 0
    seq = 0
    batch_size = 200

    for mode, title_prefix, count in (
        ("practice", "Knowledge Check", n_assessments),
        ("graded", "Mastery Exam", n_exams),
    ):
        for batch_start in range(0, count, batch_size):
            batch_end = min(batch_start + batch_size, count)
            batch_items = []

            for i in range(batch_start, batch_end):
                topic = random.choice(TOPICS)
                qs = [make_question(seq * q_max + k) for k in range(random.randint(q_min, q_max))]
                seq += 1

                # Alternate status: draft and active (50/50 mix)
                status = "active" if i % 2 == 0 else "draft"

                batch_items.append({
                    "title": f"{title_prefix} {i}: {topic.capitalize()}",
                    "mode": mode,
                    "objectives": [f"Master {topic}", f"Understand {random.choice(TOPICS)}"],
                    "course": "Mega Scale 2026",
                    "method": "agent",
                    "status": status,
                    "questions": qs,
                })

            resp = request_with_retry(client, "POST", "/v1/agents/run", json={
                "tool": "assessment.batchCreate",
                "params": {
                    "items": batch_items,
                },
            }, headers=agent_headers)

            if not resp.is_success or not resp.json().get("ok"):
                progress.console.print(f"[red]{email} batch ({mode} {batch_start}-{batch_end}) failed: {resp.text}[/red]")
                progress.update(task, advance=batch_end - batch_start)
                continue

            batch_count = resp.json()["result"].get("count", 0)
            minted += batch_count
            progress.update(task, advance=batch_end - batch_start)

    return minted

def main():
    parser = argparse.ArgumentParser(description="Bulk mint assessments and exams via agent surface.")
    parser.add_argument("--api", default=DEFAULT_API)
    parser.add_argument("--assessments", type=int, default=10000, help="Total practice assessments (split across users)")
    parser.add_argument("--exams", type=int, default=1000, help="Total graded exams (split across users)")
    parser.add_argument("--q-min", type=int, default=5, help="Min questions per item")
    parser.add_argument("--q-max", type=int, default=10, help="Max questions per item")
    parser.add_argument("--users", default="ada@example.com,mira@example.com",
                        help="Comma-separated user emails to split the totals across")
    parser.add_argument("--password", default="password123")
    args = parser.parse_args()

    users = [u.strip() for u in args.users.split(",") if u.strip()]
    if not users:
        console.print("[red]No users provided.[/red]")
        sys.exit(1)
    if args.q_min < 1 or args.q_max < args.q_min:
        console.print("[red]Invalid --q-min/--q-max range.[/red]")
        sys.exit(1)

    # Divide the totals across users; earlier users absorb any remainder.
    n = len(users)
    base_a, rem_a = divmod(args.assessments, n)
    base_e, rem_e = divmod(args.exams, n)

    total_minted = 0
    try:
        with httpx.Client(base_url=args.api, timeout=120.0) as client:
            with Progress(
                SpinnerColumn(),
                TextColumn("[progress.description]{task.description}"),
                BarColumn(),
                TaskProgressColumn(),
                TimeRemainingColumn(),
                console=console,
            ) as progress:
                for idx, email in enumerate(users):
                    a = base_a + (1 if idx < rem_a else 0)
                    e = base_e + (1 if idx < rem_e else 0)
                    console.print(f"[bold cyan]Minting for {email}: {a} assessments + {e} exams[/bold cyan]")
                    total_minted += mint_for_user(
                        client, email, args.password, a, e, args.q_min, args.q_max, progress,
                    )
    except KeyboardInterrupt:
        console.print("\n[yellow]Interrupted by user. Exiting cleanly...[/yellow]")
        sys.exit(0)

    console.print("\n[bold green]Mega-minting complete![/bold green]")
    console.print(f"Users: {', '.join(users)}")
    console.print(f"Total items minted: {total_minted} (target {args.assessments + args.exams})")

if __name__ == "__main__":
    main()
