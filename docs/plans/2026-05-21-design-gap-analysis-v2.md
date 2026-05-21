# Design → App Gap Analysis v2 — 2026-05-21

**Sources combined:**
- Screenshots: `design/screenshots/*.png` (20 images, captured 2026-05-21)
- Design source: `design/source/src/*.jsx`
- Prior plan: `docs/plans/2026-05-21-design-catchup.md` (v1)
- Spec: `docs/specs/2026-05-20-harus-platform-design.md`
- Live app: `web/` (Next.js 15, Rust/Axum backend)

**Verdict:** v1 plan is structurally correct but screenshots reveal 6 specification updates and 2 net-new gaps not covered. Supersedes the gap summary in v1; task list in v1 is still valid for F1–F13 with amendments noted per gap below.

---

## Gap Summary (updated)

| # | Area | Gap | Status vs v1 |
|---|------|-----|-------------|
| B1 | Auth | No email+**password** auth — current app uses API-key paste; design has full email+password form | UPDATED: password field confirmed by `screen-signup.jsx:92` and all screenshots |
| B2 | Backend | `POST /v1/quizzes` missing | unchanged |
| B3 | Backend | `GET /v1/me/stats` missing — specific shape now known (5-stat strip) | UPDATED: shape spec below |
| B4 | Backend | `GET /v1/quizzes/count` missing — quiz setup needs live "N ITEMS MATCH" badge | NEW |
| F1 | CSS | `--serif`/`--sans`/`--radius-lg` vars absent; Paper+Cobalt dim variants missing | unchanged |
| F2 | Fonts | Source Serif 4, Inter, JetBrains Mono not loaded | unchanged |
| F3 | Atoms | No shared Button/Tag/Card/Icon/Logo/KV/Stat/Divider | unchanged |
| F4 | Shell | Sidebar icons, Logo, footer avatar, "v2.4" sub-header, bell+search topbar icons | UPDATED: "Agent API" label already correct in live app |
| F5 | Auth UI | Login is "paste API key" — design is 2-column marketing+email+password form | UPDATED: password, 4-stat grid, compliance footer, SSO+code buttons, agent callout |
| F6 | Library | Tab labels wrong; no hero "Up next"; no cohort context rail; no "+ New quiz" | UPDATED: tab labels, cohort panel, and New quiz button confirmed |
| F7 | Quiz | No setup screen; no live count badge; no session summary rail | UPDATED: "N ITEMS MATCH" counter and session summary rail confirmed |
| F8 | Quiz | Active quiz missing type support; palette rail; integrity panel; allowed materials | UPDATED: integrity checks + allowed materials panels confirmed |
| F9 | Results | Missing donut, cohort histogram, "Export PDF" + "Share quiz" header buttons | UPDATED: Export PDF button added |
| F10 | Progress | Needs 5-stat strip with deltas, range selector, Export, line+bar charts | UPDATED: 12-week selector + Export button + specific stat shapes confirmed |
| F11 | Exams | No split list/detail; wrong tabs; no "+ Compose exam" | UPDATED: exams tabs All/Active/Scheduled/Drafts + Compose exam button confirmed |
| F12 | Author | 3-pane editor + status bar not implemented | UPDATED: validation status bar confirmed |
| F13 | Agent | No landing intro view; no OpenAPI/MCP manifest/New API key header actions | UPDATED: landing view confirmed in screenshots 10 and 11 |

---

## Detailed Gaps

### B1 — Auth: email + password (UPDATED)

**Design:** `screen-signup.jsx:92-94` — explicit `<Field label="Password"><input type="password" /></Field>`. Screenshots `01-signup.png` and `15-library-paper-theme.png` both show the password field clearly.

Full form shape:
```
POST /v1/auth/register  { email, password, name, role: 'learner'|'instructor'|'agent' }
POST /v1/auth/login     { email, password } → { token, user }
```

Additional UI elements confirmed in screenshots:
- Email field has institution-detection hint: "Recognized: stanford.edu · SSO available"
- Below form: "Continue with SSO" + "Use access code" ghost buttons
- Below everything: agent shortcut callout box with `POST /v1/agents/register` hint
- Left panel: 4-stat grid (18,402 active learners / 1,243 instructors / 94 institutions / 6.1M graded attempts)
- Left footer: "SSO · SAML · FERPA · GDPR · OpenAPI 3.1 · MCP" compliance row

**Live app:** `web/app/login/page.tsx` — single-column "Paste your API key" form. Completely wrong.

**v1 plan task:** Task 1 (B1 auth) — step spec is email-only. Update to email+password; add the DB `password_hash` column.

---

### B3 — GET /v1/me/stats shape (UPDATED)

Screenshot `08-progress-dashboard.png` confirms the exact shape the stats endpoint must return:

```json
{
  "avg_score":       0.768,
  "avg_score_delta": +0.092,          // vs prior 12w
  "attempts_total":  49,
  "attempts_this_week": 12,
  "hours_spent":     31.4,
  "hours_spent_delta": -2.1,          // vs cohort avg
  "current_streak":  11,              // days
  "best_streak":     18,
  "mastered_topics": 18,
  "mastered_topics_total": 27,
  "mastered_topics_delta_since": "W8" // label string
}
```

The endpoint also needs a `?window=12w|4w|all` query param (the range selector button in the UI).

**v1 plan task:** Task 2 (B3) needs shape spec. Add the above JSON contract.

---

### B4 — GET /v1/quizzes/count (NEW)

Screenshot `04-quiz-setup.png` shows a "30 ITEMS MATCH" badge that updates as the user adjusts categories/tags/difficulty/types. This requires a lightweight count endpoint:

```
GET /v1/quizzes/count?cats=algorithms,math&tags=bfs,dfs&diff=inter&types=mc,short
→ { count: 30 }
```

No auth required beyond the session token. Response must be fast (<100ms) — it's called on every filter change.

**v1 plan:** Not present. New task needed (see task list below).

---

### F4 — Shell: sidebar + topbar (UPDATED)

**Confirmed in design source `sidebar.jsx`:**
- `sidebar.jsx:9` — sidebar entry is `{ id: "agent", label: "Agent API", ... }` → live app already uses "Agent API" ✓
- `sidebar.jsx:25` — "Assessment Platform · v2.4" subtitle below logo
- `sidebar.jsx:71-84` — footer avatar: initials circle "JT" + "Jordan Tahir / Student · CS '27" + gear icon
- `sidebar.jsx:108-113` — Topbar right side: bell icon + search icon (separated by a vertical border from any page actions)

**Current live app:** `web/app/(learner)/layout.tsx` NAV items match labels correctly; but no icons, no logo, no footer avatar, no version string, no bell/search in topbar.

---

### F5 — Auth UI: full two-column signup (UPDATED)

In addition to what v1 described, screenshots confirm:
1. **Paper theme works on signup** (`15-library-paper-theme.png` is actually the signup form in paper theme — confirms the form must respect `data-theme`).
2. **No password confirm field** — single password input (not confirm-password).
3. **No email-domain SSO API call needed for MVP** — the "Recognized: stanford.edu · SSO available" hint is hardcoded in the design (demo only).

---

### F6 — Library: tabs, cohort rail, New quiz button (UPDATED)

**Screenshots `02-library.png` confirms:**

Tabs (with counts): `All quizzes (34)` / `Assigned to me (4)` / `Completed (11)` / `Drafts (3)`

**Current app tabs:** `all` / `active` / `archived` — wrong. Map needed:

| Design tab | API query |
|------------|-----------|
| All quizzes | `status=active` (all accessible) |
| Assigned to me | `assigned=true` |
| Completed | `completed=true` |
| Drafts | `status=draft&authored_by=me` |

**Cohort context right panel** (on the hero "Up next" card):
- Class average: 68.5%
- Class completion: 71/86
- Hardest item: Q4 — iterative DFS
- Topic mastery: Intermediate
- Your last related score: 74.0%

This panel maps to `GET /v1/quizzes/{id}/cohort-stats` or inline in `GET /v1/quizzes?assigned=true` response.

**"+ New quiz" button**: Visible in screenshot. Design shows it top-right of library content area. Should be instructor-only (role-gated) but visible to learners as "greyed out" or hidden. The design shows it active, suggesting the demo user has author rights.

---

### F7 — Quiz setup: live count + session summary rail (UPDATED)

Screenshot `04-quiz-setup.png` confirms:
- "**30 ITEMS MATCH**" live counter badge (top-right of configurator) — needs B4 endpoint
- Session summary right rail with: quiz title, mode (Practice · untimed), Questions count, Pool size, Difficulty range, Types list (mc, short, code), Duration
- Numbered blocks: ①Session mode → ②Categories → ③Tags → ④Difficulty → ⑤Question types → ⑥Length → ⑦Source → ⑧Options

---

### F8 — Active quiz: integrity panel + allowed materials (UPDATED)

Screenshot `05-quiz-active-mc.png` confirms the right rail has two sections below the question palette:

**INTEGRITY section:**
- Browser locked ✓
- Single tab session ✓  
- Autosave every 8 s ✓

**ALLOWED section:**
- One sheet of notes (any)
- Class textbook (printed)
- Standard calculator

These are exam-level configuration fields. The `Exam` entity needs `integrity_rules: string[]` and `allowed_materials: string[]` fields (or the session config carries them).

Also confirmed: question header format is `QUESTION N OF N · TYPE · N PTS` + "Flag for review" link. The `N PTS` requires per-question `points` field (already in spec §4.5 from prior gap analysis).

---

### F9 — Results: Export PDF + cohort histogram (UPDATED)

Screenshot `07-results.png` confirms header has TWO action buttons: **Export PDF** and **Share quiz**.

- Export PDF → `GET /v1/sessions/{id}/export.pdf` (new endpoint, not in spec)
- Share quiz → existing share modal

Score card structure: donut (73%) + score fraction (11/15 pts) + PASS badge + "ABOVE CLASS AVG" badge + AI-generated score summary text.

---

### F10 — Progress: range selector + Export (UPDATED)

Screenshot `08-progress-dashboard.png` confirms:
- **Range selector**: "12 weeks" button (likely dropdown: 4w / 12w / all)
- **Export button**: Downloads the progress data
- **Kicker**: "SPRING 2026 · LAST 12 WEEKS"
- Stats: AVG SCORE / ATTEMPTS / HOURS SPENT / CURRENT STREAK / MASTERED TOPICS (5 up)
- Charts: "Score trend — Weekly rolling average across all subjects" (line chart) + "By subject — Average score, last 12 weeks" (horizontal bar chart)

The `?window=12w` param (from B3 update) feeds the range selector.

---

### F11 — Exams: tabs + Compose button (UPDATED)

Screenshot `03-exams.png` confirms:
- Tabs: `All (4)` / `Active (2)` / `Scheduled (1)` / `Drafts (1)`
- Header action: **"+ Compose exam"** button (instructor-only)
- Kicker: "COMPOSED ASSESSMENTS · MULTI-QUIZ · WEIGHTED"
- Each list card shows: course/title/status tag, duration (120m), question count (3 sec), method (manual/agent icon)
- Right detail panel: status tags + full title + description text (scrolls)

---

### F12 — Author studio: validation status bar (UPDATED)

Screenshot `09-author-studio.png` confirms a status bar between the header metadata row and the questions pane:

```
✓ Outline complete  |  ✗ 2 questions need review  |  ≈ 5 questions · 15 pts  |  Last edit · 4 minutes ago · by you
```

These are computed fields, not stored — derived from question state. Rules:
- "Outline complete" = title + course + at least 1 question
- "N questions need review" = questions with `status='draft'` or `points=null`
- "N questions · N pts" = count and sum of `points`
- "Last edit" = from quiz `updated_at`

---

### F13 — Agent: landing intro view (UPDATED)

Screenshots `10-agent-api-keys.png` and `11-agent-mcp-tools.png` both show the same landing/intro view at the top of the Agent integration screen (confirmed: this is the state of the default view before tabs are clicked, or it IS the content above the tabs):

**Header area:**
- Kicker: "PROGRAMMATIC SURFACE · OPENAPI 3.1 · MCP-COMPATIBLE"
- Title: "Agent integration"
- Three action buttons: **OpenAPI** (download), **MCP manifest** (download), **New API key**

**Hero card (below header):**
- Accent chip: "TWO INTERFACES, ONE MODEL"
- Serif headline: "Quizzes, attempts, and rubrics are first-class API objects."
- Body paragraph explaining dual learner/agent surface
- Right column: "HELLO, WORLD" curl example card with copy button, showing `curl https://api.harus.app/v1/quizzes \ -H "Authorization: Bearer hk_live_3fY9…ax2P"` → `→ 200 OK · 24 quizzes`

This landing view appears to be **above** or **replacing** the tab bar on the Agent screen's default state. The current app (`web/app/(learner)/agent/page.tsx`) starts directly at tabs.

---

## What is ALREADY correct in the live app

- Sidebar label "Agent API" ✓ (`layout.tsx` NAV)
- `ShareModal.tsx` and `LearningObjectives.tsx` components exist ✓
- Agent page has 4-tab structure (keys/tools/import/activity) ✓
- Library page imports `LearningObjectives` and `ShareModal` ✓
- Auth hook (`useAuth.ts`) and client (`client.ts`) exist ✓
- Theme `data-theme` attribute switching ✓ (layout.tsx)
- Role-gating for Author/Agent routes ✓ (layout.tsx roles check)

---

## How-to-Fill Plan (task list)

Tasks below amend/extend the v1 plan. Reference tasks in `docs/plans/2026-05-21-design-catchup.md` by their heading for detailed step specs.

### New / Updated Backend Tasks

**Task B1 (UPDATE):** Auth — email + password
- Add `password_hash TEXT NOT NULL` to `users` migration (not just `email`)
- `POST /v1/auth/register` body: `{ email, name, password, role }`
- `POST /v1/auth/login` body: `{ email, password }` → `{ token, user { id, name, email, role } }`
- Hash with `argon2` crate; return opaque bearer token (store in `api_keys` with `is_session=true`)
- Files: `db/migrations/20260521100000_auth_email.sql`, `api/src/http/auth.rs`, `api/src/http/mod.rs`

**Task B3 (UPDATE):** Stats endpoint — specific shape
- `GET /v1/me/stats?window=12w` response: see JSON contract in B3 section above
- Compute from `sessions` + `attempt_items` + `user_tag_deltas` tables
- `window` param: `4w` | `12w` | `all` (default `12w`)
- Files: `api/src/http/me.rs`

**Task B4 (NEW):** Quiz count endpoint
- `GET /v1/quizzes/count?cats=&tags=&diff=&types=` → `{ count: N }`
- Re-uses the same filter logic as `GET /v1/quizzes` but returns only the count
- No pagination, no body — must be fast (single `SELECT COUNT(*)`)
- Files: `api/src/http/quizzes.rs`

### Frontend Tasks (amendments to v1 headings)

**Task F5 (AMEND):** Auth UI — add to v1 Task 5:
- Password field (single, no confirm)
- 4-stat grid in left panel (static values acceptable for MVP)
- "Continue with SSO" + "Use access code" ghost buttons (no-op for MVP)
- Agent shortcut callout box with `POST /v1/agents/register` code hint
- Compliance footer row

**Task F6 (AMEND):** Library — add to v1 Task 6:
- Fix tab labels: `all→"All quizzes"`, `active→"Assigned to me"`, `archived→"Completed"`, add `"Drafts"` tab
- Add cohort context right panel to hero card (fetch from `GET /v1/quizzes/{id}/cohort-stats` or mock for MVP)
- Add "+ New quiz" button (role-gated: instructor/admin only; links to `/author/new`)

**Task F7 (AMEND):** Quiz setup — add to v1 Task 7:
- Wire live "N ITEMS MATCH" counter using B4 endpoint (debounced 300ms on filter change)
- Session summary right rail (client-side computed from current filter state)

**Task F8 (AMEND):** Active quiz — add to v1 Task 8:
- Right rail: Integrity panel (browser-locked / single-tab / autosave interval) — values from session config
- Right rail: Allowed materials list — values from exam config (empty/hidden for practice sessions)
- Question header format: `QUESTION N OF N · TYPE · N PTS` + "Flag for review" link

**Task F9 (AMEND):** Results — add to v1 Task 9:
- "Export PDF" button → `GET /v1/sessions/{id}/export.pdf` (stub that returns 501 for MVP is acceptable)
- "ABOVE/BELOW CLASS AVG" badge next to PASS/FAIL

**Task F10 (AMEND):** Progress — add to v1 Task 10:
- Time range selector: "12 weeks" button (dropdown: 4w / 12w / all) → passes `?window=` to B3 endpoint
- "Export" button (stub 501 for MVP)
- Use `?window=12w` as default; re-fetch stats on selector change

**Task F11 (AMEND):** Exams — add to v1 Task 11:
- Tabs: `All (N)` / `Active (N)` / `Scheduled (N)` / `Drafts (N)` (fix from "All/Active/Drafts")
- "+ Compose exam" button (instructor/admin only)

**Task F12 (AMEND):** Author studio — add to v1 Task 12:
- Validation status bar between metadata row and questions pane (client-computed)
- Header buttons: Import / Preview / Save draft / Publish (Publish → `PATCH /v1/quizzes/{id}` `{ status: 'active' }`)

**Task F13 (AMEND):** Agent screen — add to v1 Task 13:
- Landing intro view at top of Agent screen (above tabs):
  - Header action buttons: OpenAPI download, MCP manifest download, New API key
  - Hero card with "TWO INTERFACES, ONE MODEL" chip + serif headline + body + curl example card
- OpenAPI download → serve `api/openapi.yaml` at `GET /v1/openapi.yaml` (no auth)
- MCP manifest download → serve tool descriptors at `GET /v1/mcp/manifest` (no auth)

---

## Screenshot reference index

| Screenshot | Key design details captured |
|---|---|
| `01-signup.png` | Password field, role picker, stat grid, compliance footer |
| `02-library.png` | Tab labels with counts, cohort context panel, "+ New quiz" button |
| `03-exams.png` | Exams tabs All/Active/Scheduled/Drafts, "+ Compose exam", split list/detail |
| `04-quiz-setup.png` | "30 ITEMS MATCH" live badge, session summary rail, numbered blocks |
| `05-quiz-active-mc.png` | Question palette, integrity checks, allowed materials, "N PTS" header |
| `06-quiz-active-code.png` | Code question type with editor + Run tests + mock output |
| `07-results.png` | Export PDF + Share quiz header buttons, donut, cohort histogram |
| `08-progress-dashboard.png` | 5-stat strip with deltas, 12w range selector, Export button, two charts |
| `09-author-studio.png` | Validation status bar, per-question pts, Import/Preview/Save/Publish |
| `10-agent-api-keys.png` | Agent landing intro: hero + curl example + OpenAPI/MCP/New key header |
| `11-agent-mcp-tools.png` | Same landing view (confirms it's the default agent screen state) |
| `13-share-modal-link.png` | Share modal Link tab: URL, Quick actions, QR, privacy toggles, agent-equivalent |
| `15-library-paper-theme.png` | Signup form in paper theme (confirms theme-awareness of auth screen) |
