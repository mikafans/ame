# Plan: Unify browsing into Explore

> Status: PLANNED — to be implemented by gemini-developer on a branch off `main`.

## Goal

Replace the two flat, latest-N lists (`/library`, `/exams`) with **one scalable
Explore surface** that handles both practice assessments and exams. No
human-facing authoring UI — creation/composition stays on the Agent API.

## Non-goals

- No Studio / consolidated authoring page.
- Don't delete `/library` or `/exams` page files — keep them as functional
  deep-link targets, just remove them from the sidebar.
- No changes to `/author/{id}`, grading, or the agent surface.

## Background

- `/v1/explore` is the only browse endpoint built to scale (cursor pagination +
  search). `/v1/assessments` (used by `/library` and `/exams`) returns flat,
  unpaginated lists — useless past ~50 items, and we now mint ~5.5k items/user.
- `/v1/explore`'s `kind` param is misnamed: it filters `status` (active/draft),
  **not** practice-vs-exam. `tb_assessments` has a `mode` column that explore
  currently ignores.
- The tag filter matches the `objectives` array, not the `tb_tags` table — so a
  tag autocomplete must be sourced from distinct `objectives`, or the options
  won't match what the filter queries.

## Backend — `api/`

1. **`api/src/http/explore.rs` — add a real type filter + expose mode**
   - Add `mode: Option<String>` to `ExploreQuery` (`practice` | `graded`). Keep
     the existing `kind`→status filter unchanged.
   - When `mode` is present: `AND mode = $n`.
   - Add `mode` to the `SELECT` and to each JSON item: `"mode": row.get::<String, _>("mode")`.

2. **New endpoint `GET /v1/explore/facets`** (same file) — owner-scoped, powers
   the type-selector counts and the tag autocomplete:
   ```json
   { "tags": ["...distinct objectives..."], "counts": { "practice": 5000, "graded": 500 } }
   ```
   - `counts`: `SELECT mode, count(*) FROM tb_assessments WHERE <owner-scope> GROUP BY mode`
   - `tags`: `SELECT DISTINCT unnest(objectives) AS t FROM tb_assessments WHERE <owner-scope> ORDER BY t` (cap ~200)
   - Reuse the exact owner-scope `WHERE` clause already in `explore`
     (created_by OR sub-account owner).

3. Register the new path in `api/src/http/openapi.rs`, then run `make openapi`
   to regenerate `api/openapi.yaml` + the TS types.

## Frontend — `web/`

4. **`src/components/Sidebar.tsx` + `app/(learner)/layout.tsx` — new IA**
   - Sidebar `items`: remove `library` and `exams`. Add Explore to a new top
     section:
     - `BROWSE → Explore`
     - `LEARN → Take assessment, Flashcards, Question bank, Last results, Progress`
     - `TEACH → Author studio, Grading` (unchanged)
     - `INTEGRATE → Agent API` (unchanged)
   - `layout.tsx`: keep `/library` + `/exams` in `getRouteId`/`routeMap` (still
     reachable) but with no nav entry. Change the **default fallback** from
     `/library` to `/explore` in both `getRouteId()` and `routeMap[...] || "/explore"`.

5. **`app/(learner)/explore/page.tsx` — rebuild as the primary browser**
   Keep the cursor "Load more" pattern. Add above the table:
   - **Type selector** — `ToggleButtonGroup`: `All | Practice | Exam`, wired to
     the `mode` query param. Show counts from `/v1/explore/facets`:
     `Practice (5000)` / `Exam (500)`. (This is the count the exam tabs never had.)
   - **Tag selector** — replace the free-text tags `TextField` with MUI
     `Autocomplete` (`multiple`, `freeSolo`), options seeded from `facets.tags`.
     Join selected values comma-separated into the existing `tags` param
     (matches the `objectives &&` filter).
   - Keep the search box.
   - **Row action adapts by `item.mode`:**
     - `practice` → `Preview` (`/assessments/{id}/preview`) + `Start`
     - `graded` → `Open` (`/exams` deep link or `/assessments/{id}/preview`)
     - `draft` status → `Edit` (`/author/{id}`)
   - Add a **Type** column rendering a chip from `item.mode`.
   - Reset list + refetch when type/tags/search change (existing `applyFilters`).

6. **Default-destination redirects: `/library` → `/explore`**
   - `web/middleware.ts`: the `hasToken && pathname === "/login"` redirect target.
   - `web/app/login/page.tsx`: the `dest` fallback.
   - `web/app/page.tsx`: the signed-in CTA href (relabel "Open Library" → "Open Explore").

7. *(Optional)* `app/(learner)/exams/page.tsx` tab counts — superseded by the
   Explore selector counts now that `/exams` is de-navved. Skip unless `/exams`
   stays a first-class destination.

## Files touched

```
api/src/http/explore.rs        (mode filter, mode field, facets endpoint)
api/src/http/openapi.rs        (register facets path)
api/openapi.yaml               (regenerated — make openapi)
web/src/components/Sidebar.tsx (IA)
web/app/(learner)/layout.tsx   (IA + default route)
web/app/(learner)/explore/page.tsx  (rebuild)
web/middleware.ts              (/library → /explore)
web/app/login/page.tsx         (/library → /explore)
web/app/page.tsx               (CTA → /explore)
```

## Verification

- `make openapi` clean, then `make check` green (cargo fmt/clippy + tsc + prettier).
- Manual on running stack (`make dev`): Explore loads; type-selector counts match
  (~5000 / ~500 for Ada); filtering by type + tag + search works; "Load more"
  paginates; practice rows Start, exam rows Open, drafts Edit; login lands on
  `/explore`.
