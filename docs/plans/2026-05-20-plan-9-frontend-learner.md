# Plan 9 (2026-05-20): Frontend learner path

**Goal:** Land the five learner-facing screens — Library, Quiz setup, Active Quiz, Results, Progress — backed by the typed OpenAPI client. Golden-path e2e (start session → answer → finish → see result) passes via `make validate`.

**Spec:** `docs/specs/2026-05-20-harus-platform-design.md` §9 (Screens) — specifically §9.1 (Library/Home), §9.2 (Quiz setup), §9.4 (Active Quiz), §9.5 (Results), §9.6 (Progress). §10.1 (Share modal + LearningObjectives cross-cutting components).

**Pixel-faithful reference:** `design/source/src/screen-*.jsx`. **Do not paraphrase** layout from spec §9 — defer to JSX.

**Prerequisites:** Plan 8 (agent surface + share API, since the Share modal is cross-cutting in §10.1).

**Module rule:** `web/` may consume `api/openapi.yaml` via codegen only. No handwritten Rust ↔ TS coupling.

---

## Task 1: Typed OpenAPI client

- [ ] **Step 1:** Generate a TypeScript client from `api/openapi.yaml` into `web/src/api/generated/`. Tool: `openapi-typescript` or equivalent.
- [ ] **Step 2:** Drift check — `make check` regenerates and fails if `web/src/api/generated/` is dirty.
- [ ] **Step 3:** Hand-written wrapper layer in `web/src/api/client.ts` that adds auth + error envelope handling.

## Task 2: Cross-cutting components (spec §10.1)

- [ ] **Step 1: `<ShareModal kind, id, defaults={}>`.** Three tabs: Link / Embed / Card. Calls `POST /shares`, renders the OG image. Hidden behind a feature flag if the OG worker isn't deployed yet.
- [ ] **Step 2: `<LearningObjectives objectives={string[]}>`.** Renders 3-4 bullets; soft-warning indicator (yellow chip) when `objectives.length > 6`.
- [ ] **Step 3:** Both components are storybook-driven; design tokens from `design/tokens.md` verbatim.

## Task 3: Library / Home (§9.2)

- [ ] **Step 1:** Lists quizzes for the learner's enrolled courses. Hero card shows `<LearningObjectives>` from the active quiz.
- [ ] **Step 2:** Quiz row includes Share trigger (opens `<ShareModal>`).
- [ ] **Step 3:** Data source: `GET /quizzes?course=<id>`.

## Task 4: Quiz setup (§9.2 form)

- [ ] **Step 1:** Form mirrors the practice-flavour `POST /sessions` body shape exactly (cats, types, count, etc).
- [ ] **Step 2:** Submitting POSTs to `/sessions`, navigates to Active Quiz with the returned `sessionId`.

## Task 5: Active Quiz (§9.4)

- [ ] **Step 1:** Renders the current question by kind. Each kind has its own renderer in `web/src/components/question/<kind>.tsx`.
- [ ] **Step 2:** Answer submission: `POST /sessions/{id}/answer` (idempotent on `(sessionId, questionId)`).
- [ ] **Step 3:** Timer (if `duration_min` is set). On expiry, navigate to Results regardless of unanswered items.
- [ ] **Step 4:** Finish button → `POST /sessions/{id}/finish` → navigate to Results.

## Task 6: Results (§9.5)

- [ ] **Step 1:** Header shows score + Share trigger (header-level share covers the whole attempt).
- [ ] **Step 2:** Per-item rendering with rubric note + per-item Share trigger (item-level share with explanation).
- [ ] **Step 3:** Data source: `GET /attempts/{id}`.

## Task 7: Progress (§9.6)

- [ ] **Step 1:** Per-tag mastery rollup. Donut + histogram fed by `GET /quizzes/{id}/stats` and `GET /me/attempts`.
- [ ] **Step 2:** Weakest-tags ribbon links to relevant practice setup pre-filtered.

## Task 8: Routing + auth

- [ ] **Step 1:** Cookie-session-gated routes. Anonymous users hit `/login` (form posts to API session endpoint).
- [ ] **Step 2:** `/embed/*` routes are unauthenticated and render `<EmbedView>` only.

## Task 9: Tests

- [ ] Storybook stories for `<ShareModal>` and `<LearningObjectives>`.
- [ ] Component tests per question kind renderer.
- [ ] e2e (Playwright via `make validate`): login → start session → answer all → finish → see result with score.
- [ ] Visual regression on Library + Results headers.

## Definition of done

- `make check` + `make validate` pass.
- Plan 9 ledger rows are `done`.
- Five screens are reachable + functional against a real API.
- Visual diff vs `design/source/src/screen-*.jsx` is within tolerance (review with designer).
