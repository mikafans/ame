# AI-7.3 — Quiz visibility controls + share dialog

**Phase:** 7 · **Lane:** web · **Agent:** sonnet · **Depends on:** AI-3.2
**Spec:** §"Visibility / listing / sharing"
**Paths (edit only here):** `web/app/(learner)/quizzes/`

## Objective
Let an owner set a quiz's visibility and produce a share link.

## Do
- A visibility selector (private / unlisted / public) on the quiz, calling
  `PATCH /v1/quizzes/{id}`. If publishing to `public` is plan-blocked, surface
  the 429/403 message cleanly (don't swallow it).
- A share dialog: shows the canonical `/quizzes/{id}` link (primary path) and
  the presentation toggles (include score / explanation / attribution) backed by
  `POST /v1/shares`. The existing ShareModal sends `id` (camelCase) — reuse it.

## Read first
- The current quiz library/detail page + the existing ShareModal component.

## Gotchas
- MUI only; no `window.confirm`; remove any placeholder buttons.
- "Unlisted" is the share-with-friends default; make that the easy path, with
  "public" a deliberate extra step (it's plan-gated + moderatable).

## Done when
- Visibility changes persist; share dialog yields a working link + flags;
  `make e2e` passes.
