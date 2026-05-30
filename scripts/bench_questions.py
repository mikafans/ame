#!/usr/bin/env python3
"""Mint questions through the API and benchmark GET /v1/questions at scale.

Dependency-free (stdlib only). Exercises the real app code path — batch
POST /v1/questions (and some single POSTs) — rather than raw SQL, so the
benchmark reflects the API's own query design.

Usage:
    python3 scripts/bench_questions.py --total 1000000          # mint then bench
    python3 scripts/bench_questions.py --mode bench             # bench only
    python3 scripts/bench_questions.py --total 200000 --batch 50

Env:
    AME_API_URL   base URL (default http://localhost:28080)
"""

import argparse
import json
import os
import random
import time
import urllib.error
import urllib.request

DEFAULT_API = os.environ.get("AME_API_URL", "http://localhost:28080")

# A small vocabulary so full-text search has realistic, varied matches.
TOPICS = [
    "kubernetes", "rust", "postgres", "axum", "tokio", "docker", "linux",
    "networking", "tls", "ownership", "lifetimes", "async", "borrow", "index",
    "query", "schema", "migration", "pod", "deployment", "service",
]
ADJECTIVES = ["basic", "advanced", "tricky", "core", "edge-case", "common"]


def req(method, url, token=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(url, data=data, method=method)
    r.add_header("content-type", "application/json")
    if token:
        r.add_header("authorization", f"Bearer {token}")
    with urllib.request.urlopen(r) as resp:
        return resp.status, json.loads(resp.read() or "null")


def login(base, email, password):
    _, body = req("POST", f"{base}/v1/auth/login", body={"email": email, "password": password})
    return body["token"]


def make_question(i):
    """Generate one question; kind rotates so all graders are exercised."""
    topic = random.choice(TOPICS)
    adj = random.choice(ADJECTIVES)
    kinds = ["mc", "tf", "short"]
    kind = kinds[i % len(kinds)]
    prompt = f"[{i}] A {adj} question about {topic} and {random.choice(TOPICS)}."
    if kind == "mc":
        payload = {"correct_index": 1, "options": ["alpha", "beta", "gamma", "delta"]}
    elif kind == "tf":
        payload = {"correct": i % 2 == 0}
    else:
        payload = {"accepted": [topic], "judge": "exact", "normalize": "exact"}
    q = {"kind": kind, "prompt": prompt, "payload": payload, "points": 1}
    # Tag ~half the rows so tag joins are non-trivial at scale.
    if i % 2 == 0:
        q["tags"] = [topic]
    return q


def mint(base, token, total, batch, single_count):
    print(f"minting {total} questions (batch={batch}, singles={single_count}) ...", flush=True)
    start = time.time()
    made = 0
    idx = 0
    last_report = start

    # A handful of single POSTs to exercise the size-1 path too.
    for _ in range(single_count):
        req("POST", f"{base}/v1/questions", token, {"questions": [make_question(idx)]})
        idx += 1
        made += 1

    batch_times = []
    while made < total:
        n = min(batch, total - made)
        qs = [make_question(idx + k) for k in range(n)]
        idx += n
        t0 = time.time()
        req("POST", f"{base}/v1/questions", token, {"questions": qs})
        batch_times.append(time.time() - t0)
        made += n
        now = time.time()
        if now - last_report >= 5:
            rate = made / (now - start)
            print(f"  {made}/{total}  ({rate:.0f} q/s)", flush=True)
            last_report = now

    elapsed = time.time() - start
    batch_times.sort()
    p50 = batch_times[len(batch_times) // 2] * 1000 if batch_times else 0
    p95 = batch_times[int(len(batch_times) * 0.95)] * 1000 if batch_times else 0
    print(
        f"minted {made} in {elapsed:.1f}s  ({made / elapsed:.0f} q/s)  "
        f"batch insert p50={p50:.1f}ms p95={p95:.1f}ms",
        flush=True,
    )


def time_get(base, token, path, runs=3):
    times = []
    total = None
    for _ in range(runs):
        t0 = time.time()
        _, body = req("GET", f"{base}{path}", token)
        times.append((time.time() - t0) * 1000)
        if isinstance(body, dict):
            total = body.get("total", total)
    cold = times[0]
    warm = min(times[1:]) if len(times) > 1 else times[0]
    return cold, warm, total


def bench(base, token):
    _, total_body = req("GET", f"{base}/v1/questions?page=1&pageSize=1", token)
    total = total_body.get("total", 0)
    deep_page = max(1, total // 25)  # near the end of the dataset
    cases = [
        ("page 1, size 25", "/v1/questions?page=1&pageSize=25"),
        ("page 1, size 100", "/v1/questions?page=1&pageSize=100"),
        (f"deep offset (page {deep_page})", f"/v1/questions?page={deep_page}&pageSize=25"),
        ("FTS search=kubernetes", "/v1/questions?search=kubernetes&page=1&pageSize=25"),
        ("FTS deep (page 50)", "/v1/questions?search=rust&page=50&pageSize=25"),
        ("filter kind=mc", "/v1/questions?kind=mc&page=1&pageSize=25"),
    ]
    print(f"\nbenchmark (total rows = {total}):", flush=True)
    print(f"  {'case':<32} {'cold(ms)':>10} {'warm(ms)':>10} {'matched':>10}")
    for label, path in cases:
        cold, warm, t = time_get(base, token, path)
        print(f"  {label:<32} {cold:>10.1f} {warm:>10.1f} {str(t):>10}", flush=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--api", default=DEFAULT_API)
    ap.add_argument("--email", default="instructor@example.com")
    ap.add_argument("--password", default="password123")
    ap.add_argument("--total", type=int, default=1_000_000)
    ap.add_argument("--batch", type=int, default=50)
    ap.add_argument("--singles", type=int, default=20, help="single-row POSTs before batching")
    ap.add_argument("--mode", choices=["mint", "bench", "both"], default="both")
    args = ap.parse_args()

    token = login(args.api, args.email, args.password)
    if args.mode in ("mint", "both"):
        mint(args.api, token, args.total, args.batch, args.singles)
    if args.mode in ("bench", "both"):
        bench(args.api, token)


if __name__ == "__main__":
    main()
