# AME — Refreshed Plan & Rebrand (2026-07-01)

Two things this doc does:

1. **Refresh the roadmap** — what's actually left, plus a learner-facing track
   informed by the best OSS learning tools (Anki, Open edX, Coursera, Duolingo).
2. **Give AME an identity** — `ame` = **飴** ("candy / sweets" in Japanese), themed
   in **Hatsune Miku's palette**. One coherent brand: *learning that's sweet.*

This does **not** replace [`docs/ROADMAP.md`](../ROADMAP.md) — the operator-console
line (v0.3 → v1.1) stands. This adds a **parallel learner track** and a **brand
layer**, and applies the kill-gate rule to each new surface.

---

## Part 1 — Where we actually are

**Version:** `v0.2.1`. The admin-console epic is ~2/5 done.

| Milestone | Status |
|-----------|--------|
| v0.1 platform + admin foundation | ✅ shipped |
| v0.2 #1 Token Audit, #2 Settings/Flags | ✅ shipped (0.2.0) |
| v0.4 A1 Deepen panel | ✅ shipped early (0.2.0) |
| **v0.3 #3 Feedback/Flags queue** | ⬜ not started |
| **v0.3 Data retention & pruning** | 🟡 in flight — migration + `retention.rs` drafted, uncommitted |
| **v0.3 Learner mobile (H5)** | ⬜ not started — root cause known (`Sidebar.tsx` permanent drawer) |
| **v0.4 A2 Async deepen notes** | ⬜ not started |
| **v1.0 #4 Analytics Dashboard** | ⬜ blocked on Phase D metrics |
| **v1.1 GA polish** | ⬜ integration/finish |
| **v2.0 High-scale hot-path** | ⬜ future |

**The honest gap:** AME is a strong *assessment + agent* platform, but it has no
**retention loop**. A learner takes a quiz, sees results, dives deeper — then
leaves. Nothing brings them back tomorrow. Every tool below closes that loop.

---

## Part 2 — What the best OSS teaches us

Scan of the category leaders, filtered through the kill-gate ("what breaks for a
real user if we ship without this?"). Ordered by value-to-effort.

### Anki → **Spaced Repetition (SRS)** — the headline addition
Anki's whole value is one idea: **review the right card at the right time.** AME
already has a question bank and a flashcards spec (`2026-05-28-flashcards-design.md`).
Adding a scheduler turns a one-shot quiz tool into a daily-habit tool.

- Adopt **FSRS** (the modern, open, well-documented successor to SM-2 — same one
  Anki now ships). Pure function: `(card_state, rating) → next_due`. No LLM, no
  heavy infra.
- New `tb_review_state` (per user × question: stability, difficulty, due date,
  reps). A daily **"Due today"** queue endpoint + page.
- Ratings map to the four Anki grades (Again / Hard / Good / Easy).
- **Kill-gate:** ✅ Without it there is no reason to return daily. This is *the*
  retention mechanic. Highest priority of the new track.

### Duolingo → **Streaks & daily goal** — the sweet, on-brand hook
A streak is the cheapest retention multiplier in the industry, and it fits the
**candy** identity perfectly (a growing candy jar, sweetness meter).

- `tb_user_activity` day-buckets already implied by stats; derive a streak count.
- Daily goal (N reviews), streak counter in the AppBar, gentle "don't break your
  streak" empty-state. No push notifications in scope (no infra for it yet).
- **Kill-gate:** ✅ pairs with SRS — the goal *is* "clear today's due queue."
  Ship together or right after.

### Open edX / Coursera → **Learning Paths (course-lite)**
edX/Coursera group content into an ordered sequence with progress. AME has
assessments but no way to say "do these five, in order, to learn Rust."

- A `Path` = ordered list of existing assessments + a title/description. Progress =
  % of member assessments completed. Reuses everything; thin new surface.
- **Kill-gate:** 🟡 *Defer to a spike.* Valuable, but authors can fake it with tags
  today. Validate demand before building the entity. **Do not** build cohorts,
  peer-review, or discussion forums — that's edX's weight, not ours.

### Open edX / Coursera → **Certificates of completion**
A shareable, verifiable completion artifact — strong motivation, cheap to build.

- On finishing a Path (or a graded assessment above a threshold), mint a
  `tb_certificates` row + a public verify URL (`/verify/{id}`) and a rendered
  card. No PDF pipeline required initially — an OG-image / printable page.
- **Kill-gate:** 🟡 Depends on Paths landing first. Defer with Paths.

### Cross-cutting: **Cloze / occlusion question type**
Anki's most-used card type. AME's question types are MC/TF/short/essay/code.
- **Kill-gate:** 🟡 Nice, not load-bearing. Defer until SRS proves the flashcard
  loop is used.

**Verdict:** SRS + Streaks pass the gate now. Paths/Certificates are a validated
spike. Everything else defers. This respects the retro lesson — *validate the need
before the task board.*

---

## Part 3 — The `ame` identity: 飴 (candy) × Hatsune Miku

### The story
`ame` is **飴** — Japanese for candy / hard sweets. The product promise:
**assessment that's sweet, not bitter.** Studying as a treat you come back for.
Hatsune Miku — teal-haired, the world's most famous open/vocaloid icon — supplies
the palette. The two fuse: **Miku teal + candy pink**, playful but clean.

> This is a **palette + motif** adoption (colors, accents, a candy motif), **not**
> use of the Miku character art/likeness. Keeps us clear of Crypton's character
> licensing while honoring the aesthetic. No official Miku imagery ships.

### Palette (replaces indigo/emerald in `ThemeRegistry.tsx`)

| Token | Light | Dark | Source |
|-------|-------|------|--------|
| **primary** (Miku teal) | `#39C5BB` | `#57D9CE` | canonical 初音ミク color |
| primary.dark | `#158F86` | `#39C5BB` | deep teal |
| **secondary** (candy pink) | `#FF6699` | `#FF85AD` | Miku necktie/nail accent |
| accent (soft mint) | `#86CECB` | `#2A6F6B` | hair highlight |
| background.default | `#F4FBFA` (candy-mint tint) | `#12181B` (near-black teal) | — |
| background.paper | `#FFFFFF` | `#1A2226` | — |
| success | keep teal-green | | reads as "correct" |
| error | `#E4405F` (raspberry) | | on-brand red |

Design cues, kept tasteful (kill-gate on cuteness — no motion sickness, no clutter):
- **Candy motif, sparingly:** streak = a "candy jar" filling; XP = "sugar." A
  subtle wrapper-twist divider or rounded-pill chips (already `borderRadius: 8`).
- **Gradient buttons** already exist — reskin to teal→pink.
- Keep Inter/Roboto typography; optionally add a rounded display face for the
  wordmark only.
- Wordmark: lowercase **`ame`** with a small candy/twist glyph, teal→pink.
- **Accessibility is a hard gate:** teal-on-white and pink-on-white must pass WCAG
  AA for text; use the `.dark` variants for text, bright variants for fills only.
  Dark mode must stay first-class.

### Scope guard
Theme swap touches **one file** (`ThemeRegistry.tsx`) for ~90% of the surface,
because the whole app is MUI-tokened. Wordmark + favicon + a streak widget are the
only net-new components. **No** per-page restyling, **no** Tailwind, **no**
character art.

---

## Part 4 — Refreshed sequence

The operator line and the learner line run in parallel (as the ROADMAP already
intends). Suggested ordering:

1. **v0.3 (finish the operator line)** — land the in-flight data-retention work
   (migration + `retention.rs` are already drafted), then #3 Flags queue, then
   learner-mobile H5. *H5 is a prerequisite for the learner track feeling good.*
2. **v0.3.x — Brand refresh** *(small, parallel, morale + differentiation)*:
   the Miku/candy palette in `ThemeRegistry.tsx` + wordmark + favicon. Low risk,
   high delight, unblocks nothing but lifts everything. Can ship anytime.
3. **v0.5 — Retention loop** *(the new headline)*: **SRS (FSRS)** + **daily
   queue** + **streaks/daily goal**. This is the biggest product lever and the
   reason the brand refresh is worth doing — a daily-habit app deserves a
   daily-delight skin.
4. **v0.6 — Paths spike** *(gated)*: validate learning-paths demand; if it holds,
   build Path + Certificates. Otherwise drop.
5. **v1.0 / v1.1** — Analytics + GA polish, as in ROADMAP (now includes SRS/streak
   metrics in the analytics dashboard: DAU, review completion, streak retention).

### Test-first reminder (retro rule #2)
SRS is a refactor-adjacent feature touching the session/results flow. Pin the
contract tests first: FSRS scheduling is a pure function — unit-test the
`(state, rating) → next_due` table against known FSRS vectors **before** wiring
the DB. E2E the "answer → rate → due tomorrow → appears in queue" loop against the
target shape before migrating.

---

## Decisions (settled 2026-07-01)

1. **Brand: full — palette + candy motif + gamification.** The candy-jar streak,
   XP-as-"sugar", and daily goal ship as part of the identity. These are only
   meaningful *with* SRS, so the brand refresh and the retention loop land as one
   coordinated v0.5, not separately.
2. **SRS: yes — v0.5 is the retention track.** FSRS scheduler + daily due-queue +
   streaks become a first-class learner milestone alongside the operator line.
3. **Learning Paths: spike first.** Validate author demand before building the
   `Path`/`Certificate` entities. Not committed; not out.
4. **Name stays `ame`** — the 飴 story *is* the identity. Surface the tagline
   *"ame — study, sweetened"* in the wordmark, README, and `skill.json` description.

---

## v0.5 — Retention loop (the new headline milestone)

**Objective:** a learner comes back tomorrow because there's something sweet due.
Brand + gamification + SRS ship together — the skin exists *because* the loop does.

**Kill-gate:** without a daily due-queue there is no reason to return; this is the
one feature that changes AME from a one-shot quiz tool into a habit. Passes.

**Test-first (retro rule #2):** FSRS is a pure function — pin its
`(state, rating) → next_due` unit tests against known FSRS vectors *before* any DB
wiring; E2E the "answer → rate → due tomorrow → shows in queue" loop against the
target shape before migrating.

Rough shape (ordered smallest → biggest, to be broken into an epic later):

1. **Brand refresh** — Miku teal + candy-pink palette in `ThemeRegistry.tsx`,
   wordmark, favicon, tagline. Isolated, shippable first, unblocks the delight of
   everything after. WCAG AA hard-gated.
2. **FSRS core (pure, tested)** — `(card_state, rating) → next_due` with the four
   Anki grades. No DB, no HTTP — just the scheduler + its test vectors.
3. **`tb_review_state`** — per user × question: stability, difficulty, due, reps.
   Migration + `GET /v1/reviews/due` (today's queue) + rate endpoint.
4. **"Due today" page** — the daily queue UI on the learner surface (rides the H5
   responsive work, so H5 lands first or alongside).
5. **Streak + daily goal** — derive a streak from activity day-buckets; candy-jar
   widget in the AppBar; daily-goal = "clear today's queue"; XP = "sugar."
6. **Analytics hooks** — feed review-completion, streak-retention, and DAU into the
   v1.0 dashboard so the loop is measurable.

**Dependency note:** step 4 wants **Learner-mobile H5** (ROADMAP v0.3) done, since
the due-queue is a phone-first daily surface. Sequence H5 before or with v0.5.
