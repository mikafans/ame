# Agent / Identity Refactor — Battle Center

Multi-agent execution hub for the refactor specced in
[`docs/specs/2026-05-30-agent-identity-refactor.md`](../../specs/2026-05-30-agent-identity-refactor.md).

This is the coordination board. The machine-readable task list is
[`tasks.jsonl`](./tasks.jsonl); each task's full brief is in
[`context/<id>.md`](./context/).

## Current state (2026-05-30)

- **Branch:** `refactor/agent-identity` (in-place, no worktree on the branch
  itself; dispatched agents may use their own worktrees). One PR at the end.
- **P0 — DONE & committed.** AI-0.1 role migration (`e47ee93`), AI-0.2 Role enum
  collapse `{User,Admin,Agent}` + faucet delete (`f3b42cf`), AI-0.3 web de-gate
  (`9d6931d`). `make check` green on each.
- **P1 — DONE & committed.** AI-1.1 sub-account schema (`20260530150000`), AI-1.2
  ownership resolution in auth, AI-1.3 `/v1/me/agents` CRUD + integration test
  (`api/tests/me.rs`). `make check` green.
- **P2 — DONE & staged.** AI-2.1 agent profiles schema (`20260530160000`), AI-2.2
  behavioral tools (`profile.get`, `memory.*`, `target.set`), AI-2.3 updated
  reference client and docs. `make check` green.
- **P3 — DONE & staged.** AI-3.1 quiz visibility schema (`20260530170000`), AI-3.2
  visibility API surface (`PATCH`, `GET`, `explore`), AI-3.3 cross-owner attempts,
  AI-3.4 share-link demotion. `make check` green.
- **P4 — AI-4.1 DONE & staged.** TB_USERS.PLAN column (`20260530180000`).
- **Next gate: AI-4.2 Plan + quota model.**

**Two decisions are now load-bearing — do not regress them:**

1. **Authoring is for every authenticated user.** Author studio, exam compose,
   grading, draft quizzes, and the Agent API page are NOT admin-gated. The
   `agent` role is token-only and never loads the web UI. Do not reintroduce
   `role === "admin"`/`isAdmin` gates on authoring surfaces.
2. **Sharing is quiz-only, link-visibility** `{private,unlisted,public}` (canonical
   URL, no opaque tokens). Answering requires an account; cross-owner attempts
   roll up to the responder. `tb_share_links` is presentation/embed-only; the
   anonymous-attempt path is removed.

## Roles in this effort

- **Haru** dispatches each task to an implementing agent (per the `agent` field).
- **TL (Claude)** reviews every dispatched result, re-runs verification in the
  **foreground**, gates the commit, and keeps `tasks.jsonl` + this board current.
  TL does not implement P1+ directly unless asked.

## TL review protocol (every handed-off task)

1. Diff the agent's work against the task's `paths` — flag any edit outside them.
2. Check it honors the two load-bearing decisions above and the global gotchas.
3. **Re-run verification yourself, foreground:**
   `make check > /tmp/<id>-check.log 2>&1; echo EXIT=$?` (and `make e2e` if the
   task's `verify` calls for it). Never trust a sub-agent's "passed" claim; never
   background-and-poll.
4. Read the log tail to confirm tests actually ran (not an early bail).
5. Only on green: set the task `done` in `tasks.jsonl`, then ask Haru before
   committing. Commit is task-scoped with the task id in the body.

## How to claim and run a task

1. **Pick a `todo` task whose `depends_on` are all `done`.** Read its `context`
   file before touching code.
2. **Claim it:** set `owner` (your agent name) and `status:"doing"` on its line
   in `tasks.jsonl`. Commit that one-line change first (`chore: claim AI-X.Y`).
3. **Work only inside the task's `paths`.** If you need to edit a file outside
   them, stop and add a new task line instead of widening silently — that file
   is probably another agent's lane.
4. **Verify** with the task's `verify` command (default `make check`). Long
   output → redirect to a file and read it, don't grep rtk output.
5. **Hand off:** set `status:"review"` (needs a review pass) or `done`. Put the
   task id in the commit body. Small, task-scoped commits only.

## Parallelism rule

**Two tasks are safe to run concurrently iff their `paths` are disjoint.** That
is the whole concurrency model — no locks, no leases beyond the `owner` field.
Tasks are cut along file boundaries on purpose: one agent owns `quizzes.rs`,
another owns `sessions.rs`, etc. Run each agent in its **own git worktree** under
this repo (worktrees-by-default); register the worktree path in
`.claude/settings.local.json` → `permissions.additionalDirectories`.

Recommended dispatch model is in each task's `agent` field — mechanical,
well-scoped work goes to **haiku-developer**; cross-cutting type changes and
design-sensitive endpoints go to **sonnet/opus**.

## Phase DAG (the serialization backbone)

```
P0 role collapse ─┬─> P1 sub-accounts ─┬─> P2 agent profiles + tools
                  │                     ├─> P3 quiz visibility + cross-owner attempts
                  │                     └─> P4 plan + quotas ─┬─> P5 export
                  │                                           └─> P6 admin + audit
                  └────────────────────────────────────────────> P7 frontend (needs P1–P6 APIs)
```

- **P0 is the bottleneck** — it rewrites the `Role` enum, so it cannot be
  parallelized and everything waits on it. Do P0 first, alone.
- After P0, **P1 unblocks the widest fan-out.** P2/P3/P4 can then run in
  parallel (different files), with P5/P6 trailing P4.
- **P7 (frontend) is independent of the internal API file layout** — each page
  only needs its backing endpoints live, so P7 tasks start as soon as their
  specific phase lands, not after all of P6.

## Lane view (who can run together)

| Lane | Owns | Tasks |
|------|------|-------|
| `db` | `db/migrations/` | AI-0.1, AI-1.1, AI-2.1, AI-3.1, AI-4.1, AI-6.1 |
| `domain` | `api/src/domain/` | folded into the api task that needs the type |
| `api` | one `.rs` file each | AI-0.2, AI-1.2/1.3, AI-2.2, AI-3.2/3.3/3.4, AI-4.2/4.3, AI-5.1, AI-6.2 |
| `agents` | `agents/` | AI-2.3 |
| `web` | one page-dir each | AI-0.3, AI-7.1…7.7 |

Within a phase, the `db` migration lands first; the per-file `api` tasks then
fan out in parallel.

## Status values

`todo` → claimable when deps are `done` · `doing` → claimed, in progress ·
`review` → implemented, needs a review/cleanup pass before dependents start ·
`done` → verified · `blocked` → needs an external decision (note it in the line).

## Global gotchas (apply to every task)

- MC options are bare `Vec<String>` — never `{text:...}`.
- Token secrets: `sha256` only, constant-time compare via `auth/token.rs`
  (`generate_secret`/`hash_secret`/`verify_token_secret`). Never Argon2 for tokens.
- No `DEMO_MODE` — it was deleted; do not reintroduce a runtime auth bypass.
- No scheduled-exam fields, no SSO. Email+password only for humans; agents are
  token-only.
- Config files 2-space indent; Rust 4-space. Conventional commits, no emojis.
- Ask before committing/pushing is the human's rule — within a dispatched task,
  commit task-scoped changes with the task id, but do not push or open PRs
  unless the task says so.
