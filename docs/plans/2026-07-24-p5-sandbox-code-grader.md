# ADR: P5 sandboxed code grader (design pass — for review)

Status: **draft for review — no code written**. Scope: issue #36 P5
("sandboxed code runner — real CS auto-grading"). Companion to
`2026-07-23-field-model-capability-design.md` (decision #4) and the edX borrow
set. This document is the design pass requested before any sandbox code lands.

## 1. Problem

A CS field course needs to auto-grade learner-submitted **programs**, not just
recognise a memorised string. Today:

- `QuestionKind::Code` routes unconditionally to manual review —
  `grade_answer` returns `AnswerEvaluationStatus::ManualReview`
  (`crates/ame-platform-application/src/question.rs:301`), and
  `grade_assessment` then withholds the score and sets
  `pending_manual_review` (`.../assessment.rs:157-164`). There is **no**
  exemplar/exact-match path in the grader (the `CLAUDE.md` note describing one
  is stale for this rework).
- Task submissions already model async, out-of-band evaluation:
  `TaskEvaluationMethod::Automatic` exists alongside `SelfReview` / `Agent` /
  `Manual`, and the submission lifecycle already has `Submitted → InReview →
  Reviewed/Rejected` with a nullable `score`
  (`crates/ame-platform-domain/src/task.rs:44-83`). A grader that fills in
  `score` + `feedback` fits this shape with no new lifecycle.

## 2. Kill-gate — what breaks for a real user without P5?

**Right now: nothing ships broken.** The golden Flink CS journey
(`topic-blueprints.json:368-601`) contains only explanation / example /
diagnostic / scenario activities — **no code-submission activity exists**. So
P5 is not blocking any currently-seeded learner surface.

P5 becomes blocking the moment a CS journey introduces a real coding activity
(e.g. "write a tumbling-window aggregation" or a Flink SQL query). That is a
**P2 golden-journey** decision, not shipped today.

**Consequence for sequencing:** design now (this doc), but **defer
implementation until the golden CS journey actually adds a coding activity**.
Building an untrusted-code execution surface before any journey uses it would
be exactly the `milestone-done-by-plumbing` pitfall — a capability with no
seeded, tested consumer. Recommendation §7 reflects this.

## 3. Constraints

- **One deployment** (CLAUDE.md architecture): the API is a single Rust/axum
  service. No separate admin service; role gating is per-route. Any grader
  process/queue is operated alongside it, not as a new product tier.
- **Owner-scoped + provenance-gated** authoring is non-negotiable: a graded
  code activity's content comes from a published generation run, same as every
  other authored activity.
- **Module boundaries**: `engine/`/`assess/` may depend on `bank/`/`domain/`,
  never the reverse; `bank/` must not learn that attempts exist. A grader is an
  `assess/`-side concern consuming `bank/` content.
- **Untrusted code**: learner submissions are hostile by assumption — no
  network, constrained CPU/memory/wall-clock/pids/filesystem, non-root, dropped
  capabilities. This is the whole reason P5 is design-first.
- **Nix-first, Make-driven**: the runner's toolchain must come from the
  devShell / a pinned image, and be exercised through a `make` target.

## 4. Grading contract (independent of execution model)

Regardless of how code runs, the graded unit is:

```
CodeGradingSpec {              # authored content, provenance-gated
  language: "python" | ...     # enumerated, closed set
  cases: [ { id, input, expected, weight, hidden } ]   # I/O test cases
  limits: { wall_ms, mem_mb, ... }   # server-clamped, author cannot exceed max
}

CodeGradingResult {
  per_case: [ { id, passed, observed, truncated } ]
  score: 0.0..=1.0             # weighted pass ratio
  status: Correct | Incorrect | Partial | Error(compile/timeout/runtime) | ManualReview
  feedback: json               # visible-case diffs only; hidden cases report pass/fail
}
```

- Grading is **I/O-test-case based** (edX's canonical shape), not exemplar
  string match. Weighted pass ratio maps cleanly onto the existing `score`
  (`0.0..=1.0`) already used by tasks and assessments.
- Hidden cases prevent overfitting to visible examples; their diffs are never
  returned, only pass/fail.
- On any sandbox failure (timeout, OOM, compile error) the result is explicit
  (`status = Error(...)`) — **never silently "correct"**, mirroring how numeric
  grading returns explicit `Incorrect` on unparseable input.

This contract is the part worth pinning with tests first; the execution model
behind it can change without breaking it.

## 5. Execution-model options

### Option A — Async external grader queue (edX XQueue / xqueue-watcher shape)

The API enqueues a submission; a separate worker pulls, runs the code in an
isolated sandbox, and posts the result back; the submission moves
`Submitted → InReview → Reviewed` and `score`/`feedback` are filled in.

- **Pros:** matches `TaskEvaluationMethod::Automatic` + the existing async
  review lifecycle with **zero new states**; the untrusted runner is fully
  out-of-process and independently resource-capped/killable; slow or hostile
  submissions never block an API request; horizontally scalable; this is the
  battle-tested edX pattern the design plan already named.
- **Cons:** most moving parts — a queue (or a DB-backed work table), a worker
  loop, retry/timeout/dead-letter handling, and result-callback auth. Grading
  is eventually-consistent (learner sees "in review" briefly).

### Option B — Synchronous sandboxed subprocess

The API handler spawns the sandboxed runner inline (subprocess with
seccomp/rlimits, or a one-shot container) and returns the grade in the same
request.

- **Pros:** simplest mental model; immediate feedback; no queue/worker/callback
  infrastructure; the async lifecycle states go unused.
- **Cons:** untrusted execution inside the API request path — a hostile
  submission ties up a request worker for the full wall-clock limit; couples API
  availability/latency to grading load; sandbox escape is co-located with the
  API process; hard limit on concurrency = API worker count. Poor fit for
  "hostile by assumption."

### Option C — Container-per-submission

Each submission runs in a fresh throwaway container (gVisor/Firecracker/plain
OCI with a locked-down profile), orchestrated either sync (B-like) or async
(A-like).

- **Pros:** strongest isolation boundary; clean per-run teardown; language
  toolchains are pinned in the image.
- **Cons:** heaviest ops (image build/registry, runtime like gVisor, per-run
  startup cost); overkill for the single-deployment, single-operator model
  today; container runtime availability has already bitten this project
  (`local-up`/Podman/gvproxy history in the session log). Cost not justified
  until multi-tenant/high-volume.

## 6. Comparison

| Dimension            | A: async queue        | B: sync subprocess   | C: container/run    |
| -------------------- | --------------------- | -------------------- | ------------------- |
| Fits existing states | yes (Automatic+async) | unused               | either              |
| Isolation from API   | strong (out-of-proc)  | weak (in-request)    | strongest           |
| Latency to learner   | seconds (eventual)    | immediate            | immediate/eventual  |
| Ops complexity       | medium                | low                  | high                |
| DoS resistance       | good                  | poor                 | good                |
| Right-sized for now  | yes                   | risky                | over-built          |

## 7. Recommendation

1. **Adopt the Option A shape (async external grader) as the target**, but
   implement it in the **leanest form**: a **DB-backed work table** polled by an
   in-repo worker task, *not* a standalone queue service. Reuses Postgres +
   sqlx we already run; no new infrastructure; upgradeable to a real queue later
   without changing the §4 contract or the submission lifecycle.
2. **Isolation for v1 = sandboxed subprocess under OS limits** (non-root,
   seccomp/`rlimit` CPU+mem+pids, no network, read-only ephemeral FS, hard
   wall-clock kill), run **by the worker**, out of the API request path. Defer
   gVisor/containers (Option C) until volume or multi-tenancy justifies them —
   the §4 contract makes that swap non-breaking.
3. **Language scope v1 = one language, Python.** The Flink journey's teachable
   coding surface is best introduced as small self-contained algorithmic/SQL-ish
   exercises first; JVM (Java/Scala) and a real Flink mini-cluster are a large
   toolchain step and are **explicitly deferred**. (Open question O1.)
4. **Sequence:** land P5 **with** the first CS coding activity in the P2 golden
   journey, test-first, so the capability ships with a seeded, tested consumer —
   not before. Pin the §4 contract tests and a per-case grading unit test first,
   then the worker + sandbox, then the journey activity.

## 8. Test-first plan (when P5 is scheduled)

1. Pure-Rust unit tests for the **§4 grading contract**: weighted pass ratio →
   `score`; hidden-case redaction; explicit `Error` status on timeout/compile
   failure; malformed spec rejected at authoring (mirrors `validate_question`).
2. Worker contract test against the DB work table: enqueue → claim (atomic,
   single-winner) → result write → submission transitions
   `Submitted → Reviewed` with `score`/`feedback`, evidence recorded to the
   objective (same path as `review_submission` in `api/src/http/tasks.rs`).
3. Sandbox integration test: a known-malicious submission (infinite loop /
   fork bomb / network attempt / large alloc) is contained and returns
   `Error`, not a hang or an escape.
4. e2e: a seeded CS coding activity graded end-to-end in the golden journey.

## 9. Open questions for review

- **O1 — language scope.** Confirm Python-only for v1, or does the intended
  golden CS coding activity require Flink SQL / JVM from the start? This is the
  single biggest driver of runner complexity.
- **O2 — sandbox mechanism on the target host.** OS-level (seccomp + rlimits +
  namespaces) is enough for v1 given one operator; do we want gVisor/nsjail
  from the start for defence-in-depth, accepting the ops cost?
- **O3 — feedback richness.** How much of a failing visible case do we show
  (expected vs observed diff) without turning grading into an answer key?
- **O4 — scheduling.** Confirm P5 lands with the P2 golden CS coding activity
  (recommended) rather than as standalone plumbing ahead of a consumer.
