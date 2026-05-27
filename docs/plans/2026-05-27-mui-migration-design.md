# MUI Full-Adoption Migration + Makefile Cleanup

**Date:** 2026-05-27  
**Status:** Approved  
**Scope:** `web/` frontend + `Makefile`

---

## Overview

Replace all custom inline-style UI components and page layouts with MUI v6 primitives. Remove Tailwind CSS. Clean up Makefile targets. The result is a fully MUI-powered frontend with standard Material Design look-and-feel, plus a leaner Makefile with unambiguous stage names.

MUI packages (`@mui/material`, `@mui/icons-material`, `@emotion/react`, `@emotion/styled`) are already installed. `ThemeRegistry.tsx` and its wiring into `app/layout.tsx` are already in the working tree (uncommitted). This migration builds on that foundation.

---

## 1. Theme (`web/src/components/ThemeRegistry.tsx`)

Expand the existing `ThemeRegistry` into a full MUI theme:

- **Palette**: MUI default blue primary (`#1976d2`), white background, `#f5f5f5` paper.
- **Typography**: Roboto via `@fontsource/roboto` (add to `web/package.json` deps). Drop `next/font` Google Font imports from `app/layout.tsx`. Body fontSize 14px.
- **Shape**: `borderRadius: 8`.
- **Shadows**: MUI defaults (elevation-based).
- `CssBaseline` stays — it replaces the Tailwind reset.

`app/globals.css` is reduced to a near-empty file (no `@tailwind` directives, no CSS variable declarations — those move into the MUI theme object or are dropped).

---

## 2. Remove Tailwind

Uninstall from `web/package.json`:
- `tailwindcss`
- `postcss`
- `autoprefixer`

Delete (if present):
- `web/tailwind.config.*`
- `web/postcss.config.*`

Remove `@tailwind base/components/utilities` from `globals.css`.

---

## 3. Component Library (`web/src/components/ui/`)

Delete the entire directory. Replace at callsites:

| Old | MUI replacement |
|---|---|
| `Button` | `@mui/material/Button` |
| `Card` | `@mui/material/Card` + `CardContent` |
| `Tag` | `@mui/material/Chip` |
| `Stat` | `Box` + `Typography` (two-line block) |
| `KV` | `Box` + `Typography` |
| `Divider` | `@mui/material/Divider` |
| `Icon` | Import from `@mui/icons-material` directly at callsites |
| `Logo` | **Kept** — custom branded component, not replaced |

Update all `@/components/ui` import paths.

---

## 4. Page Layouts (15 files)

Replace all inline `style={{...}}` JSX with MUI layout primitives:

- **`Box`** — generic container (replaces `<div style=...>`)
- **`Stack`** — flex rows/columns with `spacing` prop
- **`Grid` / `Grid2`** — two-column login layout, stat grids
- **`Typography`** — all headings, labels, body text
- **`TextField`** — login form inputs (full controlled form)
- **`Tabs` + `Tab`** — library tab bar, exams tab bar
- **`CircularProgress`** — loading states
- **`Drawer` (permanent)**  + **`List` / `ListItemButton`** — learner sidebar (`app/(learner)/layout.tsx`)
- **`AppBar`** — top header if needed on any page

### Files in scope

```
app/layout.tsx
app/login/page.tsx
app/page.tsx
app/(learner)/layout.tsx
app/(learner)/library/page.tsx
app/(learner)/exams/page.tsx
app/(learner)/progress/page.tsx
app/(learner)/sessions/[id]/page.tsx
app/(learner)/sessions/[id]/results/page.tsx
app/(learner)/results/page.tsx
app/(learner)/plans/[id]/page.tsx
app/(learner)/quizzes/[id]/preview/page.tsx
app/(learner)/practice/page.tsx
app/(learner)/grading/page.tsx
app/(learner)/agent/page.tsx
app/(learner)/author/[quizId]/page.tsx
```

---

## 5. Makefile Cleanup

### Renames

| Old | New |
|---|---|
| `dev-env` | `dev` |
| `dev-stop` | `stop` |

### Dropped targets

| Target | Reason |
|---|---|
| `validate` | Replaced by `ci` |
| `pre-remote` | Replaced by `ci` |

### New target

```makefile
ci: check test-db e2e  ## Full CI gate: fmt + lint + unit + db tests + e2e
```

### Help doc strings

All targets must have a `## description` comment visible in `make help`. Any target currently missing one gets one added.

### `.PHONY` list

Updated to reflect all renames and additions.

---

## 6. Testing Strategy

Three Playwright checkpoints:

1. **Before** — captured at spec-write time (`/tmp/before-login.png`, `/tmp/before-library.png`, `/tmp/before-progress.png`). Already done.
2. **Mid** — after theme + component lib migration, before page layout migration. Spot-check: app loads, no crash, MUI CssBaseline active.
3. **After** — same three routes re-shot (`/tmp/after-*.png`). Visual diff reviewed by user.

**Regression gate:** `web/e2e/uiux-spec.spec.ts` must pass unchanged after the full migration (`make uiux`).

---

## 7. Out of Scope

- No API or backend changes.
- No new features or bug fixes — this is a pure UI infrastructure migration.
- The `web/e2e/` Playwright spec files are not modified (they are the regression gate, not migration targets).
- `web/src/api/` and `web/src/hooks/` are untouched.
