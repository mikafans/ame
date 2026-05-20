# Plan 7 (2026-05-20): Stats & feedback

**Goal:** Land `/quizzes/{id}/stats`, `/exams/{id}/stats`, `POST /messages`, identity/token/admin user APIs, and the item analysis precompute job.

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §6.1 stats + feedback subsections, §4.13 (ActivityLog — populated in P8), §4.15 (Message).

**Prerequisites:** Plan 6 (exams) so exam stats have data.

---

## Task 1: Quiz stats

`GET /quizzes/{id}/stats` — tool `stats.cohort`, scope `stats.read`.

- [ ] **Step 1: Shape.** `{ avg, median, distribution[], items[] }`. `items[]` is per-question item analysis: `{ questionId, correctRate, avgTimeMs, discriminationIdx }`.
- [ ] **Step 2: Query params.** `cohortId?`, `window?: 'last30d' | 'all'`.
- [ ] **Step 3: Precompute.** Item analysis is expensive — precompute on `attempt.submitted` webhook event (Plan 8 produces this) into a `quiz_item_stats` materialized view or rollup table.
- [ ] **Step 4:** Tests: synthetic attempts → expected distribution; verify cohort filter narrows correctly.

## Task 2: Exam stats

`GET /exams/{id}/stats` — tool `exam.stats`, scope `stats.read`.

- [ ] **Step 1: Shape.** `{ passRate, sectionAvgs[], timeP50, timeP95 }`.
- [ ] **Step 2: Section breakdown.** `sectionAvgs[]` = one entry per `exam_sections` row.

## Task 3: Feedback (messages)

`POST /messages` — tool `feedback.send`, scope `feedback.write`.

- [ ] **Step 1: Body.** `{ userId, channel: 'in_app' | 'email', body, linkQuizId? }`. `channel: 'email'` is reserved — accept the value but queue without sending until an email transport is configured.
- [ ] **Step 2: Storage.** Insert into `messages` table. In-app delivery is a pull model (user UI polls `/me/messages` — endpoint in Plan 9 or here, decide at impl time).
- [ ] **Step 3:** Idempotency middleware applies.

## Task 4: Identity / key / webhook APIs (spec §6.1)

`/me/keys`, `/me/keys/{id}/rotate`, `/me/keys/{id}` (DELETE), `/me/webhooks`, `/me/webhooks/{id}` (DELETE) — instructor/admin UI; no agent scope required (these are human-only routes gated on the session cookie).

- [ ] **Step 1:** `POST /me/keys` accepts `{ label, scopes[] }`; `admin` scope only acceptable from a current admin.
- [ ] **Step 2:** Rotate returns the new full token + invalidates the previous hash.
- [ ] **Step 3:** Delete sets `revoked_at`.
- [ ] **Step 4:** Webhook secret shown once on create; stored hashed.

## Task 5: Admin user APIs

- [ ] **Step 1:** Admin-only CRUD for users (list, role change, deactivate).
- [ ] **Step 2:** Scope: `admin` (the only scope that allows operating on other users).

## Task 6: Tests

- [ ] Stats endpoint shape matches spec for both quizzes and exams.
- [ ] `POST /messages` with `channel: 'email'` does not actually send but is queued.
- [ ] Key rotation invalidates the old token (existing P2 test pattern).
- [ ] Admin scope required for `POST /me/keys` with `admin`.

## Definition of done

- `make check` + `make test-stats` pass.
- OpenAPI snapshot includes every Plan 7 path.
- Plan 7 ledger rows are `done`.
- Item analysis precompute job is wired (cron or webhook-triggered).
