# AI-7.5 — Account / plan page

**Phase:** 7 · **Lane:** web · **Agent:** haiku-developer · **Depends on:** AI-4.1, AI-5.1
**Spec:** §"Tiers" + §"Export"
**Paths (edit only here):** `web/app/(learner)/account/`

## Objective
A simple account page showing the user's plan + quota usage, with the export
action.

## Do
- Show current `plan` (free/premium) and quota usage (agents used/limit, etc.)
  from the user/me endpoint.
- A "Export my data" button → `GET /v1/me/export`, enabled only for premium;
  for free, show it disabled with a short "premium only" note (no payment flow).

## Gotchas
- No upgrade/payment UI — premium is toggled by the operator (admin), so don't
  build a checkout. MUI only.
- Don't leave a placeholder export button for free users that does nothing —
  disabled-with-reason is fine; a silent no-op is not.

## Done when
- Plan + usage render; export works for premium, clearly gated for free;
  `make e2e` passes.
