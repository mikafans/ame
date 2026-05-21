## Status
DONE

## Files modified
- web/app/(learner)/progress/page.tsx — Complete rewrite of Progress dashboard screen per design spec
  - Added window state management (4w, 12w, all)
  - Integrated `/v1/me/stats` fetch with window parameter
  - Integrated `/v1/me/attempts?limit=200` for chart data
  - Implemented page header with kicker, serif title, and range selector button group
  - Built 5-stat strip (AVG SCORE, ATTEMPTS, HOURS SPENT, CURRENT STREAK, MASTERED TOPICS)
  - Implemented score trend line chart (custom SVG with weekly averages)
  - Implemented by-subject bar chart (horizontal bars with tag labels, top 6 by count)
  - Added loading/empty states with proper fallbacks

## make check
PASS (cd web && mise exec -- bun run type-check 2>&1 | tail -10):
```
$ tsc --noEmit
```
No errors.

## Next
Progress dashboard is ready for testing. The page follows the design spec with all required components, proper state management, and correct API integrations.
