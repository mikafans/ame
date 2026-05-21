## Status
DONE

## Files modified
- `/web/app/(learner)/author/[quizId]/page.tsx` (complete rebuild following design spec)

## Changes summary

### Implementation
1. **Page header** — kicker "EDITING DRAFT · {course} · AUTOSAVED", serif h1 "Author studio", action buttons (Import ghost, Preview ghost, Save draft outline, Publish primary)
2. **Metadata row** — 5 inline fields in grid (Title, Course, Duration, Difficulty, Attempts) with auto-save on blur
3. **Validation status bar** — 4 computed chips:
   - Outline complete check (title non-empty + at least 1 question)
   - Questions ready status (all have points + prompt, or N need review)
   - Total count + points summary
   - Last edit timestamp + "by you"
4. **3-pane layout** (320px | flex | 280px):
   - **LEFT**: Questions list with "Add" button, question rows showing Q-number, truncated prompt (40 chars), type label, points badge
   - **MIDDLE**: Question editor with prompt textarea (serif), MC options preview, points/tag/difficulty inputs, explanation textarea
   - **RIGHT**: Distribution KV, Rubric criteria block, Recent activity feed with 3 static entries
5. **Data flow** — fetch quiz + questions on mount, auto-save on question blur, add question via POST endpoint, publish via PATCH status=active

### Components used
- Button, Card, Icon, KV, Tag from @/components/ui
- useAuth, makeClient from hooks/api
- Serif font for headers and prompts (via var(--serif))

### Helper functions
- `kindLabel()` — map question kind to short label
- `getMinutesAgo()` — format timestamp as "Nm ago", "Nh ago", etc.
- `McOptionsEditor()` — read-only display of MC options
- `ActivityEntry()` — single activity log row

## Type-check
PASS (author page compiles without errors; pre-existing error in exams/page.tsx unrelated to this task)

## Next
Author studio page is complete per spec. Ready for PR review and e2e testing against actual API endpoints.
