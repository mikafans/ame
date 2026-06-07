# AME — Agent-Surface Workflow Audit (mid-term)

**Date**: 2026-06-07
**Branch**: `feat/admin-token-audit`
**Scope**: the agent surface end-to-end — discovery (`llms.txt` / `skill.json`),
the scope model, the tool catalog, and where an owner observes agent output.
**Reviewer**: claude-opus-4-8 (session, TL mode)

## TL;DR

The agent surface is coherent and well-documented: one authenticated door
(`POST /v1/agents/run`), a public discovery contract (`GET /llms.txt`,
`GET /skill.json`), and a fixed tool catalog gated by coarse scopes. `llms.txt`
**is still the canonical agent-learning entry point** and is live — it is
compiled into the binary (`include_str!`) and served verbatim.

The one real seam is **scope/tool drift**: the scope picker and `llms.txt` both
advertise three scopes — `feedback.write`, `plan.read`, `plan.write` — that **no
agent tool consumes**. `feedback.write` does gate a REST route
(`POST /v1/messages`), but that route writes into a table with **no reader**: no
GET endpoint, no inbox UI, no email transport. So "where does the owner see the
result?" — today, **nowhere**. This needs a decision: build the read side, or
drop the scope from the agent surface.

---

## 1. The one-door model

An agent token is confined to four paths; every other route returns `403`
(`agents.rs` routers, `llms.txt` §"The single door"):

| Path | Purpose |
|---|---|
| `POST /v1/agents/run` | the dispatcher — **every** tool runs through here |
| `GET /v1/agents/activity` | the agent's own call log, newest first |
| `GET /llms.txt` / `GET /skill.json` / `GET /openapi.yaml` | public discovery |

Every tool is the same shape: `{ "tool": "<name>", "params": {...} }` →
`{ "ok": true, "tool", "result" }`. Scope mismatch → `403`; bad params → `422`.
Agents never hold learner sessions and never take assessments — the learner REST
surface is human-only.

## 2. Discovery: is `llms.txt` still relevant?

**Yes — it is the live, canonical contract.** `GET /llms.txt`
(`agents.rs:79`) serves `api/llms.txt` via `include_str!`, so the file in the
repo *is* what agents fetch. It is the human-readable entry point; `skill.json`
(`build_skill_manifest`, `agents.rs:205`) is its machine-readable twin (the
18-tool JSON catalog with `input_schema` per tool).

Two consumers, one source intent:

- `llms.txt` — prose contract + task playbooks (analyze-performance,
  generate-questions, adaptive-generation). This is how an agent *learns* AME.
- `skill.json` — the tool list an agent *binds* to. `?strict=1` drops the
  `method`/`path`/`scope` hints.

**Drift to fix**: `llms.txt` §"Getting a key" lists `feedback.write`,
`plan.read`, `plan.write` under **Valid scopes**, but its own **Tools** section
defines no tool for any of them (only `assessment.*`, `attempt.*`, `stats.*`,
and the self-management four). An agent reading the contract literally can
request a scope it can never exercise. See §4.

## 3. Scopes vs tools — why 8 ≠ 18

Scopes are **coarse capability bundles**, not per-tool grants. The token stores
scopes (`api_tokens.scopes`, `Scope` enum at `domain/user.rs:53`); tools are
derived. The picker offers **8** scopes (the 9-variant enum minus `admin`, which
is never self-grantable). The manifest exposes **18** tools. They map like this:

| Scope (picker) | Agent tools it unlocks |
|---|---|
| `assessment.read` | assessment.list, assessment.get, question.list, activity.list |
| `assessment.write` | assessment.create, assessment.batchCreate, assessment.update, assessment.addQuestion, question.create, question.promote |
| `attempt.read` | attempt.list |
| `attempt.write` | attempt.grade |
| `stats.read` | assessment.stats, stats.user |
| `feedback.write` | **— none (REST `/v1/messages` only)** |
| `plan.read` | **— none (REST `/v1/plans` only)** |
| `plan.write` | **— none (REST `/v1/plans` only)** |
| *(no scope)* | profile.get, memory.set, memory.append, target.set |

So the 18 tools touch only **5** of the 8 scopes; 4 self-management tools need no
scope at all. The new-agent picker (`web/app/(learner)/agent/page.tsx`) now shows
the tools under each scope and an "Always available" footer for the four
self-management tools, so this fan-out is visible at grant time.

## 4. Orphan scopes — the `feedback.write` dead-end

`feedback.write` gates exactly one route, `POST /v1/messages`
(`messages.rs:52`). That handler inserts into `tb_messages`
(`from_user_id`, `to_user_id`, `channel ∈ {in_app, email}`, `body`,
`status='queued'`) and returns `201 { messageId, status }`.

What happens next — traced through the code:

- **No read path.** There is no `GET /v1/messages`, no inbox endpoint, nothing
  that selects from `tb_messages`. The table's `status` CHECK allows
  `queued|delivered|read` and there is an unread index
  (`..._to_user ON (to_user_id, created_at DESC) WHERE status != 'read'`), but
  nothing ever advances `queued` or reads the row.
- **No UI.** No inbox / notifications component anywhere in `web/app`.
- **No transport.** The handler comment is explicit: *"email channel: stored as
  queued; no transport in P7."*

**Conclusion**: a `feedback.write` agent can post messages to an owner, but the
owner has no way to see them. The write side is real; the read side was never
built. `plan.read` / `plan.write` are the milder version — they gate real REST
plan routes, but those routes are human-surface and unreachable by an agent
token, so granting these scopes *to an agent* is inert.

### Decision needed

Pick one (my recommendation: **B** short-term, **A** if messaging is on the
roadmap):

- **A — Build the read side.** Add `GET /v1/messages` (owner-scoped inbox) + a
  notifications surface in the learner UI, and have `attempt.grade`/playbooks
  optionally emit a message. Only then does `feedback.write` mean something an
  owner can observe. Largest scope; only worth it if agent→human notification is
  a product goal.
- **B — Drop the orphans from the agent surface.** Remove `feedback.write`,
  `plan.read`, `plan.write` from the picker's `ALL_SCOPES` and from `llms.txt`
  §"Valid scopes". Keep the `Scope` enum + REST enforcement intact (the human
  surface still uses them). Smallest change; removes the false promise.
- **C — Document the gap, change nothing.** Annotate the three as "REST API
  only — not exercised by agent tools" (the picker already labels them this way
  after today's change) and move on. Lowest effort; leaves the dead `messages`
  write reachable.

## 5. Where an owner *does* observe agent activity

For the scopes that work, the feedback loop is real and visible:

- **Activity log** — `GET /v1/agents/activity` / the `activity.list` tool; every
  agent call is logged (`tb_activity`, `tool_name`/`method`). Surfaced in the
  learner agent page and the admin views.
- **Authored content** — `assessment.*` / `question.*` tools write to the same
  tables the human UI renders; published assessments appear immediately.
- **Grades** — `attempt.grade` resolves pending essay/code attempts; the learner
  sees the score on their results page.
- **Admin audit** — `/admin/tokens` (scopes per token, usage history, revoke) and
  `/admin/health` (agent count). This branch (`feat/admin-token-audit`) extends
  exactly this surface.

So the platform already *has* the "owner sees the result" loop for authoring,
grading, and stats — `feedback.write` is the lone capability whose output has no
landing place.

## 6. Recommendations (priority order)

1. **Resolve the orphan scopes** (§4) — decide A/B/C. Until then `llms.txt`
   over-promises. *(blocking for contract honesty)*
2. **De-drift `llms.txt`** — whatever §4 decides, make "Valid scopes" list match
   the tools that exist. The file is `include_str!`-compiled, so a doc edit ships
   with the binary — keep it truthful.
3. **Keep the picker as the union, labelled** — the new per-scope tool chips +
   "REST API only" markers + "Always available" footer make the model
   self-explaining; preserve that pattern if scopes change.
4. **(If A)** add `GET /v1/messages` + inbox before re-advertising
   `feedback.write` to agents.

## Appendix — files touched this session (UI)

`web/app/(learner)/agent/page.tsx`:
- `ALL_SCOPES` gains a `tools: string[]` field; each scope card renders its tools
  as chips, or "No agent tool — REST API only" for the three orphans.
- New "Always available (no scope required)" footer listing `profile.get`,
  `memory.set`, `memory.append`, `target.set`.
