# Learn Deeper — Design

**Date:** 2026-06-14
**Status:** Draft — discussion
**Milestone:** v0.4 "Learn Deeper" (lands *after* v0.3 content-integrity; see ROADMAP)

## Why this milestone exists

Every roadmap feature from v0.2→v1.1 builds the **operator console**. This is the
first deliberately **learner-facing** track: when a learner finishes a question,
let them *push further* on that question or topic instead of moving on. It hangs
off the per-question **review surface** (the post-answer results view that already
shows selected vs. correct + `explanation`).

Two tracks live under this milestone. They are honestly **different axes** and are
kept separate on purpose:

- **Track A — Deepen (learner):** progressive "dive deeper" content on the review
  screen. The flagship of this milestone.
- **Track B — Notion authoring pipeline (author/operator):** sync question-bank
  content with a Notion workspace. *Not* a learner feature — it shares the
  question model, not the review surface. Specced here because it was requested
  alongside, but kill-gated and sequenced independently; it can slip without
  blocking Track A.

## Kill-gate (CLAUDE.md rule #1)

> *What breaks for a real user if we ship without this?*

| Capability | Breaks without it? | Verdict |
| --- | --- | --- |
| Related questions by tag | No — but it's the cheapest real learner value, reuses bank+FTS | **A1, ship** |
| Author deep-dive field | No — `explanation` exists; this is a richer second field | **A1, ship** |
| External reference URL | No — provider-agnostic, ~1 day | **A1, ship** |
| Async deepen notes (flag → agent generates → read) | No — pure enhancement; AME stores flags + notes, the *user's* agent generates | **A2, specced below** |
| Notion authoring sync | No — convenience for authors maintaining content elsewhere | **B, own spec pass, defer-friendly** |

Nothing here is load-bearing. So: ship the cheap, safe Track A1 slice first; A2
(async notes) reuses A1's renderer + read contract; B (Notion) still needs a real
design pass before tasks. Any of them can be cut without harming the milestone.

---

## Track A — Deepen

One new surface: a **Deepen panel** on the per-question review screen. Build the
panel once; the features below are progressive content inside it.

### Phase A1 — Static deepen *(cheap, safe, ships the panel)*

Reuses existing data; no new subsystems.

**A1.1 — Related questions by tag.**
Each `Question` already carries `tags: Vec<String>`
(`api/src/domain/question.rs:103`) and the bank supports `?tag=` filtering + FTS.
On the review screen, for the question just answered, surface "Practice N more on
*<tag>*" linking into the existing practice/flashcard flow scoped to that tag.

- API: reuse `GET /v1/questions?status=live&tag=<t>&limit=N`. No new endpoint if
  the existing filter suffices; add a dedicated `GET /v1/questions/{id}/related`
  only if ranking (shared-tag count, exclude already-seen) needs server logic.
- Out of scope: ML similarity, embeddings. Tag overlap only.

**A1.2 — Author deep-dive field.**
`Question.explanation` (`question.rs:109`) is the *short* per-answer note. Add an
optional **`deep_dive`** rich-text field for longer teaching content (worked
solution, "why the distractors are wrong", further context).

- Migration: `tb_questions.deep_dive TEXT NULL`; mirror into `QuestionVersion`
  (it is versioned — `question.rs:124`) so deep-dive travels with publish.
- Wire: add to `Question`, `QuestionInsert`, `QuestionPatch`
  (`api/src/bank/questions.rs`), authoring form, and the review panel (collapsed
  by default, "Dive deeper" expander).
- Rendered through the **shared sanitized renderer** defined in "Rendering deepen
  content" below — the *same* component A2's agent-generated notes use. Build it
  once here.

**A1.3 — External reference URL.**
The generic, provider-agnostic 80% of the KM ask. An optional **reference link**
per question — "further reading" pointing anywhere (a Notion page, a doc, a
video).

- Reuse the existing `Question.source` field (`question.rs:110`) if it is free for
  this; otherwise add `reference_url TEXT NULL` + optional `reference_label`.
  **Decide during build** — check current `source` usage first.
- Validate scheme (`http`/`https` only); render as a safe external link
  (`rel="noopener noreferrer"`).

**A1.4 — Deepen read contract (dual-surface).**
A1 has **two consumers**: the human web Drawer *and* the user's own agent. Define
the data once, serve it both ways.

- **Single-question read.** Routes today expose `GET /v1/questions` (list) and
  `/{id}/versions`, but **no bare `GET /v1/questions/{id}`**. Add a bundled
  `GET /v1/questions/{id}/deepen` returning the question + `deep_dive` +
  reference + related-by-tag in one call (agent does one round trip, not four;
  matches the `llms.txt` playbook style).
- **Ordering in attempt/assessment reads.** `GET /v1/attempts/{id}` and
  `GET /v1/assessments/{id}` must return questions in **order** so an agent can
  resolve positional references ("question 1") deterministically.
- **Answer-key gate is reused, not rebuilt.** The deepen read reveals the key +
  `deep_dive`; that is correct *post-attempt* and already gated by the existing
  in-progress/pending session rule (correct answers hidden until submit).
- **Advertise it.** Add the endpoint + a worked "dive deeper on a hard question"
  playbook to `docs/public/llms.txt` and `skill.json`, or agents won't discover it.

**A1 testing (test-first, rule #2):**
- API integration: deep-dive round-trips through insert→patch→publish→version;
  related-by-tag returns live, tag-matching, non-self questions; reference URL
  rejects non-http schemes.
- e2e: answer a question → review screen shows Deepen panel → expand deep-dive →
  follow related-tag link → follow reference link. Established auth-cookie pattern.

### Phase A2 — Async deepen notes (flag → agent-generate → store → read)

**The model is async, not a synchronous hosted tutor.** AME hosts no LLM. It
stores a *flag* the learner raises in-session and a *note* the user's own agent
generates afterward; the agent supplies all intelligence via the existing
`POST /v1/agents/run` write path. This splits the feature along its natural seams:
**capture** (zero-cost, in the moment of difficulty) → **generate** (off the
critical path, after submit) → **consume** (later, in study mode).

A1's static contract is the **no-agent fallback** — a learner without an agent
still sees `deep_dive` + reference + related-by-tag; they just get no generated
note.

#### The loop

1. **Capture — in-session flag.** The taking UI gets a per-question **"flag for
   dive deeper"** toggle. Stored as `tb_deepen_requests (owner, attempt_id,
   question_id, created_at)` (or a `deepen_flagged bool` on `tb_session_questions`).
   Pure capture — no content, no cost.
2. **Read — agent fetches the attempt.** `GET /v1/attempts/{id}` returns the
   ordered questions **with their deepen flags**, so the agent knows exactly which
   to work. Only resolvable post-submit, so the answer key is legitimately
   visible (existing session-state gate).
3. **Generate + store — agent writes the note.** New write tool `deepen.write`
   (own scope) via `POST /v1/agents/run`, upserting
   `tb_deepen_notes (owner, question_id, attempt_id, content_md, generated_by_agent,
   created_at)`. Owner-scoped personal content. Upsert per `(question, attempt)` —
   regenerate overwrites; no version history.
4. **Consume — user reads.** `GET /v1/attempts/{id}/deepen` (and/or
   `GET /v1/me/deepen`) returns the notes; the web UI renders them per flagged
   question. Per-item state is just **flagged → (note exists?) → ready**.

#### Decisions resolved

- **Trigger: pull first, webhook later.** v1 = the user tells their agent "work my
  flagged questions"; agent fetches + writes. Enhancement = fire an existing
  **webhook on attempt submit** so the owner's agent generates proactively and
  notes are waiting. AME stays orchestration-free either way — it only stores
  flags + notes.
- **Default model is the *user's* agent's choice**, not AME's — AME hosts none.

#### Do NOT conflate with the v0.3 content-flag

v0.3's "flag this question" → `tb_content_flags` → **admin** moderation queue
("this answer key is wrong"). The deepen flag is the **same gesture, opposite
audience**: personal, learning-oriented, never reaches an admin. **Separate
stores, separate consumers.** They may share one UI affordance presented as two
choices ("Report a problem" vs. "Save to dive deeper"), but must not merge in the
schema.

#### Open

- Deepen flag as its own `tb_deepen_requests` row vs. a column on
  `tb_session_questions` — affects the migration.
- Agent-less humans: flagged questions sit note-less, falling back to A1 static
  content. Acceptable? (If not, a hosted-tutor fallback is a *separate*, later
  spec — explicitly out of scope here.)

### Rendering deepen content

Both A1.2 `deep_dive` and A2 notes are **agent-/author-generated markdown
rendered in the learner's browser** — i.e. an XSS / injection surface. One
**shared, sanitized renderer** serves both. Store **raw markdown**
(`content_md`); render client-side.

**Stack:** `react-markdown` + **Prism** (already a dep — reuse `@/lib/prismTheme`
+ `toPrismLanguage`, read-only highlight, no `react-simple-code-editor`) +
`mermaid`. Only `react-markdown` and `mermaid` are new dependencies.

| Format | v0.4 | Notes |
| --- | --- | --- |
| **Markdown + code blocks** | **Yes** | Core. `react-markdown` (no `rehype-raw` — no raw HTML, no `<script>`, no iframes); code fences highlighted by the existing Prism setup. |
| **Mermaid diagrams** | **Yes** | A ` ```mermaid ` fence renders via a custom code component. Pin the version; run `securityLevel: 'strict'` (sandboxed) — Mermaid has had XSS CVEs. |
| **Images** | **Restricted** | `https:`-only external URLs, enforced by CSP `img-src`; **no SVG** (XSS vector), **no `data:` URIs**, lazy-loaded. **No upload / blob store** — AME stores no binaries in v0.4. |
| **Animation / interactive embeds** | **Deferred** | Vague, high cost, no proven need — fails the kill-gate. Revisit only on real demand, and never as arbitrary embedded HTML. |

**Security model:** treat this content as **untrusted** even though the owner's
own agent wrote it (the agent can be prompt-injected by question content).
`react-markdown` already escapes raw HTML by default (keep `rehype-raw`
**off**); add `rehype-sanitize` with an allowlist as defense-in-depth, and the
CSP is the backstop. This renderer is built **once in A1.2** and reused by A2 —
not two pipelines.

### Phase A2.1 — Deepen-notes export to a KM tool *(good-to-have; agent-first)*

Mirror a learner's generated deepen notes into their own knowledge tool (Notion,
etc.). This is the **learner-notes** Notion direction — one-way push, per-user,
content-only — *not* Track B's two-way authoring sync. Value is **retention**
(notes live where the learner already organizes knowledge), with a thin
acquisition loop if exported notes backlink to the AME question. Kill-gate:
nothing breaks without it — defer-friendly fast-follow, not a blocker.

- **v1 = zero AME code.** The same agent that generates the note (A2) writes it to
  Notion via the agent's own Notion access. AME's only jobs: notes stay fetchable
  (already true via `GET /v1/attempts/{id}/deepen`) + a stable per-note URL to
  backlink. Ship as an `llms.txt` playbook ("generate deepen notes, then mirror to
  your Notion"), not a feature.
- **Native in-app export** (in-app "Connect Notion" OAuth + AME pushes blocks) is
  the heavier path — only build it if **agent-less humans** are a target (the same
  open question as A2). Markdown→Notion-blocks maps cleanly; still one-way.

---

## Track B — Notion authoring pipeline *(different axis; defer-friendly)*

**Chosen direction:** authoring pipeline (workspace-level), not learner-notes.
Authors maintain canonical question content in a Notion workspace; it syncs with
the bank. Two-way.

This is an **authoring/content** concern, not a learner one — it belongs to the
`bank/` + authoring surface, and only rides this milestone because it was
requested together. It has the highest operational risk and the least
load-bearing value (kill-gate: nothing breaks without it), so it sequences last
and can slip a release without affecting Track A.

### The hard part: two-way sync

Two-way is the classic conflict problem. The design pass must resolve, at minimum:

- **Mapping unit.** Notion page ↔ `Question`. One page per question vs. a
  database-row-per-question in a Notion DB (the latter maps far more naturally to
  structured fields — prompt, options, answer key, tags, deep-dive).
- **Versioning interaction.** Questions are versioned (draft→live→archived,
  `QuestionVersion`). Define what a Notion edit maps to: a new draft version
  pending publish? A direct live edit (dangerous)? Almost certainly **Notion edits
  land as drafts**, never auto-publishing into live assessments.
- **Conflict resolution.** Concurrent edits in Notion and in-app. Need per-side
  `updated_at` + a stored sync token / last-synced hash; last-writer-wins at
  minimum, with a conflict surfaced rather than silently clobbered.
- **Provenance.** Use `Question.source` to mark Notion-originated content and
  store the Notion page id for re-sync.
- **Auth.** Workspace-level Notion OAuth (one connection per AME owner/workspace),
  stored like other secrets (hashed/encrypted at rest — follow the webhook-secret
  precedent). Rate limits + backoff against Notion's API.
- **Trigger model.** Pull on a schedule (CronJob, mirroring backup/retention
  jobs) and/or Notion webhooks; push on in-app publish. Start **pull-only,
  dry-run** to de-risk, then enable push.

### Why pull-first, one direction at a time

Recommend the design pass scope B as **read/import first** (Notion → draft
questions), prove the mapping and the draft-not-live guarantee, *then* add the
push half. Shipping bidirectional in one shot is how the sync-drift bugs land.

---

## Sequencing

1. **A1** — Deepen panel + related-by-tag + `deep_dive` field + reference URL +
   the dual-surface read contract (`GET /v1/questions/{id}/deepen`) + the shared
   sanitized renderer. Self-contained, shippable learner feature.
2. **A2** — Async deepen notes (flag → agent-generate → store → read). Builds on
   A1's renderer + read contract. Pull-trigger first, webhook later.
3. **B** — Notion authoring pipeline. Own spec
   (`docs/specs/<date>-notion-sync.md`); pull-first, draft-only. Defer-friendly.

Per rule #1, **A1 is ready to become a task board.** A2 is specced here and
unblocks once A1's renderer + attempt-read flags land. B still needs its own
design pass before tasks.

## Non-goals

- Embedding/ML question similarity (tag overlap only).
- An AME-hosted LLM tutor; client-side model keys; any path that reveals the
  answer key pre-submission.
- A blob/upload store — deepen content references `https:` images only, stores no
  binaries.
- Animation / arbitrary interactive embeds in deepen content (deferred).
- Learner-personal Notion notes (the *other* Notion direction — not chosen).
- Notion edits auto-publishing into live assessments.
- SSO via Notion, or Notion as an auth provider.

## Open questions

- A1.3: is `Question.source` free to repurpose as the reference link, or does it
  carry authoring provenance we must keep distinct? (Check before building.)
- A2: deepen flag as its own `tb_deepen_requests` row vs. a `deepen_flagged`
  column on `tb_session_questions`.
- A2: is the A1-static fallback enough for agent-less humans, or is a hosted
  tutor eventually needed (separate, later spec)?
- B: one Notion page per question vs. a Notion database row per question.
