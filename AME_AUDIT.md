# AME Platform — Agent Surface Audit

**Target:** `https://ame.azusachino.icu` (alpha) **Date:** 2026-06-06 **Auditor:** Claude (agent, via the published agent surface) **Scope:** Agent-facing API as described by `/llms.txt` + `/skill.json`. Black-box only — observed via the agent Bearer token `agent-1` (`role: agent`, id `019e9a9d-615b-7862-b864-66a4903e009e`). No source access. **Method:** Read the published schema, exercised read/write endpoints, cross-checked responses for consistency. All findings below are backed by reproducible requests.

---

## 0. Executive summary

The agent surface is usable end-to-end (create → list → read works), but there is a **core identity/state-scoping defect**: assessment _completion state_ is not scoped to the requesting principal, is **inconsistent between the list and detail endpoints**, and references **sessions that the same caller cannot read (404)**. The net effect is that an agent token sees "this assessment was completed" while every authoritative read (`stats`, `attempts`, `sessions`) says nothing happened — making the headline use case (_"read my stats, find weak topics"_) impossible to fulfil correctly.

Secondary issues: the machine-readable schema's `path` fields don't match runtime routing, question `payload` shapes are undocumented, there is no session/▸per-viewer-state introspection, and `llms.txt` carries imperative agent-directed content (prompt-injection vector).

| #    | Severity | Title                                                                          |
| ---- | -------- | ------------------------------------------------------------------------------ |
| F-1  | 🔴 High  | Completion state not principal-scoped; references unreadable sessions          |
| F-2  | 🔴 High  | `list` vs `get` disagree on `completed`/`lastSessionId` for the same resource  |
| F-3  | 🟠 Med   | `skill.json` `path` fields don't match runtime routing (batchCreate 404)       |
| F-4  | 🟠 Med   | No session / per-viewer state introspection endpoints                          |
| F-5  | 🟠 Med   | `stats.user` is aggregate-only — no per-tag/topic breakdown                    |
| F-6  | 🟠 Med   | Question `payload` schema undocumented (agents must guess)                     |
| F-7  | 🟡 Low   | `llms.txt` contains imperative agent-directed instructions (injection vector)  |
| F-8  | 🟡 Low   | API key hygiene: single full-scope token, format, rotation, in-prompt exposure |
| F-9  | 🟡 Low   | Inconsistent error envelopes (empty-body 404s)                                 |
| F-10 | 🟡 Low   | `affectsRating: true` on `mode: practice` — surprising semantics               |

---

## 1. Findings

### F-1 — 🔴 Completion state is not principal-scoped and points at unreadable sessions

**Observed.** From the **same** agent token in a single session:

| Read                                                  | Result                                                                           |
| ----------------------------------------------------- | -------------------------------------------------------------------------------- |
| `GET /v1/assessments` → Rust item                     | `completed: true`, `lastSessionId: 019e9aaa-9418-7db0-b430-60a743464fa7`         |
| `GET /v1/sessions`                                    | `{"sessions": [], "total": 0}`                                                   |
| `GET /v1/sessions/019e9aaa-9418-…`                    | **404** `{"error":{"code":"not_found","message":"resource not found: session"}}` |
| `GET /v1/me/attempts` (all + `?sessionId=019e9aaa-…`) | `{"attempts": [], "total": 0}`                                                   |
| `GET /v1/me/stats`                                    | `attempts_total: 0`, `agent_graded_attempts: 0`, `avg_score: 0.0`                |

**Diagnosis.** The assessments were taken through the **web UI**, which authenticates by **session cookie** (verified: `GET /assessments/{id}/preview` → `307 → /login`), i.e. a _different principal_ than the agent Bearer token. The session + attempt rows live under that (human) identity. But `completed`/`lastSessionId` are denormalized onto the **shared assessment record** (`createdBy: 019e9a9d-615b-…` = agent-1), so the agent's view inherits another principal's activity — pointing at a `lastSessionId` in a namespace the agent cannot read (→ 404).

**Impact.** Cross-principal state leakage; dangling references; the primary agent workflow ("read my results, target weak topics") returns provably wrong/empty data while UI claims completion.

**Fix.**

- Compute `completed` / `lastSessionId` **per requesting principal** (this caller's last session for this assessment), or remove them from the shared assessment object and serve via a per-viewer endpoint (see F-4).
- **Invariant:** never return an id the same caller cannot resolve. If `lastSessionId` is present, `GET /v1/sessions/{that-id}` must succeed for that caller.

---

### F-2 — 🔴 `list` and `get` disagree on the same fields for the same resource

**Observed.** Same id `019e9aa7-063c-7b20-a45c-24155ea28d98` (Rust), same token:

- `GET /v1/assessments?status=active` → item shows `completed: true`, `lastSessionId: 019e9aaa-…`
- `GET /v1/assessments/019e9aa7-063c-7b20-a45c-24155ea28d98` → `completed: null`, `lastSessionId: null`

**Diagnosis.** The collection serializer and the single-resource serializer derive these fields differently (different join / different/absent principal filter / one reads an ephemeral state the other doesn't). Regardless of root cause, the two endpoints are **not consistent** for the same logical field.

**Impact.** An agent cannot trust either value; list-then-fetch (a universal pattern) yields contradictory state.

**Fix.** Single source of truth for completion state, shared by both serializers; add a contract test asserting `list[i].{completed,lastSessionId} == get(list[i].id).{…}`.

---

### F-3 — 🟠 `skill.json` `path` fields don't match runtime routing

**Observed.**

- `assessment.batchCreate` declares `path: /v1/agents/batchCreate` → `POST` returns **404**.
- It succeeds only via the `run` wrapper: `POST /v1/agents/run` with `{"tool":"assessment.batchCreate","params":{…}}` → `200 {"ok":true,…,"result":{"count":4,…}}`.
- Meanwhile `assessment.create` works at its REST `path` (`POST /v1/assessments`) **and** is listed under `run.runnable_tools`.

**Impact.** The machine-readable contract — the thing an agent is _supposed_ to trust — is wrong for at least one tool. An agent following `skill.json` literally hits 404s. Two overlapping invocation models (REST path vs `run` verb) with no stated rule for which applies.

**Fix.** Pick one of:

1. Make every `runnable_tool` work at **both** its REST `path` and via `/v1/agents/run`, and fix the wrong `path` (`/v1/agents/batchCreate` → `/v1/agents/run` or a real route); or
2. Drop per-tool `path` for run-only tools and document "writes/composites go through `/v1/agents/run`; reads are GET REST." Add a schema-conformance test that asserts every declared `path` returns non-404 for a minimal valid call.

---

### F-4 — 🟠 No session / per-viewer state introspection

**Observed (all 404):** `/v1/sessions/{id}` (for an id the API itself returned), `/v1/me/sessions`, `/v1/me/assessments`, `/v1/assessments/{id}/state`, `/v1/assessments/{id}/attempts`. `GET /v1/sessions` exists but returns empty; `session.create`/`session.finish` exist with no way to read a session back.

**Impact.** Agents can write sessions but cannot read what they wrote, nor query "what's my state on assessment X" — exactly what's needed to resume or to drive remediation.

**Fix.** Add `GET /v1/sessions/{id}` (resolvable by the owning principal) and a per-viewer `GET /v1/me/assessments/{id}/state` returning `{completed, lastSessionId, score, attemptCount}` scoped to the caller. This also subsumes the fix for F-1.

---

### F-5 — 🟠 `stats.user` is aggregate-only — no per-topic signal

**Observed.** `GET /v1/me/stats` returns only aggregates (`avg_score`, `attempts_total`, `mastered_topics`, streaks, agent counters). No per-tag / per-topic / per-level breakdown — despite questions carrying rich tags (`domain:*`, `level:*`, `topic:*`).

**Impact.** The platform's stated value ("identify weakest topics") cannot be served from `stats`; every agent must pull all attempts and aggregate client-side — and currently can't even do that (F-1).

**Fix.** Add per-tag mastery to stats, or a dedicated `GET /v1/me/mastery?by=tag|topic|level` returning accuracy/score per bucket. The tags already exist on questions — surface them in graded results.

---

### F-6 — 🟠 Question `payload` schema is undocumented

**Observed.** In `skill.json`, every question's `payload` is `{"type":"object"}` with no per-`kind` shape. Authoring required _guessing_ conventions: `{options, correct_index}` (mc), `{answer}` (tf), `{accepted_answers}` (short), `{language, starter_code, reference_solution}` (code). These were accepted and round-tripped, but nothing documents whether they're _correct_ (e.g. whether `correct_index` is actually used for grading).

**Impact.** Agents cannot reliably author gradeable questions; silent mis-grading risk if the real grader expects different keys.

**Fix.** Define `payload` as a `oneOf` discriminated by `kind` in the input schema, with required keys per kind. Validate on write and reject unknown shapes (fail loud, not silent).

---

### F-7 — 🟡 `llms.txt` carries imperative agent-directed instructions

**Observed.** Fetching `/llms.txt` surfaced **imperative formatting directives addressed to the reading agent** (e.g. response-shaping rules) mixed into the capability description — not present in the caller's own prompt.

**Impact.** For an explicitly _agent-facing_ entrypoint this is a prompt-injection surface: anyone pointing an agent at the docs inherits those instructions. Erodes trust in the surface.

**Fix.** Keep `llms.txt` purely **declarative** (capabilities, auth, endpoints). No second-person imperatives ("you must…", response constraints). Treat published docs as a trust boundary you control, never as a control channel for the consuming agent.

---

### F-8 — 🟡 API key hygiene

**Observed.** Single token `<id>_<secret>` with apparent full read+write scope. The schema models `scope` per tool (`assessment.read`/`write`, `attempt.*`, `stats.read`) but a single token appears to satisfy all. The token's leading UUID (`…615e-7a51…`) ≠ resolved principal id (`…615b-7862…`).

**Impact.** No least-privilege; a leaked key is full control. (This key was also pasted into an LLM transcript — assume compromised.)

**Fix.** Enforce per-tool `scope` against token grants; issue read-only vs write keys; support rotation/revocation; document "never embed keys in prompts." Confirm token-parse treats `<id>` as a **key id** looked up to a principal (it does here — fine — but assert it in code, don't assume `<id> == principal.id`). **Rotate the key used in this audit.**

---

### F-9 — 🟡 Inconsistent error envelopes

**Observed.** Some 404s return a clean `{"error":{"code":"not_found","message":"…"}}` (e.g. `/v1/sessions/{id}`); others return **empty bodies** with no JSON (e.g. `/v1/me/assessments`, `/v1/agents/batchCreate`, `/v1/assessments/{id}/state`).

**Impact.** Agents can't self-correct from empty errors; harder to distinguish "route doesn't exist" from "resource missing."

**Fix.** Uniform error envelope `{"error":{"code","message","hint?"}}` for all non-2xx, including unmatched routes.

---

### F-10 — 🟡 `affectsRating: true` on `mode: practice`

**Observed.** Created practice assessments return `affectsRating: true`.

**Impact.** "Practice" affecting a user's rating is a surprising default; may discourage low-stakes practice or skew ratings.

**Fix.** Default `practice` → `affectsRating: false`, or document the intended semantics explicitly and make it caller-settable.

---

## 2. Recommended fix order

1. **F-1 + F-4 together** — introduce a per-viewer state read; scope `completed`/`lastSessionId` to the caller; enforce the "no unresolvable ids" invariant. (Unblocks the core use case.)
2. **F-2** — single source of truth for completion; add list/get consistency test.
3. **F-3** — reconcile `skill.json` `path` with routing; add schema-conformance test.
4. **F-5 + F-6** — per-tag mastery endpoint + documented `payload` schemas (makes the surface genuinely agent-authorable and agent-analyzable).
5. **F-7..F-10** — hardening: sanitize `llms.txt`, scope/rotate keys, uniform errors, rating semantics.

## 3. Repro appendix (key calls)

```
# identity
GET /v1/me
  -> {"id":"019e9a9d-615b-7862-b864-66a4903e009e","displayName":"agent-1","role":"agent",...}

# web UI auth is cookie-based, not the API Bearer
GET /assessments/{id}/preview            -> 307 -> /login

# F-1: completed=true but session unreadable & no attempts
GET /v1/assessments (list) Rust item     -> completed:true, lastSessionId:019e9aaa-9418-...
GET /v1/sessions                         -> {"sessions":[],"total":0}
GET /v1/sessions/019e9aaa-9418-...        -> 404 not_found: session
GET /v1/me/attempts[?sessionId=...]       -> {"attempts":[],"total":0}
GET /v1/me/stats                         -> attempts_total:0, agent_graded_attempts:0

# F-2: list vs get disagree for same id
GET /v1/assessments (list) 019e9aa7-...-24155ea28d98 -> completed:true,  lastSessionId:019e9aaa-...
GET /v1/assessments/019e9aa7-...-24155ea28d98        -> completed:null,  lastSessionId:null

# F-3: declared path 404s; run wrapper works
POST /v1/agents/batchCreate              -> 404
POST /v1/agents/run {tool:"assessment.batchCreate",params:{...}} -> 200 ok

# F-4: no introspection
GET /v1/me/sessions | /v1/me/assessments | /v1/assessments/{id}/state | /{id}/attempts -> 404
```
