# Plan: Complete owner-isolation + drop learner/instructor naming

**Date:** 2026-06-02
**Branch:** `refactor/remove-sharing`
**For:** Gemini implementation; Claude (TL) reviews + verifies with `make e2e`.

## Goal

Two outcomes the user asked for:

1. **Full owner-isolation everywhere.** Every assessment-by-id access path must be
   scoped to the owner (or the owner's agents). There is exactly one leak today:
   `POST /v1/sessions` checks only that an assessment *exists*, not that the caller
   owns it. Fix it so non-owners get **404** (hide existence).
2. **No more `learner`/`instructor` role naming.** Roles don't exist in the product
   anymore — there are just independent users, each with their own isolated data
   (think: one CS user, one music user). Rename the demo personas to plain,
   role-neutral names. `admin` stays (it is a real DB-granted role, untouched here).

Success = `make e2e` is **green** (run with the dev stack DOWN so Playwright starts
its own web server — see Verification) and `make check` passes.

## Already done on this branch (do NOT redo)

- `web/e2e/admin.spec.ts` deleted (admin behavior is undesigned; spec removed).
- `Makefile` `e2e` target now runs `$(MAKE) --no-print-directory db-admin` before the
  `seed.py` step (Makefile:105-113), mirroring `db-seed: db-admin`. Keep this — the
  seed's premium/agent/cohort steps need a real admin.

---

## Part 1 — API: owner-scope `create_session` (the bug)

**File:** `api/src/http/sessions.rs`, in `create_session` (~line 166).

Replace the existence-only check:

```rust
    if let Some(assessment_id) = body.assessment_id {
        let _row = sqlx::query("SELECT status, created_by FROM tb_assessments WHERE id = $1")
            .bind(assessment_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
            .ok_or(ApiError::NotFound {
                resource: "assessment",
            })?;
    }
```

with an owner-scoped check that mirrors `get_assessment` (assessments.rs:509-521):

```rust
    if let Some(assessment_id) = body.assessment_id {
        // Owner-scoped: only the owner (or their agents) may start a session on an
        // assessment. Non-owners get 404 so existence is not leaked.
        sqlx::query(
            "SELECT 1 FROM tb_assessments \
             WHERE id = $1 \
               AND (created_by = $2 \
                    OR EXISTS (SELECT 1 FROM tb_users u WHERE u.id = created_by AND u.owner_user_id = $2))",
        )
        .bind(assessment_id)
        .bind(auth.owner_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::NotFound {
            resource: "assessment",
        })?;
    }
```

Notes:
- Use `auth.owner_id` (not `auth.user.id`) to match `get_assessment` — this keeps
  agent/sub-account ownership working consistently.
- No DB migration needed: `created_by` and `owner_user_id` already exist.

**Audit (cheap, do it):** grep `api/src/http` for other handlers that look up an
assessment / exam by id without an owner predicate. Confirmed already-scoped:
`get_assessment`, `list_assessments`. Session `answer`/`finish` operate on a session
row keyed by the caller's `user_id`, so they're fine. If any other by-id assessment
read lacks the predicate, scope it the same way. Don't expand scope beyond assessments.

**API tests:** check `api/tests/` for session-creation tests that start a session on an
assessment created by a *different* test user — those will now 404 and must be updated
to create the assessment as the same user (or assert 404 for the cross-owner case).

---

## Part 2 — Rename personas (drop learner/instructor)

Roles are gone; these are just two independent users. Proposed names (adjust to taste —
the user delegated naming; keep them role-neutral and keep `@example.com`):

| Old email                | Old name        | New email             | New name      | Notes |
|--------------------------|-----------------|-----------------------|---------------|-------|
| `learner@example.com`    | Alice Learner   | `ada@example.com`     | Ada Lovelace  | **Primary** demo user — owns + works with the main content |
| `instructor@example.com` | Bob Instructor  | `mira@example.com`    | Mira Okafor   | Second user — owns a separate assessment for isolation/explore demos |
| `admin@example.com`      | Carol Admin     | *(unchanged)*         | Carol Admin   | Real DB-granted admin; leave as-is |

The whole point of the rename is that the **primary user owns the content it interacts
with** — that's what makes the e2e specs pass *correctly* under isolation (not by
loosening isolation).

### 2a. `scripts/seed.py` — flip ownership to the primary user

Currently (seed.py:723-742): `auth` = instructor token creates tags/questions/
assessments/exam; the learner only *takes* the instructor's assessment (breaks under
Part 1). Restructure so:

- `USERS` (seed.py:31-36): rename the two non-admin entries per the table. Drop the
  `role` field's old semantics — keep `"role": "user"` for both (admin row keeps
  `"admin"`; note the API rejects `role: admin` on register, db-admin promotes it).
- **Primary user (Ada) owns the rich content:** set the content-authoring `auth`
  (seed.py:729) to the **primary** user's token. So tags*, questions, the main
  assessment, the draft, and the exam are all `created_by` Ada.
  - *Tags may be global; if `/v1/tags` is not owner-scoped, leave tag auth as-is.*
- **Primary user is premium + owns an agent:** `upgrade_premium_users` (seed.py:328-343)
  should upgrade the **primary** user to premium (premium unlocks the agent-creation
  quota). In `seed_agents` (seed.py:~637-742), the primary user must successfully create
  an agent, and that agent authors an assessment — so `agent-authored.spec.ts` (which
  acts as the primary user) can see agent-authored content via the owner-or-agent
  predicate. The old code created a *learner* agent that 429'd (free quota) — that must
  now succeed because the primary user is premium.
- **Attempts:** `seed_attempts` (seed.py:541-628) already uses the primary
  (ex-`learner`) account — now it takes **its own** assessment, so it passes Part 1.
  Update the `email == "learner@example.com"` / `"instructor@example.com"` lookups
  (seed.py:338, 532, 547, 644, 664, 725) to the new emails.
- **Second user (Mira):** give her at least one **own** active assessment with ≥5
  questions incl. an MCQ, so isolation/explore specs have a distinct owner to assert
  against. Minimal content is fine (can reuse question shapes).
- **Cohort** (seed.py:740, `seed_cohort` via `admin_auth`): leave functioning. If cohort
  enrollment assumes cross-user membership that now conflicts with isolation and breaks
  seeding, scope it to the owner or skip — but only if it actually errors. Don't remove
  the cohort gratuitously.

### 2b. Other scripts

- `scripts/mint_bulk.py:90` — `"email": "instructor@example.com"` → **primary**
  user's email (`make db-bulk` relies on the premium agent quota; the premium user is
  now the primary user). Keep this consistent with whoever 2a makes premium.
- `scripts/bench_questions.py:146` — `--email` default `instructor@example.com` →
  primary user's email.
- `scripts/simulate/instructor.py:12` and `scripts/simulate/agent.py:12` —
  `INSTRUCTOR_EMAIL = "instructor@example.com"` → new email. Rename the constant to
  something role-neutral (e.g. `AUTHOR_EMAIL` / `OWNER_EMAIL`). Optional: rename
  `scripts/simulate/instructor.py` and `scripts/simulate/learner.py` to neutral names;
  if you do, update `make simulate` (Makefile:194-197). If a rename balloons scope,
  just update the email/constant and leave filenames.

### 2c. e2e specs

- `web/e2e/helpers.ts` — wherever `loginAs` default emails / persona strings live,
  update them. (No hardcoded persona found in helpers.ts in the grep, but `loginAs` is
  called with literal emails in the specs below.)
- `web/e2e/learner.spec.ts` — replace `loginAs("learner@example.com")` (line 50),
  the `Alice Learner` assertion (line 59), and the comment (line 44). Optional: rename
  the file to a neutral name (e.g. `user-ui.spec.ts`). The "fresh user" session-flow
  test (`:105`) must, after Part 1, **create its own assessment first** (or act as the
  seeded primary owner) — a brand-new user owns nothing and can no longer take seeded
  content.
- `web/e2e/uiux.spec.ts` — `loginAs("learner@example.com")` (line 46) + `Alice Learner`
  assertion (line 52) → new email/name. `firstMcqAssessmentId(token)` (helpers.ts:159)
  must resolve, which it will once the primary user owns a ≥5-question MCQ assessment.
- `web/e2e/agent-authored.spec.ts:17` — `loginAs("learner@example.com")` → primary
  email; depends on 2a giving the primary user an agent-authored assessment.
- `web/e2e/authoring.spec.ts:17` — `loginAs("learner@example.com")` → primary email.
- `web/e2e/isolation.spec.ts:106` — **fix the test to exercise real isolation.** Owner A
  creates an assessment but **adds no questions**, so today the stranger's
  `POST /v1/sessions` returns 422 (empty assessment), not 404, and the test expected 404
  for the wrong reason. After Part 1: add ≥1 question to Owner A's assessment (so it's a
  valid, startable assessment), then assert the stranger gets **404**. Keep the existing
  list/detail isolation tests (`:12`, `:62`) — they already pass.

### 2d. Docs

- `README.md:19` — demo credentials.
- `docs/stack.yaml:107-108` — `learner` / `instructor` entries.
- `AGENTS.md:81` — `learner@example.com` login example.
- `CLAUDE.md` — the Data Model Constraints note references "upgrade the instructor to
  premium"; reword to the new primary user. Keep the db-admin explanation.
- **Leave historical plan docs alone:** `docs/plans/2026-05-25-role-simulation.md` and
  `docs/plans/2026-05-27-mui-migration.md` are dated records — do not rewrite them.

---

## Verification (required, in order)

1. `make check` → fmt-check + lint + Rust/web tests green. (Fix any `api/tests`
   session-creation tests broken by Part 1.)
2. Bring the dev stack **down first** — a stale `make dev` web server on :23000 with a
   mismatched `NEXT_PUBLIC_API_URL` makes Playwright reuse it and the sidebar identity
   wait times out (this bit us this session). Then:
   - `make db-reset`
   - `make db-up`
   - `make e2e`  ← starts its own API, runs `db-admin` + `seed.py`, starts its own web
     server, runs Playwright.
3. Target: **0 failed**. The 5 specs that fail today
   (`agent-authored`, `explore`, `isolation:106`, `learner:105`, `uiux:37`) must pass
   for the *right* reason — the acting user owns the content — not by relaxing isolation.

## Out of scope / non-goals

- No new "sharing" or cross-user visibility. No sub-account hierarchy work.
- No schema/migration changes.
- Don't touch the admin role or re-add an admin spec.
- Don't author large music/science question banks — minimal distinct content for the
  second user is enough to demonstrate isolation.

## Risk notes

- `api/tests/` session tests are the most likely `make check` breakage from Part 1.
- The premium-user choice must be consistent across `seed.py`, `mint_bulk.py`, and
  `bench_questions.py`, or `make db-bulk` hits the agent-creation quota (429).
- `seed_cohort` may assume cross-user enrollment; only adjust if it actually errors.
