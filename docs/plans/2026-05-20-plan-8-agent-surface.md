# Plan 8 (2026-05-20): Agent surface & sharing

**Goal:** Land the MCP manifest export, ActivityLog, webhooks, `POST /plans`, and the **Share API** (`POST /shares`, `GET /shares/{id}`, `DELETE /shares/{id}`, `GET /{kinds}/{id}/embed`) plus the OG image stub. Write agent skill markdowns that point at the OpenAPI examples.

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §4.11–4.16 (api_keys, webhooks, ActivityLog, StudyPlan, Message, ShareLink), §6.1 agent + share surfaces, §7 (MCP manifest), §8 (webhooks), §10.1 (Share modal + LearningObjectives cross-cutting components).

**Prerequisites:** Plan 7 (stats — webhooks produce stats updates).

**Why this absorbs sharing:** spec §14 declares P8 the home for sharing because `share.create` / `share.get` / `share.revoke` are MCP tools. A separate P11 is reserved if scope grows (e.g. branded social-card pipeline).

---

## Task 1: MCP manifest export

`GET /agents/mcp.json` — public (no scope).

- [ ] **Step 1:** Derive the manifest from the `Tool` column of every row in spec §6.1. The base shape is in spec §7.
- [ ] **Step 2:** Include every `share.*` tool (create, get, revoke) per §7.
- [ ] **Step 3:** Harus extension fields (`method`, `path`, `scope`) MUST be droppable at export time for strict MCP consumers — gate behind `?strict=1`.
- [ ] **Step 4:** Snapshot test — assert the manifest contains every Tool name from §6.1.

## Task 2: OpenAPI export

`GET /agents/openapi.json` — scope `quiz.read`. Already maintained as `api/openapi.yaml`; this endpoint serves the snapshot.

## Task 3: ActivityLog

- [ ] **Step 1:** Populate `activity_log` on every authenticated agent request (middleware). Columns per §4.13.
- [ ] **Step 2:** `GET /agents/activity` — tool `agent.activity`, scope `quiz.read`. Paginated (cursor + limit).
- [ ] **Step 3:** Test: a sequence of agent calls produces a contiguous log.

## Task 4: Webhooks

Outbound HTTPS POST. Signature `X-Harus-Signature: t=<unix_ts>,v1=<hmac_sha256(secret, ts + "." + body)>`. Retries 3× exponential (10s, 60s, 300s).

| Event               | Producer phase           |
| ------------------- | ------------------------ |
| `attempt.submitted` | Plan 5 (session.finish)  |
| `attempt.graded`    | Plan 5 (grader pipeline) |
| `quiz.published`    | Plan 4 (status → active) |
| `exam.opened`       | Plan 6                   |
| `exam.closed`       | Plan 6                   |
| `plan.created`      | Plan 5 (planner)         |

- [ ] **Step 1:** Webhook dispatch worker (background tokio task; retries persisted to `webhook_deliveries`).
- [ ] **Step 2:** Test: stubbed receiver verifies signature; failure path retries 3×.

## Task 5: Plans

`POST /plans` (Plan 5 implements planner; this phase ensures the route + webhook fire).

`GET /plans/{id}` — tool `plan.get`, scope `plan.read`.

## Task 6: Share API (spec §6.1 Share subsection)

| Method | Path                  | Tool           | Scope                                       |
| ------ | --------------------- | -------------- | ------------------------------------------- |
| POST   | `/shares`             | `share.create` | `quiz.read`                                 |
| GET    | `/shares/{id}`        | `share.get`    | (public)                                    |
| DELETE | `/shares/{id}`        | `share.revoke` | `quiz.read` (caller must be owner or admin) |
| GET    | `/{kinds}/{id}/embed` | —              | (public)                                    |

- [ ] **Step 1: `POST /shares`.** Body per §6.1; returns `{ shareId, url, embedUrl, og: { image, title, description } }`. Idempotency: `Idempotency-Key` header primary (per §5.2); natural-tuple fallback on `(created_by, kind, target_id, visibility, include_*)` returns the existing non-revoked share.
- [ ] **Step 2: `GET /shares/{id}`.** Public. Apply §4.16 privacy rules: strip per `include_*` flags. 404 on revoked.
- [ ] **Step 3: `DELETE /shares/{id}`.** Sets `revoked_at`. Owner or admin only.
- [ ] **Step 4: `GET /{kinds}/{id}/embed`.** `kinds ∈ quizzes | exams | items` (plural — derived from the share body's singular `kind`). Router precedence: declare `/quizzes/{id}/embed` **before** `/quizzes/{id}` so the longer path wins. `?interactive=1` enables anonymous attempts.
- [ ] **Step 5: Anonymous attempts.** Write to `anonymous_attempts` only (never `attempts`). Rate-limit per IP (suggested 30 / 10 min / IP). 429 `anonymous_attempt_rate_limited` on overflow.
- [ ] **Step 6: OG image.** v1 returns a stub or static placeholder PNG. Real Satori/Vercel-OG worker is in spec §12 out-of-scope.

## Task 7: Objectives roundtrip (P8-008)

- [ ] **Step 1:** `POST /quizzes` (Plan 4) accepts `objectives[]` — confirm wired here for agent emission.
- [ ] **Step 2:** `POST /quizzes/generate` agent emits 3-4 objectives best-effort; `warnings[]` on insufficient source.
- [ ] **Step 3:** MCP manifest tool schemas include `objectives`.

## Task 8: Agent skill markdowns

- [ ] **Step 1:** `agents/generate-questions/SKILL.md` — fetches weakest tags before generating, uses exact OpenAPI examples.
- [ ] **Step 2:** `agents/analyze-performance/SKILL.md` — reads tag stats + recent attempts.
- [ ] **Step 3:** `agents/adaptive-generation/SKILL.md` — loops weakest tags → exam pool gaps → targeted generation. Handles 422 `pool_insufficient` explicitly.
- [ ] **Step 4:** Update `AGENTS.md` and `README.md` to point at `agents/` as the source for agent operations.

## Task 9: Tests

- [ ] Share golden path: create → get (public) → revoke → 404 on next get.
- [ ] Idempotency: same body + Idempotency-Key returns same shareId; same body without key returns same shareId via tuple fallback.
- [ ] Embed route precedence: GET `/quizzes/{id}/embed` resolves to embed handler (not to `/quizzes/{id}` with stray segment).
- [ ] Anonymous attempt: writes to `anonymous_attempts`; 31st attempt within 10 min from same IP returns 429.
- [ ] MCP manifest contains all `share.*` tools.

## Definition of done

- `make check` + `make test-shares` pass.
- All Plan 8 ledger rows are `done` (P8-001..P8-009).
- Agent skill markdowns reference live OpenAPI examples (not handwritten ones).
- An external agent can complete the loop: register → discover MCP → import a quiz → mint a share → revoke.
