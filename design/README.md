# Handoff — Harus Assessment Platform

## Overview
**Harus** is an assessment platform with three first-class audiences that share one underlying data model:

1. **Learners** take quizzes and composed exams; review per-item results; track progress over time.
2. **Authors/instructors** compose individual quizzes and bundle them into weighted exams.
3. **AI agents** read and write the same objects through an OpenAPI/MCP surface — they can import quizzes, generate quizzes from source documents, compose exams from existing quizzes, fetch attempts, query cohort stats, send feedback, and create study plans.

The design's organizing principle: **every screen a human uses is backed by the same endpoint an agent uses.** No duplicate state, no scraping.

## About the design files
The files in this bundle are **design references created in HTML/React-via-Babel** — high-fidelity prototypes showing intended look, structure, and behavior. They are **not production code**. The task is to **recreate these designs in the target codebase's environment** (React + TS, Next.js, Remix, SwiftUI, native — whatever the host project uses) using its established patterns, component library, and routing. If no environment exists yet, choose the most appropriate stack (recommended: Next.js + TypeScript + Tailwind + a headless component library like Radix UI).

The HTML uses inline-JSX-with-Babel for speed of prototyping. In production:
- Replace `<script type="text/babel">` with a real build pipeline
- Replace mock data (`src/data.jsx`) with API calls
- Replace inline-style objects with whatever styling solution the host codebase uses
- Replace the toy SVG charts with a real library (Recharts, Visx, ECharts)

## Fidelity
**High fidelity.** Treat the colors, typography, spacing, layout, and interaction patterns as final. The visual system is intentionally academic/serious — restrained, dense, monospace metadata, serif headings, no playful gradients. Recreate pixel-faithfully using the target codebase's libraries.

## Stack guidance (when no codebase exists)
- **Framework:** Next.js 14+ (app router) or Remix
- **Language:** TypeScript, strict
- **Styling:** Tailwind CSS, with the design tokens (see `tokens.md`) configured as CSS variables and exposed through `tailwind.config.ts`
- **Components:** Radix UI primitives (Dialog, Tabs, Tooltip, RadioGroup) styled to match
- **Charts:** Recharts or Visx — both can be styled to match the existing chart look
- **State:** React Server Components + Server Actions for mutations; React Query (TanStack) for client-side caches
- **Forms:** React Hook Form + Zod for validation
- **Code editor (for the Code question type):** Monaco or CodeMirror 6
- **Backend:** the API surface is documented below — implement it however the team prefers (Postgres + Drizzle is a fine default)

---

## Design system

### Brand
- **Name:** Harus
- **Tagline:** "Quizzes that learners and agents can both read."
- **Mark:** Two vertical strokes with a horizontal bar between, on an accent-colored rounded square. SVG in `src/atoms.jsx` under `Logo`.

### Type
- **Serif (headings, scores, marketing):** `Source Serif 4` — weights 400/500/600/700. Optical sizing enabled.
- **Sans (UI, body, controls):** `Inter` — weights 400/500/600/700.
- **Mono (metadata, code, kickers, API keys):** `JetBrains Mono` — weights 400/500/600.

**Pairings used in the design:**
- Hero / screen titles: Source Serif 500, 26-30px, letter-spacing -0.3 to -0.4
- Section titles: Source Serif 500, 18-22px, letter-spacing -0.2
- Body: Inter 400, 13-14px, line-height 1.55-1.65
- Kickers / metadata: JetBrains Mono 500, 10-11px, letter-spacing 1.2-1.4, uppercase
- API keys / code blocks: JetBrains Mono 400, 12-13.5px, line-height 1.6

### Color tokens
Three themes are defined as CSS variable sets, switched via `data-theme` on the root. **Slate** (dark) is the default. **Paper** (warm light) and **Cobalt** (deep blue + gold) are alternates.

See `tokens.md` for the full table.

Key principles:
- Accent is used only for primary CTAs, active nav, "correct" outcomes, and one line per chart
- Amber = warning / pending / "needs review"
- Red = error / wrong / revoke
- Blue = informational / secondary categorization
- Surfaces step up in lightness (`bg` → `surface` → `surface-2` → `surface-3`) for cards, inputs, and elevated rows

### Spacing & radii
- Base unit: 4px (most spacing on a 4/6/8/10/12/14/16/18/20/22/24/28/32/36/40 scale)
- Page padding: 28-40px horizontal
- Card padding: 20-28px
- Border radius: `6px` for inputs/buttons/cards, `10px` for large hero cards, `4px` for small chips/tags, `999px` for round pills
- Borders: 1px solid `var(--border)`, occasionally 1px dashed `var(--border)` for KV rows

### Iconography
Line icons at stroke-width 1.6, 16px default. All inline SVG, see `src/atoms.jsx` `Icon` component for the full set. Do NOT add filled icons or icon fonts — the line style is intentional.

### Decorative placeholder
Striped diagonal background with a dashed border and a mono caption. Used wherever real imagery would go in production. See `Placeholder` in `src/atoms.jsx`.

---

## Screens / Views

### 1. Signup (`SignupScreen`)
**Purpose:** Sign in, register, or get an agent shortcut.

**Layout:** Two-column split at 1.05fr / 1fr.
- **Left:** marketing column on a vertical gradient from `surface` to `bg`. Logo top-left, large serif headline mid-page, descriptive paragraph, a 2x2 grid of statistic cards, and a compliance footer.
- **Right:** auth form. Tab switcher (Create account / Sign in) over a stack of labeled inputs. Role picker is a 3-up segmented control (learner / instructor / agent) with a short helper line that changes per role. Submit button is full width primary. Below: "OR" divider, two ghost buttons (SSO, access code). At the bottom: an "Agent shortcut" callout with a mono inline `POST /v1/agents/register` reference.

**Notes:**
- Recognized institutional email lights up an SSO hint
- The whole page is on `var(--bg)` background; nothing else
- The marketing left column should NEVER use brand or 3rd-party imagery — use the Placeholder pattern if imagery is needed later

### 2. Library (`LibraryScreen`)
**Purpose:** Browse and start any assigned quiz.

**Layout:**
- Section title "Library" with kicker "Spring 2026 · Active term" and action buttons (Filter, New quiz)
- Tab bar: All / Assigned / Completed / Drafts with counts
- **Hero "Up next" card**, full-width, two-column inside the card:
  - Left: status tags, large serif title, description, three KV stats (Questions, Duration, Attempts), CTA buttons
  - Right: cohort context KV list + recommended prep list with bullet icons
- **Quiz grid**: `repeat(auto-fill, minmax(330px, 1fr))`, each card has:
  - Header: course code + difficulty (mono kicker) and an updated-at tag
  - Body: a 4px-wide accent stripe down the left, then title and description
  - Metadata strip: question count, duration, completion ratio (mono)
  - Footer (on `surface-2`): tags chips on the left, "Open →" link on the right

**Color rule:** each card's accent stripe is one of `var(--accent)`, `var(--blue)`, `var(--amber)` based on the quiz's color field. This gives the grid visual rhythm.

### 3. Exams (`ExamsScreen`)
**Purpose:** Browse and start composed assessments (bundled quizzes with weighted sections). Distinct from individual quizzes.

**Layout:** Split list/detail.
- Tabs: All / Active / Scheduled / Drafts
- **Left list (340px):** vertical stack of exam cards. Each card has a left accent border (filled when selected), course mono kicker, status tag, serif title, and a mono metadata row (duration, sections, manual-vs-agent marker)
- **Right detail (flex):** large card with:
  - **Header band:** status & tag chips, large serif title, description, primary CTA on the right, "Composed by …" caption below
  - **Stat strip (5 columns, dividers between):** Duration, Total items, Total points, Pass mark, Assigned
  - **Composition section:**
    - A horizontal weight bar — each section is a colored band proportional to its weight
    - A table of sections (§ / title / mix / items / weight) with a serif numeral in the section's color
  - **Footer (3 columns):** Window (KV list), Integrity (check/x list with icon colors), Composition trace (different content for manual vs agent-composed — for agents, show seed/inputs/strategy/confidence in a mono code block)

**The composition trace is the conceptual centerpiece of this screen.** It makes the distinction between hand-authored and agent-composed exams visible and auditable to the instructor.

### 4. Quiz (`QuizScreen`) — two stages
**Stage A: Setup (`QuizSetup`)** — the learner composes a session before starting.

Layout: left form (flex) + right summary rail (340px).

Form blocks (numbered kickers 01-08):
1. **Session mode** — 3-up cards: Practice (untimed) / Timed / Adaptive. Selected card uses `accent-dim` background.
2. **Categories** — pill chips with a color dot, item count, and selected/unselected states. Pill radius 999px.
3. **Tags** — mono micro-chips, 4px radius. Show "+ tag" when unselected, "✓ tag" with filled accent background when selected.
4. **Difficulty** — 4-up segmented control (Introductory / Intermediate / Advanced / Mixed). When "Mixed" is selected, reveal a 25/50/25 ratio bar inside a surface-2 panel.
5. **Question types** — 5-up segmented selector with check marks.
6. **Length** — two `NumberStep` controls (−/+ buttons either side of a large serif number). Disable Time limit when mode = Practice.
7. **Source** — Item bank vs Specific quiz. Picking Specific reveals a 3-row list of quizzes from the library.
8. **Options** — toggle rows (label + sub) with a slide switch on the right.

Right rail (`Session summary`):
- Title block summarizing the session
- KV list of all chosen values
- Estimated coverage % of stated goals
- Primary CTA (disabled when pool < count or types empty)
- Save as preset
- **Agent-equivalent panel** — mono `POST /v1/sessions { … }` block. Pedagogical bridge between UI and API.
- Recent presets list

**Stage B: Active quiz** (after Start)

Layout: main column (flex) + 280px right rail (palette).

Main column:
- Sticky exam header (bg) with attempt + course mono kicker, title, timer (turns red when < 5 min), Save & exit
- 3px progress bar tied to answered count
- Question content (padding 44px 56px, max-width 820px):
  - Mono question metadata + flag-for-review button
  - Serif prompt at 26px
  - **One of five inputs:**
    - **MC:** large rectangular options, each with a letter badge on the left (A/B/C/D in mono); selected = accent-dim background + accent border + check mark
    - **TF:** two big serif squares ("True" / "False")
    - **Short:** monospace input with format examples below
    - **Code:** code editor box with file header (`solution.py · python 3.11`), Run tests / Reset buttons, textarea body, and a footer with mock test runner output
    - **Essay:** large serif textarea with word count, autosave indicator, rubric reminder
- Footer: Previous / Next or Submit attempt

Right rail:
- Question palette (5-column grid of numbered buttons; answered = accent-dim, current = accent border, flagged = amber dot top-right)
- Legend
- Integrity checks
- Allowed materials block

### 5. Results (`ResultsScreen`)
**Purpose:** Review just-submitted attempt and how it compares.

Layout:
- Title + export/share actions
- **Top row (2 cards):**
  - Score card (1.3fr): donut chart on the left, tags + big serif score + paragraph summary on the right; below, 4-up stat strip (Duration, Cohort avg, Percentile, Topic mastery) with up/down tone indicators
  - Cohort distribution (1fr): histogram with the learner's bin highlighted in accent; legend below
- **Per-item review card:**
  - Header bar with type kickers + tag chips
  - List of items, one per row, in `48px 1fr 110px` grid:
    - Status badge (check or x on tinted background)
    - Item type kicker + serif prompt + "Your answer" KV + grader's note
    - Points earned (right-aligned serif) + "See solution →" link
  - Footer band (on `surface-2`) with study-plan callout + Back/View plan CTAs

### 6. Progress (`DashboardScreen`)
**Purpose:** Trends, cohort comparison, item analysis, study plan.

Renders progressively based on the `statsDepth` tweak (`minimal` / `standard` / `full`).

Always shown:
- **Top stat strip** (5 columns inside a card, dividers between): Avg score, Attempts, Hours, Streak, Mastered topics
- **Score trend** (1.5fr): area line chart, weekly rolling
- **By subject** (1fr): horizontal bar chart, score & attempts

Shown at `standard` and above:
- **Cohort comparison** (1fr): histogram with the learner's bin highlighted + 3 KV stats below
- **Upcoming** (1fr): vertical list of assignments with color dot per course

Shown at `full`:
- **Item analysis** card: difficulty × discrimination scatter (IRT) at full width, then a 5-up grid of per-item summary cards (with red/amber/accent correct-pct tag)
- **Study plan** (1.3fr): generated 6-week recovery plan, week column in mono + content + estimated hours. Footer callout shows "Generated by `plan.create`"
- **Mastery map** (1fr): 30-cell heatmap using `color-mix(in oklch, accent X%, surface-2)`; below, three labeled groups (Strong, Working, Needs review)

### 7. Author studio (`AuthorScreen`)
**Purpose:** Compose, generate, and distribute assessments.

Layout:
- Title + actions (Import, Preview, Save draft, Publish)
- **Metadata strip** (one card with 5-column inputs + a status footer with check/x pills and a "last edit" mono caption)
- **Three-pane editor** at 320px / flex / 280px:
  - **Left (Questions list):** rows for each draft question (Q#, prompt, type kicker + status tag, points). Selected row gets a left accent border and `accent-dim` background. Footer block: "Generate questions" callout.
  - **Middle (Editor):** header with type label + Duplicate/Move/Delete; form fields for prompt, options (with a fillable correctness-circle on the left of each), points/tag/difficulty row, explanation textarea.
  - **Right rail (2 cards):** Distribution KV list + Edit distribution button; Rubric autograding explainer + mono criteria block; Recent activity feed with colored bars distinguishing You vs agents.

### 8. Agent integration (`AgentScreen`)
**Purpose:** The programmatic surface — keys, MCP tool descriptors, import demo, activity log.

Layout:
- Title + actions (OpenAPI, MCP manifest downloads, New API key)
- **Explainer band** card (2-column): on the left, a tag + serif headline + paragraph + 4 KV inline; on the right (`surface-2`), a "Hello, world" curl CodeBlock with the 200 OK confirmation line in accent
- **Section tabs:** API keys / MCP tools / Import demo / Recent activity

**API keys subsection:**
- Left (1.4fr): list of keys. Each row: label + masked mono prefix on top, action buttons (Copy/Rotate/Revoke) on the right, created/last-used mono caption + scope tags below
- Right (1fr): Auth explainer + sample request CodeBlock + scope reference (2-column check-list of all available scopes)

**MCP tools subsection:**
- Left (300px): scrollable tool list. Selected tool gets `accent-dim` + left accent border. Each row shows the tool name in mono accent + description.
- Right (flex): selected tool detail panel — header with method tag (POST = amber, GET = default) and path mono, then 2-column body: Inputs (list with bullets) + Returns (mono block) on the left, Example curl CodeBlock + MCP descriptor JSON CodeBlock on the right

**Import demo subsection:**
- Left (1.1fr): JSON/MD textarea (`surface-2` background, mono font, 380px+ tall), header with format toggle, footer with endpoint mono + Send request CTA
- Right (1fr): Response card. Status tag (200 OK / 200 OK · warnings). When response arrives: timestamp mono + ms latency, response.json CodeBlock, and either an accent success callout with the created URL or an amber warnings block

**Recent activity subsection:**
- Single full-width log table. Header row on `surface-2`, then mono rows with columns: Time / Tool (accent) / Agent / Status (accent/red colored) / Note

---

## Cross-cutting components

### Learning objectives (`LearningObjectives` in `src/share.jsx`)
**Purpose:** Show 3–4 outcome bullets up front so the learner knows what they'll get from a quiz or exam.

**Appears on:**
- Library "Up next" hero card — between the description and the CTA row
- Exams detail panel — between the stat strip and the Composition section

**Layout (default):**
- Wrapped in a `surface-2` panel with `1px solid var(--border)` and 6px radius, 18px / 20px padding
- Top kicker: small sparkle icon + mono uppercase "What you'll learn", `var(--muted)`
- 2-column CSS grid (`1fr 1fr`) of `<li>` items, gap 10px
- Each `<li>` has a 2px left accent bar, 10px left padding, a 2-digit mono index ("01", "02"), and the objective text in `var(--text-2)`

**Compact variant:** prop `compact` switches to a single-column list with a check icon per row — used in tighter rails.

**Data:** every Quiz and every Exam carries an `objectives: string[]` field on the API. Authoring UIs (Author Studio) should let instructors edit this; agents producing quizzes via `quiz.generate` should also emit objectives.

### Share modal (`ShareProvider`, `ShareModal`, `ShareButton` in `src/share.jsx`)
**Purpose:** Let a learner or instructor share a quiz, an exam, or a single explained question to any external surface — socials, embed, link, image. The shared payload includes the prompt and (optionally) the explanation, so the **knowledge** travels, not just a marketing tease.

**Mounting:** Wrap the whole app in `<ShareProvider>`. It provides an imperative `useShare()` hook returning `{ open(payload) }` plus a small `<ShareButton payload=… />` component.

**Triggers in the design:**
| Surface | Where | Payload kind |
|---|---|---|
| Library hero | Next to Preview questions | `quiz` |
| Library card footer | Small mono "↑ Share" link beside the "Open →" link | `quiz` |
| Exams detail | Below the primary CTA | `exam` |
| Results header | "Share quiz" replacing the old "Share with TA" | `quiz` |
| Results per-item review | Small mono Share beside "See solution →" — payload includes the **prompt + explanation** | `item` |

**Modal structure:** 2-column layout (`1fr 280px`) inside a centered 820px max-width card on a blurred dark backdrop.

- **Left column (content):**
  - Header: kicker (`Share · single question` etc), serif title, course caption, close button (×)
  - Tabs: **Link / Socials / Card / Embed**
    - **Link:** copy-URL input + a Copy button that turns accent + check on success; "Quick actions" 3-up grid (Copy as Markdown / Download PDF / Print); a "QR for the room" panel with a striped placeholder square and a Generate button
    - **Socials:** 3-column grid of platform buttons (X, LinkedIn, Reddit, Hacker News, Email, Mastodon) each with a colored badge; a live preview of the composed message; a "Copy composed message" button
    - **Card:** 1200×630 preview rendered as a real card on the page (serif prompt, mono attribution, accent kicker, logo, blurred accent radial in the top-right corner); Download PNG / Copy image / Edit style actions; OG hint
    - **Embed:** `<iframe>` code block with file header "embed.html" and a copy-with-feedback button; a 3-column grid of 6 platform preset cards (Notion, Obsidian, WordPress, MDX, Slack, Discord); a hint that `?interactive=1` enables in-place attempts

- **Right rail (`surface-2`, 280px):**
  - "What travels with it" — toggle rows for Question prompt (locked on), Explanation (only shown when payload.kind === 'item'), Attribution, Your score (off by default)
  - Privacy: 2-up segmented control — Anyone with the link / Cohort only
  - Agent-equivalent: mono `POST /v1/shares` snippet showing the same payload

**API surface:** add `POST /v1/shares` and `GET /v1/shares/{id}` (see `api.md`). Embed routes: `/{kind}/{id}/embed` returns a stripped read-only viewer; the `?interactive=1` query lets viewers attempt without writing to the cohort's attempts.

**Privacy rules to enforce server-side:**
- A shared link never reveals other learners' attempts or scores
- `includeScore` is opt-in only
- `visibility: "cohort"` restricts the share to authenticated members of the cohort that owns the quiz/exam
- Anonymous attempts via `?interactive=1` are logged but not written to any user's record

---

## Sidebar & top bar

### Sidebar (`Sidebar`)
- 232px fixed-width, sticky to viewport
- Logo + version mono caption at top
- Sections: Learn (Library / Exams / Take quiz / Last results / Progress), Teach (Author studio), Integrate (Agent API)
- Each item: icon + label, selected gets `accent-dim` background + accent border + accent text. Mono section labels above each group.
- Footer: 32px avatar circle + name + role caption + settings icon

### Top bar (`Topbar`)
- Sticky to viewport, on `bg`
- Mono breadcrumb above a 26px serif title; optional 13px subtitle below
- Right side: optional action buttons, then a divided cluster of bell + search icons

---

## Interactions & behavior

### Navigation
- All sidebar items are buttons that swap the right pane (in production, use the host router)
- The Library "Up next" card's Start → opens the Quiz setup
- Results → "View 6-week plan" goes to Progress

### Quiz flow
- Setup must produce a non-empty session before the questions appear; the Start button stays disabled otherwise
- The active quiz autosaves every 8s (decoration only in mock; implement against the real API in production)
- Submit confirms then routes to Results

### Timer
- Active quiz timer counts down once per second
- Below 5 minutes (300 s), the timer box switches to `red-dim` background + red border + red text

### Tweaks (in-page configurator)
- Floating panel in the bottom-right; toggle exposes:
  - **Color theme:** slate / paper / cobalt — switches `data-theme` on `<html>`
  - **Stats depth:** minimal / standard / full — controls how much of Dashboard renders
  - **Agent panel:** show/hide the Agent integration sidebar entry
- In production this panel is not user-facing — it exists for demo. The theme tokens should ship as a runtime-switchable preference; stats depth would become user settings.

### Animations
- Sidebar nav items: 120ms color/background transition
- Buttons: 120ms background + border-color transition
- Cards (hoverable): 140ms border-color shift to `border-strong` on hover
- Toggle switches: 140ms left transition for the knob
- No "fancy" animations — the system is academic/restrained

### State management
- Per-screen state lives in the screen component for now. In production:
  - Quizzes / exams / attempts: React Query against the API
  - Active quiz attempt: server-backed draft store keyed by `attemptId`
  - Theme + tweaks: localStorage / user prefs

---

## API surface (for agents and the app itself)

All endpoints expect `Authorization: Bearer hk_…`. Scopes are checked per endpoint. Rate limit: 120 req/min per key. Errors return RFC 7807 problem+json.

### Quiz
| Method | Path | Tool name | Scope | Notes |
|---|---|---|---|---|
| GET    | `/v1/quizzes`              | `quiz.list`     | `quiz.read`   | List with filters: course, tag, status |
| GET    | `/v1/quizzes/{id}`         | `quiz.get`      | `quiz.read`   | Full questions + rubric |
| POST   | `/v1/quizzes`              | `quiz.import`   | `quiz.write`  | Body: `{ source, format, courseId? }`. Format = `'json' \| 'md'`. Returns `{ quizId, questionCount, warnings[] }`. |
| POST   | `/v1/quizzes/generate`     | `quiz.generate` | `quiz.write`  | Body: `{ source, questionCount, types?, difficulty? }`. Returns `{ quizId, questions[] }`. |
| PATCH  | `/v1/quizzes/{id}`         | `quiz.update`   | `quiz.write`  | Partial update; locked once published |
| DELETE | `/v1/quizzes/{id}`         | `quiz.delete`   | `quiz.write`  | Soft delete |

### Exam
| Method | Path | Tool name | Scope | Notes |
|---|---|---|---|---|
| GET    | `/v1/exams`                | `exam.list`     | `quiz.read`   | List composed exams |
| GET    | `/v1/exams/{id}`           | `exam.get`      | `quiz.read`   | Returns `{ exam, sections[], cohortStatus }` |
| POST   | `/v1/exams`                | `exam.compose`  | `quiz.write`  | Body: `{ title, sections: { quizId, weight, items? }[], duration, window?: { open, close } }`. Returns `{ examId, totalPoints, sections[], warnings[] }`. |
| GET    | `/v1/exams/{id}/stats`     | `exam.stats`    | `stats.read`  | Returns `{ passRate, sectionAvgs[], timeP50, timeP95 }` |

### Sessions / attempts
| Method | Path | Tool name | Scope | Notes |
|---|---|---|---|---|
| POST   | `/v1/sessions`             | `session.create`| `attempt.write` | Body matches the Quiz setup form: `{ cats[], tags[], types[], diff, count, duration, mode }`. Returns `{ sessionId, questions[] }`. |
| GET    | `/v1/attempts/{id}`        | `attempt.get`   | `attempt.read`  | Returns `{ user, score, answers[], rubric[] }` |
| POST   | `/v1/attempts/{id}/grade`  | `attempt.grade` | `attempt.write` | Trigger or override grading |

### Stats
| Method | Path | Tool name | Scope | Notes |
|---|---|---|---|---|
| GET    | `/v1/quizzes/{id}/stats`   | `stats.cohort`  | `stats.read`    | `{ avg, median, distribution[], items[] }` |

### Feedback & plans
| Method | Path | Tool name | Scope | Notes |
|---|---|---|---|---|
| POST   | `/v1/messages`             | `feedback.send` | `feedback.write`| `{ userId, channel, body, linkQuizId? }` |
| POST   | `/v1/plans`                | `plan.create`   | `plan.write`    | `{ userId, goal, lookbackDays? }` |

### Sharing
| Method | Path | Tool name | Scope | Notes |
|---|---|---|---|---|
| POST   | `/v1/shares`               | `share.create`  | `quiz.read`     | Body: `{ kind: "quiz"\|"exam"\|"item", id, visibility, includeExplanation?, includeScore? }`. Returns `{ shareId, url, embedUrl, og: { image, title, description } }`. |
| GET    | `/v1/shares/{id}`          | `share.get`     | (none — public) | Resolves a share to a stripped read-only view. |
| GET    | `/v1/{kind}/{id}/embed`    | —               | (none — public) | Read-only iframe payload. `?interactive=1` allows attempting without writing. |

### MCP descriptors
Every tool above ships with an MCP descriptor (name, description, input_schema). The bundled HTML shows the descriptor preview format in `src/screen-agent.jsx` (`mcpDescriptor`).

### Webhooks (recommended)
- `attempt.submitted` — fires when a learner finishes
- `attempt.graded` — fires when grading completes
- `quiz.published` — fires when a draft goes live

---

## Files in this bundle

```
design_handoff_harus_platform/
├── README.md                 ← this file
├── tokens.md                 ← exhaustive design tokens
├── api.md                    ← OpenAPI 3.1 sketch
└── source/
    ├── Harus Quiz Platform.html
    ├── tweaks-panel.jsx
    └── src/
        ├── app.jsx
        ├── data.jsx          ← shape of all data — replace with real API
        ├── atoms.jsx         ← Button, Tag, Card, Icon, etc.
        ├── charts.jsx        ← toy SVG charts — replace with a chart lib
        ├── share.jsx         ← LearningObjectives + ShareProvider/Modal
        ├── sidebar.jsx
        ├── screen-signup.jsx
        ├── screen-library.jsx
        ├── screen-exams.jsx
        ├── screen-quiz.jsx
        ├── screen-results.jsx
        ├── screen-dashboard.jsx
        ├── screen-author.jsx
        └── screen-agent.jsx
```

## Implementation order (recommended)
1. **Tokens + atoms + Sidebar + Topbar** — get the shell visible
2. **Library + Exams** — read-only screens; verifies the data shapes; objectives render from the same `objectives` field on Quiz and Exam
3. **Quiz setup + active quiz + Results** — the learner's happy path; wire the Share button on Results per-item so it includes the explanation
4. **Dashboard** — replace toy charts; the IRT scatter and mastery heatmap are the trickiest
5. **Share modal + Share API** — `POST /v1/shares` + embed routes; OG image generator (Vercel OG / Satori works well)
6. **Author studio** — write paths; ties to the API; objectives should be editable here
7. **Agent integration** — keys management, MCP manifest export, import endpoint, activity log
8. **Webhooks + auth scopes** — production-readiness

## Out of scope for this handoff
- Real authentication / SSO / SAML wiring
- Production proctor integrations
- Payment / billing
- Email templates (the in-app `feedback.send` is shown; the email channel is not designed)
- Mobile-specific layouts — the current designs target ≥1280px viewports. Treat ≤1024px as a separate design pass.

## Brand & imagery
Do **not** add stock photography or marketing illustrations. Where imagery would belong (e.g., empty states, course covers), use the `Placeholder` pattern from `atoms.jsx`: a diagonal striped background with a dashed border and a mono caption describing what should go there. Real imagery is a separate design pass with the customer's own assets.
