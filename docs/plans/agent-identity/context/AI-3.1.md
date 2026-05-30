# AI-3.1 — Quiz visibility column + drop anon attempts

**Phase:** 3 · **Lane:** db · **Agent:** haiku-developer · **Depends on:** AI-0.1
**Spec:** §"Schema deltas → Visibility" (quiz-only) + §"Cross-owner attempts"
**Paths (edit only here):** `db/migrations/`

## Objective
Add visibility to quizzes (the only shareable entity) and remove the
anonymous-attempt store (answering now requires an account).

## Do
Migration that:
```sql
ALTER TABLE tb_quizzes ADD COLUMN visibility text NOT NULL DEFAULT 'private'
  CHECK (visibility IN ('private','unlisted','public'));
CREATE INDEX idx_quizzes_public ON tb_quizzes (created_at DESC)
  WHERE visibility = 'public';
DROP TABLE IF EXISTS tb_anonymous_attempts;
```

## Gotchas
- **Quiz only** — do NOT add visibility to `tb_questions` or `tb_exams`
  (deferred by decision; questions ride inside their quiz).
- Dropping `tb_anonymous_attempts` will break `shares.rs` until AI-3.4 removes
  the code — that's expected; the two tasks land together in phase 3.

## Done when
- Column + index applied, anon table dropped; `make check` passes (once AI-3.4
  is also in for a green build of the whole phase).
