# Plan 2 (2026-05-20): Schema rewrite & auth refactor

**Goal:** Land the full entity model from spec §4 and the auth model from spec §6.3 on Postgres + Axum. Replaces the May-19 schema (carried forward via migrations, not a drop/recreate).

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §4 (entities), §5 (schema overview), §6 (HTTP surface), §13 (migration deltas).

**Prerequisites:** P1 (foundation) complete. May-19 P2 closed out (P2-001..P2-011).

**Out of scope for this phase:** Quiz authoring (P4), engine (P5), Share API handlers (P8). Schema for share_links + anonymous_attempts lands here so P8 can implement handlers without a migration round-trip.

---

## Task 1: Entity tables (spec §4 + §13)

- [ ] **Step 1: Migration plan.** One migration per logical group; never edit a landed migration. Order:
  1. `users` (+ `role` enum: learner | instructor | admin | agent).
  2. `courses`, `cohorts`, `tags`.
  3. `questions` (5 kinds: mc, tf, short, essay, code — see Plan 3 for payload validators) with `points int default 1` (spec §4.5).
  4. `quizzes` (`status` enum, `objectives text[] default '{}'`, spec §4.6); `quiz_questions` join — see Plan 4 for the columns Plan 2 doesn't need.
  5. `exams` + `exam_sections` (`status` enum, `objectives text[] default '{}'`).
  6. `sessions`, `attempts`.
  7. `api_keys`, `webhooks`, `activity_log`, `study_plans`, `messages`.
  8. `share_links` (`created_by_user_id`, `kind`, `target_id`, `visibility` enum, `include_explanation`, `include_score`, `include_attribution`, `og_image_url`, `revoked_at`); `anonymous_attempts` (`share_id`, `ts`, `ip_hash`, `response_payload`).
- [ ] **Step 2: Backfill rules.** Spec §13 lists every delta from the May-19 schema. Notable backfills:
  - `questions.points` defaults to 1 — backfill all existing rows.
  - `quizzes.objectives` and `exams.objectives` default to `'{}'` — no backfill required.
  - `messages.channel` canonicalizes to `'in_app'` (snake_case) — update existing rows.
- [ ] **Step 3: Indexes (spec §5.3).** Load-bearing indexes: `attempts (user_id, submitted_at DESC)`, `share_links (created_by_user_id) WHERE revoked_at IS NULL`, `anonymous_attempts (share_id, ts DESC)`, etc.
- [ ] **Step 4: CHECK constraints.** `share_links.visibility ∈ ('public', 'cohort')`; `messages.channel ∈ ('in_app', 'email')`; quiz/exam status enums.

## Task 2: Scope enum widening (spec §6.2)

- [ ] **Step 1: Add new scopes.** Extend `Scope` enum + DB `CHECK` constraint to include `feedback.write`, `plan.read`, `plan.write`, `admin`. Read-implies subset: `quiz.write` implies `quiz.read`, etc.
- [ ] **Step 2: Display shorthand.** Spec §6.2 line 310: the UI MAY render `*` as shorthand for "all active scopes". `*` is **never** stored and **never** accepted on `POST /me/keys`. Add a parser assertion + test.

## Task 3: Auth model (spec §6.3)

- [ ] **Step 1: Cookie sessions for humans.** Add `axum-login` or equivalent. Effective scopes derive from `users.role` (learner/instructor/admin); the May-19 human bearer-token path is removed.
- [ ] **Step 2: Bearer for agents.** Keep `Authorization: Bearer hk_<env>_<id>` for `role='agent'` users. Revoke = set `revoked_at`; next auth pass returns 401.
- [ ] **Step 3: `POST /agents/register` bootstrap.** Unauthenticated endpoint. Body: `{ label?, scopes[] }`; rejects `admin` scope. Rate-limit per IP (suggested: 5 / hour / IP). Returns `{ apiKey: 'hk_<env>_<full>', userId, openapiUrl, mcpManifestUrl }`. The key is shown once.

## Task 4: Idempotency (spec §5.2)

- [ ] **Step 1:** Keep the May-19 `idempotency_keys(key, request_hash, response_body, created_at)` table and middleware. Already lands as P2-005..P2-010.
- [ ] **Step 2: Document the natural-tuple fallback** for `POST /shares`. Spec §6.1 Share row: when no `Idempotency-Key` header is provided, the server dedupes on `(created_by_user_id, kind, target_id, visibility, include_*)` and returns the existing live (non-revoked) share. Add a `UNIQUE` index that supports this lookup (`WHERE revoked_at IS NULL`).

## Task 5: Tests

- [ ] DB-backed integration test for every new table's roundtrip (insert + select with the typed model).
- [ ] Auth tests: revoked key returns 401, scope insufficient returns 403, `*` rejected on insert.
- [ ] `/agents/register` test: rate-limit triggers; `admin` rejected.
- [ ] Idempotency tests: header path + natural-tuple fallback on `POST /shares`.

## Definition of done

- `make check` and `make test-db` pass.
- All tasks in `docs/plans/tasks/plan-2-schema-auth.jsonl` are `done` (specifically P2-012, P2-013 are the new-spec deltas).
- Spec §4 entity coverage is verifiable: `grep -c "CREATE TABLE" db/migrations/*.sql` ≥ 16 (one per entity in §4.1–§4.16).
