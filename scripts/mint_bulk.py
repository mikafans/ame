# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27", "rich>=13"]
# ///
"""
Mint 100k questions and 10k exams using the agent surface tools.
Uses the optimized 'assessment.create' with nested questions for maximum speed.

Usage:
    uv run scripts/mint_bulk.py
    uv run scripts/mint_bulk.py --api http://localhost:28080 --exams 10000 --q-per-exam 10
"""

import argparse
import os
import random
import sys
import time
from typing import Any

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
                except:
                    pass
                time.sleep(max(0.2, retry_after))
                continue
            return resp
        except httpx.RequestError as e:
            console.print(f"[yellow]Request error, retrying in 2s: {e}[/yellow]")
            time.sleep(2)
            continue

def main():
    parser = argparse.ArgumentParser(description="Bulk mint questions and exams via agent surface.")
    parser.add_argument("--api", default=DEFAULT_API)
    parser.add_argument("--exams", type=int, default=10000)
    parser.add_argument("--q-per-exam", type=int, default=10)
    args = parser.parse_args()

    try:
        with httpx.Client(base_url=args.api, timeout=120.0) as client:
            # 1. Login as primary user
            console.print("[bold cyan]Logging in as primary user...[/bold cyan]")
            login_resp = request_with_retry(client, "POST", "/v1/auth/login", json={
                "email": "ada@example.com",
                "password": "password123"
            })
            if not login_resp.is_success:
                console.print("[red]Login failed. Make sure 'make db-seed' was run.[/red]")
                sys.exit(1)
            
            primary_token = login_resp.json()["token"]
            headers = {"Authorization": f"Bearer {primary_token}"}
            
            # 2. Get Agent
            console.print("[bold cyan]Creating/Getting agent key...[/bold cyan]")
            agent_resp = request_with_retry(client, "POST", "/v1/me/agents", json={
                "label": f"Mega Minter {random.randint(1000, 9999)}",
                "scopes": ["assessment.read", "assessment.write"],
                "focusTags": TOPICS[:10]
            }, headers=headers)
            
            if not agent_resp.is_success:
                console.print(f"[red]Failed to create agent: {agent_resp.text}[/red]")
                sys.exit(1)
                
            agent_data = agent_resp.json()
            agent_key = agent_data["apiKey"]
            agent_headers = {"Authorization": f"Bearer {agent_key}"}
            
            console.print(f"Agent ready: [green]{agent_data['id']}[/green]")

            # 3. Mega Mint using nested questions
            with Progress(
                SpinnerColumn(),
                TextColumn("[progress.description]{task.description}"),
                BarColumn(),
                TaskProgressColumn(),
                TimeRemainingColumn(),
                console=console
            ) as progress:
                task = progress.add_task("[cyan]Mega-minting (Exams + Questions)...", total=args.exams)
                
                for i in range(args.exams):
                    topic = random.choice(TOPICS)
                    qs = [make_question(i * args.q_per_exam + k) for k in range(args.q_per_exam)]
                    
                    # Alternate between practice and graded
                    mode = "practice" if i % 2 == 0 else "graded"
                    title_prefix = "Knowledge Check" if mode == "practice" else "Mastery Exam"
                    
                    # Create assessment WITH nested questions
                    resp = request_with_retry(client, "POST", "/v1/agents/run", json={
                        "tool": "assessment.create",
                        "params": {
                            "title": f"{title_prefix} {i}: {topic.capitalize()}",
                            "mode": mode,
                            "objectives": [f"Master {topic}", f"Understand {random.choice(TOPICS)}"],
                            "course": "Mega Scale 2026",
                            "method": "agent",
                            "questions": qs
                        }
                    }, headers=agent_headers)
                    
                    if not resp.is_success or not resp.json().get("ok"):
                        progress.console.print(f"[red]Batch {i} failed: {resp.text}[/red]")
                        continue
                    
                    exam_id = resp.json()["result"]["id"]
                    
                    # Occasionally publish some (not all, to keep some as drafts)
                    if i % 2 == 0:
                        request_with_retry(client, "POST", "/v1/agents/run", json={
                            "tool": "assessment.update",
                            "params": {
                                "id": exam_id,
                                "status": "active"
                            }
                        }, headers=agent_headers)
                    
                    progress.update(task, advance=1)

    except KeyboardInterrupt:
        console.print("\n[yellow]Interrupted by user. Exiting cleanly...[/yellow]")
        sys.exit(0)

    console.print("\n[bold green]Mega-minting complete![/bold green]")
    console.print(f"Total exams target: {args.exams}")
    console.print(f"Total questions target: {args.exams * args.q_per_exam}")

if __name__ == "__main__":
    main()
