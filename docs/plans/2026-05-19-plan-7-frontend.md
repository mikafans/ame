# Plan 7: Web Frontend

**Goal:** Build the Next.js 15 UI with Tailwind v3 and shadcn/ui.

**Prerequisites:** Plans 1-6 completed. API must be fully functional.

---

## Task 1: Frontend Infrastructure

- [ ] **Step 1: OpenAPI Client**
  Add `openapi-typescript` to `web/package.json`.
  Create a script to generate `web/lib/api/schema.d.ts` from `api/openapi.yaml`.
  Write a thin typed fetch wrapper leveraging the generated types.
- [ ] **Step 2: shadcn/ui initialization**
  Run `bunx --bun shadcn-ui@latest init` configured for Tailwind CSS v3.
  Add base components: button, card, input, dialog, form.

## Task 2: Taking Screens (Quiz & Exam)

- [ ] **Step 1: Question Renderers**
  Implement `Markdown` component using `react-markdown` + `shiki`.
  Implement `MCQRenderer`, `FreeTextRenderer`, `ClozeRenderer`.
- [ ] **Step 2: State Management**
  Set up `zustand` store for active session state to persist through unmounts/renders.
- [ ] **Step 3: Taking Pages**
  `app/quiz/[sessionId]/page.tsx`
  `app/exams/[examId]/session/[sessionId]/page.tsx`

## Task 3: Dashboard & Browsing

- [ ] **Step 1: Dashboard (`app/page.tsx`)**
  Fetch and display weakest tags, recent attempts.
- [ ] **Step 2: Exam & Tag Lists**
  `app/exams/page.tsx`, `app/tags/page.tsx`.

## Task 4: Admin Panel

- [ ] **Step 1: Admin Layout**
  `app/admin/layout.tsx` (ensure gated UI elements).
- [ ] **Step 2: Question & Exam Management**
  Bank browser, manual create form, dynamic blueprint editor.

## Task 5: Testing & QA

- [ ] **Step 1: Zod Parity Tests**
  Ensure Zod schemas match OpenAPI schemas.
- [ ] **Step 2: Playwright E2E**
  Write e2e tests covering the golden path: login -> take quiz -> see deltas.
- [ ] **Step 3: `make validate`**
  Ensure `make check` and `make e2e` run flawlessly.
