# Phase 4 — Drop legacy quiz/exam tables + collapse baseline churn

**Branch:** `refactor/agent-identity` (work in place, no worktree)
**Context:** Final phase of the assessment-unification migration. Legacy `tb_quizzes`/`tb_exams`
are still read by a few live endpoints; those must be migrated onto the unified `tb_assessments`
table **before** the tables can be dropped. Ships as **two commits**.

> ⚠️ The quota/export/moderate queries are **runtime** `sqlx::query(...)` (NOT compile-checked).
> A wrong column name passes `make check` build/clippy but 500s at runtime. The integration test
> `test_admin_flow_and_audit` exercises `/v1/admin/moderate` — it must stay green.

## Reference: `tb_assessments` columns (target table)

`id`, `title`, `description`, `mode`, `status` (`draft`/`active`/`archived`), `objectives`,
`course`, `duration_min`, `time_limit_seconds`, `total_points`, `passing_points`,
`show_results_during`, `affects_rating`, `method`, `composition_trace`,
`created_by uuid` (FK `tb_users`), `created_at`, `updated_at`,
`visibility text` (`'public'`/`'private'`, default `'private'` — added via ALTER at baseline.sql:647).

---

## Commit A — migrate live endpoints onto `tb_assessments`

Do NOT touch `db/migrations/`, `api/src/http/messages.rs`, or `api/src/domain/exam.rs` in this commit.
Do NOT drop any tables here.

### A1. `api/src/http/quota.rs` (~L75–83)
Swap the `PublicQuiz` usage count off `tb_quizzes`:
```sql
SELECT COUNT(*) FROM tb_assessments a
 WHERE a.created_by IN (SELECT id FROM tb_users WHERE id = $1 OR owner_user_id = $1)
   AND a.visibility = 'public'
```

### A2. `api/src/http/export.rs`
- Rename `ExportResponse.quizzes` → `assessments` (struct field **and** the `Ok(Json(ExportResponse { ... }))` construction).
- Query (~L49–58): `SELECT * FROM tb_quizzes` → `SELECT * FROM tb_assessments`. Keep the
  `created_by IN (...)` filter and the `json_agg`/`COALESCE(...,'[]'::json)` wrapper.
- Rename the local `quizzes` var → `assessments`; fix the doc comment ("Returns quizzes, ...").

### A3. `api/src/http/admin.rs` (fn `moderate_quiz`, ~L315–349; `ModerateBody` ~L60–70)
- `SELECT id, title FROM tb_assessments WHERE id = $1`
- `UPDATE tb_assessments SET visibility = 'private' WHERE id = $1`
- `ModerateBody`: rename field `quiz_id` → `assessment_id` (serde camelCase ⇒ JSON `assessmentId`); update all `body.quiz_id` refs.
- `ApiError::NotFound { resource: "quiz" }` → `resource: "assessment"`.
- Audit entity `Some("quiz")` → `Some("assessment")`.
- Rename fn `moderate_quiz` → `moderate_assessment`; update route reg
  `.route("/v1/admin/moderate", post(moderate_quiz))` → `post(moderate_assessment)`.
  **Keep the path `/v1/admin/moderate` unchanged.**

### A4. Regenerate OpenAPI
```
make openapi > /tmp/ph4a-openapi.log 2>&1
```

### A5. Frontend consumers (update only if present)
```
grep -rn "\.quizzes\b" web/app web/api web/components web/hooks web/lib 2>/dev/null | grep -v ".next"
grep -rni "moderate" web 2>/dev/null | grep -v ".next" | grep -i quizid
```
Update only consumers of `ExportResponse.quizzes` or the moderate `quizId` body.
**Do NOT** touch the unrelated optional `quizId` fields in `exams/page.tsx` / `results/page.tsx`
(those are handled in Commit B's stale cleanup).

### A6. Verify
```
make check > /tmp/ph4a-check.log 2>&1
```
Gate: build + clippy clean, all tests pass, **`test_admin_flow_and_audit` green**,
`openapi_spec_is_current` green.

**Commit A message:** `refactor(api): migrate quota/export/moderate off legacy tb_quizzes onto tb_assessments`

---

## Commit B — drop legacy tables, collapse baseline churn, remove dead code

Includes the already-done Phase 3 change (drop `tb_messages.link_quiz_id`) which is currently
**unstaged** in the working tree (`api/src/http/messages.rs` + `baseline.sql:273`).

### B1. `db/migrations/20260601000000_baseline.sql` — remove legacy `CREATE TABLE`s
Delete these table definitions entirely (verified zero code readers after Commit A):
- `tb_quizzes` (~L124)
- `tb_quiz_questions` (~L136)
- `tb_exams` (~L147)
- `tb_exam_sections` (~L169) + its index `tb_exam_sections_exam` (~L183)
- `tb_quiz_item_stats` (~L311)

### B2. `baseline.sql` — collapse `tb_sessions` churn (declare-then-drop)
The CREATE TABLE declares legacy cols then later ALTER-drops them. Make the CREATE final:
- In `tb_sessions` CREATE (~L185–206): remove `exam_id` (L191), `quiz_id` (L192), and the
  `CONSTRAINT tb_sessions_kind_link` block (L202–206).
- Set `kind` default to `'assessment'` directly in the CREATE; clean the status check to
  `CHECK (status IN ('in_progress','finished','abandoned'))`.
- Remove the now-redundant ALTER section "remove legacy quiz_id and exam_id" (~L609–641):
  steps 1–5 (drop kind_link, drop FKs, drop quiz_id/exam_id cols), step 6 (kind default — now
  inline), step 7 (status check — now inline). **Keep** step at L600 (`ALTER TABLE tb_attempts
  ADD COLUMN correct_answer JSONB`) unless that col can be folded into the `tb_attempts` CREATE
  too — fold it if straightforward.

### B3. `baseline.sql` — remove backfill that references legacy tables
- The `UPDATE tb_sessions SET assessment_id = COALESCE(quiz_id, exam_id) ...` (~L596–598) — delete.
- Any assessment-section/assessment backfill blocks that `SELECT FROM tb_quizzes`/`tb_exams`
  (~L516–595 region) — delete. Fresh DBs have no legacy rows to backfill.
- The `ALTER TABLE tb_quizzes ADD COLUMN visibility ...` block (~L446–450) — delete (table is gone).
- Keep the `ALTER TABLE tb_assessments ADD COLUMN visibility ...` (~L647) **or** fold it into the
  `tb_assessments` CREATE (preferred — cleaner). If folded, make CHECK `('public','private')` and
  default `'private'`.

### B4. Fold in Phase 3 (already unstaged)
- `baseline.sql:273` (`link_quiz_id uuid REFERENCES tb_quizzes(id),` in `tb_messages`) — already removed; keep removed.
- `api/src/http/messages.rs` — field + INSERT already updated; keep.

### B5. Remove dead code
- `api/src/domain/exam.rs` — delete the file (the `Exam` struct at L67 is defined but never
  constructed/consumed; only a comment in `sessions.rs:470` mentions "exam"). Remove `pub mod exam;`
  from `api/src/domain/mod.rs:6`. If anything fails to compile, STOP and report — do not invent shims.
- `api/src/http/me.rs` cohort stats: remove the legacy `quiz_id` alias —
  `CohortStatsQuery.quiz_id` field (~L788), the `.or(q.quiz_id)` (~L827), and the `quizId` param in
  the `#[utoipa::path(... params(...))]` (~L813). Keep `assessment_id`/`assessmentId`.
- Stale FE optional fields: in `web/app/(learner)/results/page.tsx` `SessionSummary` (~L31) remove
  `quizId` and `examId` (keep `assessmentId`); in `web/app/(learner)/exams/page.tsx` `ExamSection`
  (~L28) remove `quizId`. Only remove if not referenced elsewhere in those files — grep first.

### B6. Regenerate OpenAPI (cohort quizId param removed)
```
make openapi > /tmp/ph4b-openapi.log 2>&1
```

### B7. Reset DB + verify (baseline was already applied → checksum mismatch without reset)
```
make db-reset > /tmp/ph4b-dbreset.log 2>&1
make check   > /tmp/ph4b-check.log 2>&1
```
Gate: `make db-reset` applies the rewritten baseline cleanly; `make check` fully green
(build, clippy, all unit + integration tests, `openapi_spec_is_current`).

### B8. Final stale sweep (report only — do not delete without confirmation)
After the above, grep the whole repo for stragglers and **list** anything left:
```
grep -rn "tb_quizzes\|tb_exams\|tb_quiz_questions\|tb_exam_sections\|tb_quiz_item_stats\|link_quiz_id" \
  api db web agents docs 2>/dev/null | grep -v ".next" | grep -v "docs/plans/2026-05-31-phase4"
```
Expected: only doc/spec/plan history references remain. Report the list; do not auto-delete docs.

**Commit B message:** `refactor(db): drop legacy quiz/exam tables and collapse baseline migration`

---

## Out of scope / flag for Haru (do NOT fix here)
- Possible `visibility` enum mismatch: Phase 2 notes mention an `Unlisted` variant in
  `AssessmentVisibility`, but baseline's `tb_assessments` CHECK only allows `('public','private')`.
  If Commit A/B touches this and the constraint rejects `unlisted`, STOP and report — it's the
  suspected `shares.rs` schema-mismatch bug, tracked separately.
- Do not collapse migrations other than the single `20260601000000_baseline.sql`.

## Definition of done
- [ ] Commit A: 3 endpoints on `tb_assessments`, OpenAPI regen, `make check` green
- [ ] Commit B: 5 legacy tables gone, baseline churn collapsed, dead code removed, Phase 3 folded
- [ ] `make db-reset` + `make check` green after Commit B
- [ ] Final grep sweep shows no legacy refs outside docs
