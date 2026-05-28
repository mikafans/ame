# Flashcard Mode — Design

**Date:** 2026-05-28
**Status:** Approved (build, then improvise)

## Goal

A lightweight self-review page where a learner draws a randomly-shuffled deck of
10 / 30 / 50 questions from the live bank, flips each card to reveal the answer +
explanation, self-rates confidence (Got it / Missed it), and sees a session
summary. No grading round-trip, no persistence.

## Scope

- **Pure frontend.** No API or DB changes. `GET /v1/questions` already returns
  full questions (incl. `payload` with correct answers and `explanation`) to any
  authenticated user — no instructor gate (`api/src/http/questions.rs:103`).
- New route: `web/app/(learner)/flashcards/page.tsx` (client component).
- New sidebar nav item under "Learn".

### Out of scope (improvise later)

Spaced repetition, saving results to the backend, quiz-scoped decks.

## Phase machine

Single page, `phase` state: `setup → review → summary`.

### Setup

Mirrors the Practice setup screen (`web/app/(learner)/practice/page.tsx`):

- Topic chips from `GET /v1/tags` — **single-select** (or "All topics"). The
  question list response carries no `tags` field, so topic filtering must happen
  server-side via the `?tag=` query param (one tag at a time).
- Question-type chips: mc / tf / short / essay / code — **multi-select**, optional
  (empty = all types). Each `Question` carries `kind`, so this is filtered
  client-side.
- Deck-size chips: **10 / 30 / 50**.
- "Start deck" button.

On start: `GET /v1/questions?status=live&limit=200` (+ `&tag=<topic>` if a topic
is selected). Filter the returned set by selected types client-side, shuffle
(Fisher–Yates), slice to N.

- Fewer than N available → use what exists, show a note ("Only M live questions
  match").
- Zero matches → inline MUI `Alert`, stay on setup.

### Review

One card at a time, progress shown as `n / total`.

**Front:** `kind` badge + `prompt`. For `code`, render the code snippet
(reuse `CodeRenderer` patterns). "Show answer" button; Space key also flips.

**Back:** derived answer + `explanation`:

| kind  | answer shown |
|-------|--------------|
| mc    | `options[correct_index]` |
| tf    | "True" / "False" (`correct: bool`) |
| short | `accepted` joined (", ") |
| essay | explanation only |
| code  | explanation only (+ snippet already on front) |

If a question has no `explanation` and no crisp answer (essay/code) →
"No model answer provided."

After flip: **Got it** / **Missed it** buttons (keys `1`/`2` or `←`/`→`).
Selecting advances to the next card; tally kept in memory.

### Summary

"You knew X of N." Buttons:

- **Review missed** — re-runs `review` with only the missed cards (re-shuffled).
  Disabled when none missed.
- **New deck** — back to `setup`.

## Components / files

- `web/app/(learner)/flashcards/page.tsx` — phase machine + setup + summary.
- A `Flashcard` review component (front/back flip, self-rate) — co-located or in
  `web/src/components/`.
- `web/src/components/Sidebar.tsx` — add `{ id: "flashcards", label: "Flashcards",
  icon: "flashcards", section: "Learn" }` + icon in `ICON_MAP`
  (e.g. `StyleOutlinedIcon`).
- `web/app/(learner)/layout.tsx` — add `flashcards: "/flashcards"` to `routeMap`
  and `getRouteId` (`/flashcards` → `"flashcards"`).

## Testing

- Unit/logic: shuffle + slice respects N and available count; answer-derivation
  per kind; missed-card re-deck.
- e2e (Playwright, optional first pass): setup → start → flip → rate → summary
  happy path. Follow the project's established auth-cookie pattern.

## Data shapes (reference)

- `McPayload`: `{ options: string[], correct_index: int }`
- `TfPayload`: `{ correct: bool }`
- `ShortPayload`: `{ accepted: string[], normalize, judge }`
- `Question`: `{ id, kind, prompt, payload, explanation, code_snippet, status, ... }`
