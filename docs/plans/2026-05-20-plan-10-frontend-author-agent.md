# Plan 10 (2026-05-20): Frontend author + agent

**Goal:** Land the three remaining screens — Author studio, Exams, Agent integration — plus the role-gated nav and the demo-only Tweaks panel. Closeout for the redesign.

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §9.3 (Exams), §9.7 (Author studio), §9.8 (Agent integration), §10.2 (Tweaks panel + agent sidebar role-gating).

**Pixel-faithful reference:** `design/source/src/screen-author.jsx`, `screen-exams.jsx`, `screen-agent.jsx`.

**Prerequisites:** Plan 9 (learner frontend + shared infrastructure).

---

## Task 1: Author studio (§9.7)

Three-pane layout: left = questions list (sortable), center = active question editor, right = distribution + rubric + activity.

- [ ] **Step 1: Questions list pane.** Reordering posts `PATCH /quizzes/{id}` with the new `quiz_questions.order_index` sequence.
- [ ] **Step 2: Per-kind editor.** One editor component per kind (mc, tf, short, essay, code). Validates payload client-side mirroring `validate_payload` from Plan 3.
- [ ] **Step 3: Per-question points input.** Integer, minimum 0, default 1.
- [ ] **Step 4: Objectives editor.** `<LearningObjectives>` (from Plan 9) in editable mode. Soft-warn UI when length > 6 — matches the server's `warnings[]` response.
- [ ] **Step 5: Generate-merge prompt.** "Generate questions" button calls `POST /quizzes/{id}/generate-questions` (or the equivalent — confirm exact path with spec §6.1). Returns candidates which the author can merge into the quiz one at a time.
- [ ] **Step 6: Publish gating UI.** Disable the "Publish" button (would `PATCH /quizzes/{id}` with `status: 'active'`) if the quiz fails Plan 4's publish gating client-side (zero questions, any draft question).

## Task 2: Exams screen (§9.3)

- [ ] **Step 1:** Two-pane layout. List on the left, detail on the right. Detail uses `GET /exams/{id}` and shows the composition trace (instructor view only — learners never see this pane).
- [ ] **Step 2: Compose flow.** Multi-step form mirrors `POST /exams` body. Section editor supports static + dynamic.
- [ ] **Step 3: Pool insufficiency.** When the compose POST returns 422 `pool_insufficient`, surface the failing constraint inline on the offending section.
- [ ] **Step 4: Objectives editor.** Same `<LearningObjectives>` editable component as Author studio.
- [ ] **Step 5: Share modal trigger.** Per §9.3, the Share button is **near the CTA** (top right of detail pane).

## Task 3: Agent integration (§9.8)

- [ ] **Step 1: API keys panel.** Calls `/me/keys` for list + create + rotate + revoke.
- [ ] **Step 2: Scope chips.** Renders the persisted scopes; admin chips visible only to admins. May render `*` shorthand for "all current scopes" (spec §6.2 display shorthand — never stored).
- [ ] **Step 3: MCP descriptor preview.** Calls `GET /agents/mcp.json` and renders it verbatim (read-only formatted JSON viewer). Reference: `design/source/src/screen-agent.jsx` `mcpDescriptor`.
- [ ] **Step 4: Webhooks panel.** List + create + delete. Secret shown once on create (modal — confirm copy before dismiss).
- [ ] **Step 5: Bootstrap helper.** UI helper that mints an agent via `POST /agents/register` and copies the resulting `apiKey` to clipboard once.

## Task 4: Role-gated nav (§10.2)

- [ ] **Step 1:** Sidebar shows the Agent integration entry only for `role ∈ ('instructor', 'admin')`. Learners never see it.
- [ ] **Step 2:** Author studio nav entry: `instructor` and `admin` only.
- [ ] **Step 3:** Exams compose: `instructor` and `admin`. Attempt mode: any authenticated user with an active exam invite.

## Task 5: Tweaks panel (demo-only, §10.2)

- [ ] **Step 1:** Build the Tweaks panel for the demo / Storybook environment only.
- [ ] **Step 2:** Gate behind a `NEXT_PUBLIC_DEMO_MODE` env flag that is **never set in production**.
- [ ] **Step 3:** Add a runtime assertion that throws (loudly) if the panel mounts with `NODE_ENV === 'production'`.

## Task 6: Archive cleanup

- [ ] **Step 1:** Once Plan 10 is `done`, move May-19 plan markdowns (`docs/plans/2026-05-19-plan-*.md`) into `docs/_archive/` via `git mv` so the canonical plan set is unambiguous.
- [ ] **Step 2:** Update CLAUDE.md "When in doubt" to drop the "May-19 spec is historical" caveat once the archive move is committed.

## Task 7: Tests

- [ ] Per-kind editor: round-trip a payload through create → patch → read.
- [ ] Publish gating: button disabled in expected states.
- [ ] Pool insufficient: 422 surfaces inline.
- [ ] MCP descriptor view renders without errors against a real `/agents/mcp.json`.
- [ ] Role-gated nav: learner cannot navigate to `/author/*` or `/exams/compose`.
- [ ] Tweaks panel assertion: production build excludes the panel entirely (bundle size check).

## Definition of done

- `make check` + `make validate` pass on a full build (`NEXT_PUBLIC_DEMO_MODE` unset).
- Plan 10 ledger rows are `done`.
- All eight screens from spec §9 are reachable and functional.
- Redesign closeout: `docs/_archive/` move committed; `CLAUDE.md` caveat removed.
