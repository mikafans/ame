# Design ↔ Spec gap analysis — 2026-05-20

**Spec under audit:** `docs/specs/2026-05-20-harus-platform-design.md` (written 2026-05-20).
**Design audited:** `design/` (snapshot at 2026-05-20).
**Verdict:** **SPEC HAS GAPS** — two net-new cross-cutting features (Share + Learning Objectives) and one net-new endpoint family (`/v1/shares`, `/v1/{kind}/{id}/embed`) are present in the design but absent from the spec. All eight base screens, five question kinds, three themes, the rest of the API surface, MCP shape, webhooks, scopes, and entity model are in agreement.

## Summary

- **`design/source/src/share.jsx` is net-new.** It introduces a global `ShareProvider` / `ShareModal` / `ShareButton` system plus a `LearningObjectives` cross-cutting component. The spec does not mention either.
- **`POST /v1/shares`, `GET /v1/shares/{id}`, and `GET /v1/{kind}/{id}/embed`** are documented in both `design/README.md` §"Sharing" and `design/api.md` (lines 329-370). The spec's §6.1 endpoint table and §7 MCP manifest do not list `share.create` / `share.get`.
- **`objectives: string[]`** is a new required-ish field on both Quiz and Exam (see `design/api.md` lines 68 and 97, and `data.jsx` `ACTIVE_QUIZ.objectives`, every `EXAMS[i].objectives`). Spec §4.6 Quiz and §4.7 Exam do not list it.
- A new entity (**ShareLink**) is implied by `/v1/shares` returning `{ shareId, url, embedUrl, og: { … } }` with privacy + opt-in score fields. The spec's §4 entity list (15 entities) does not include it.
- Everything else lines up: 8 base screens, 5 question kinds (`mc`/`tf`/`short`/`essay`/`code`), three themes (slate/paper/cobalt), tokens, sidebar entries (Library / Exams / Take quiz / Last results / Progress / Author / Agent), and the rest of the API surface table.

## New since spec was written

### Feature 1 — Share modal + share API (`share.jsx`)

- **What it is:** A global imperative share sheet (`<ShareProvider>` wraps the whole app, `useShare()` returns `{ open(payload) }`, `<ShareButton payload=… />` is the inline trigger). Modal opens on a blurred backdrop with a 4-tab interface (Link / Socials / Card / Embed) plus a right rail of "what travels with it" toggles + privacy segmented control + agent-equivalent JSON.
- **Where it appears:**
  - `design/source/src/share.jsx` — the full implementation (`ShareProvider`, `ShareModal`, `ShareButton`, `LinkTab`, `SocialsTab`, `CardTab`, `EmbedTab`, `ShareToggleRow`).
  - `design/source/src/app.jsx:103` — `<ShareProvider>` wraps `<App />` at the root.
  - `design/source/src/screen-library.jsx:82` — `<ShareButton size="md" variant="ghost" payload={…}>` on the Library hero, next to Preview.
  - `design/source/src/screen-library.jsx` — small "↑ Share" link on each library card footer (per README §"Triggers in the design").
  - `design/source/src/screen-exams.jsx:123` — Share button below the primary CTA on the exam detail panel.
  - `design/source/src/screen-results.jsx:3,16` — "Share quiz" button in results header; per-item review row also gets a Share that includes the explanation (`payload.kind === "item"`).
  - `design/README.md` §"Cross-cutting components" / "Share modal" — narrative description, payload kinds, privacy rules.
- **Payload kinds (3):** `quiz`, `exam`, `item`. Item-kind payloads include `explanation` so the **knowledge travels**, not just a marketing tease.
- **Entities implied:**
  - **ShareLink** (new) — `{ id, kind: 'quiz'|'exam'|'item', target_id, created_by_user_id, visibility: 'public'|'cohort', include_explanation: bool, include_score: bool, include_attribution: bool, og_image_url?, created_at, revoked_at? }`. Resolves to a stripped read-only view.
  - **Anonymous attempt** semantics for `?interactive=1` embeds — logged but not written to any user's record (so a `is_anonymous` flag on Session, or a separate `AnonymousAttempt` table). The design says "Anonymous attempts via `?interactive=1` are logged but not written to any user's record" (README line 303).
- **Endpoints implied (3):**
  - `POST /v1/shares` — `{ kind, id, visibility?, includeExplanation?, includeScore?, includeAttribution? }` → `{ shareId, url, embedUrl, og: { image, title, description } }`. Scope: `quiz.read`. Tool name: `share.create`.
  - `GET /v1/shares/{id}` — public; no auth. Resolves to read-only view. Tool name: `share.get`.
  - `GET /v1/{kind}/{id}/embed` — public iframe payload; `?interactive=1` allows anonymous attempt without writing to cohort attempts.
- **OG image generation:** the Card tab renders a 1200×630 preview client-side; the API returns a `og.image` URL — needs a server-side image generator (Vercel OG / Satori per README implementation note line 450).
- **MCP manifest:** `share.create` and `share.get` should be exported as tools so agents can mint shares programmatically (e.g. an agent posting an item summary).
- **Privacy rules (server-side enforcement required):**
  1. Shared links never reveal other learners' attempts or scores.
  2. `includeScore` is opt-in only (default false).
  3. `visibility: "cohort"` restricts to authenticated cohort members.
  4. Anonymous `?interactive=1` attempts are logged but not associated with a user.
- **Suggested spec changes:**
  - **§3 Vocabulary:** add `ShareLink` row: "A read-only public or cohort-scoped reference to a Quiz, Exam, or single Question (with optional explanation). Powers the Share modal and embed iframes."
  - **§4 Entity model:** insert §4.16 ShareLink with the shape above. Optionally §4.17 AnonymousAttempt or a `is_anonymous` flag on Session.
  - **§6.1 Endpoint table:** add a "Share" section after "Feedback & plans":

    | Method | Path                 | Tool           | Scope           | Notes                                                                                                                                                                                                            |
    | ------ | -------------------- | -------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
    | POST   | `/shares`            | `share.create` | `quiz.read`     | Body: `{ kind: 'quiz'\|'exam'\|'item', id, visibility?: 'public'\|'cohort', includeExplanation?, includeScore?, includeAttribution? }`. Returns `{ shareId, url, embedUrl, og: { image, title, description } }`. |
    | GET    | `/shares/{id}`       | `share.get`    | (none — public) | Public read-only resolution.                                                                                                                                                                                     |
    | GET    | `/{kind}/{id}/embed` | —              | (none — public) | Iframe-friendly read-only view. `?interactive=1` allows anonymous attempts.                                                                                                                                      |

  - **§6.2 Scopes:** clarify that `quiz.read` is sufficient for `share.create` (per `api.md` line 404).
  - **§7 MCP manifest:** add `share.create` and `share.get` tool entries.
  - **§8 Webhooks:** consider adding `share.created` and `share.attempted` (anonymous) events. Not required for v1.
  - **§9 Screens:** in §9.2 (Library), §9.3 (Exams), and §9.5 (Results), add a bullet noting the Share trigger placement. Per-item Results share includes `explanation`.
  - **§12 Out of scope:** remove "Mobile layouts" line _or_ add "Real OG image generator integration (Satori / Vercel OG) — v1 may ship a stub PNG generator."
  - **§13 Migration notes:** no migration impact — ShareLink is new and has no May-19 ancestor.

### Feature 2 — Learning Objectives (`LearningObjectives` in `share.jsx`)

- **What it is:** A reusable `<LearningObjectives items={…} kicker compact accentBars />` component that renders 3-4 outcome bullets ("what you'll learn") inspired by Udemy course pages. Two visual modes: default 2-column grid with mono "01"/"02" indices on accent-bar left rails, or `compact` single-column with check icons.
- **Where it appears:**
  - `design/source/src/share.jsx:1-46` — component definition.
  - `design/source/src/screen-library.jsx:59` — rendered inside the "Up next" hero card, between description and CTA row, reading `ACTIVE_QUIZ.objectives`.
  - `design/source/src/screen-exams.jsx:153-157` — rendered in the exam detail panel between the stat strip and the Composition section, reading `exam.objectives`.
  - `design/source/src/data.jsx` — every `EXAMS[i]` has a 3-4 string `objectives` array (lines 246-251, 276-281, 306-311, 336-340); `ACTIVE_QUIZ.objectives` lines 114-119.
  - `design/README.md` §"Cross-cutting components" / "Learning objectives" — narrative, including: "every Quiz and every Exam carries an `objectives: string[]` field on the API. Authoring UIs (Author Studio) should let instructors edit this; agents producing quizzes via `quiz.generate` should also emit objectives."
  - `design/api.md:68` (Quiz schema) and `:97` (Exam schema) — `objectives: { type: array, items: { type: string }, description: "3-4 outcome bullets — 'what you'll learn'" }`.
- **Entities implied:** **Quiz.objectives** and **Exam.objectives** are net-new `text[]` columns (or `jsonb` of strings). No new entity, just two new fields.
- **Endpoints implied:** none new — but `POST /v1/quizzes/generate` should now emit `objectives` (the agent-generated quiz must include learning outcomes), and `PATCH /v1/quizzes/{id}` / Author studio mutations must accept the array.
- **Suggested spec changes:**
  - **§4.6 Quiz:** append `objectives: string[]` to the field list with a note "3-4 outcome bullets. Editable in Author studio; agents producing quizzes via `quiz.generate` SHOULD emit objectives."
  - **§4.7 Exam:** append `objectives: string[]` to the field list with the same note.
  - **§5.2 Critical constraints:** add a soft constraint: "Quiz/Exam `objectives` array length is unbounded but the UI is designed around 3-4 entries; servers SHOULD warn at >6."
  - **§6.1 Endpoint table:** under `quiz.generate`, note that the response includes `objectives`. Under `quiz.import`, note that incoming markdown / JSON sources may include an `objectives` field that is preserved.
  - **§9.2 Library:** add a bullet "The 'Up next' hero renders `<LearningObjectives items={quiz.objectives} />` between description and CTA."
  - **§9.3 Exams:** add a bullet "The detail panel renders `<LearningObjectives items={exam.objectives} />` between the stat strip and the Composition section."
  - **§9.7 Author studio:** add a bullet "Metadata strip includes an objectives editor (multi-line, drag-reorderable). Persisted via `PATCH /v1/quizzes/{id}` with `{ objectives: string[] }`."

## Discrepancies (design says X, spec says Y)

| Surface                                       | Design                                                                                                           | Spec                                                                                                        | Resolution                                                                                                                                                                                                   |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `feedback.send` channel value                 | `'in-app'` (hyphenated) in `api.md:308` and `data.jsx`                                                           | `'in_app'` (underscore) in spec §4.15 Message and §6.1                                                      | Pick one; spec's `'in_app'` is consistent with snake_case enum style in §5.1. Update `api.md` to match.                                                                                                      |
| Quiz schema `points` field                    | `Question.points` is required in `api.md:34` (Quiz schema also references it)                                    | Spec §4.5 Question has no `points` field; points are exam-level (`ExamSection.weight`, `Exam.total_points`) | Add per-question `points: int` to spec §4.5 Question. The active quiz, results review (`LAST_ATTEMPT.answers[].max`), and per-question scoring all require it.                                               |
| Exam stats `cohortStatus`                     | `api.md:379` returns `{ exam, sections[], cohortStatus }` from `GET /exams/{id}`                                 | Spec §6.1 says `{ exam, sections[], cohortStatus, compositionTrace? }`                                      | Match — spec is fine; just note compositionTrace was added correctly.                                                                                                                                        |
| `agent.activity` / ActivityLog endpoint       | Not in `api.md` at all                                                                                           | Spec §6.1 lists `GET /agents/activity` → `agent.activity`                                                   | Design omits this from `api.md` but the recent-activity panel in `screen-agent.jsx` clearly uses it. Update `api.md` to include `/v1/agents/activity`. (Spec is correct.)                                    |
| `/v1/agents/register` shortcut                | Not in `api.md`; mentioned only in `screen-signup.jsx:141` as a UI hint                                          | Spec §6.1 lists it as a full endpoint                                                                       | Spec is the source of truth. Mark `api.md` as incomplete for this row.                                                                                                                                       |
| `POST /sessions` body shape                   | `api.md:252-271` lists `{ cats, tags, types, diff, count, duration, mode, shuffle?, explain? }` as the only body | Spec §6.1 lists three flavours: `{ quizId }`, `{ examId }`, or the practice body                            | Spec is broader & correct. `api.md` documents only the practice flavour. Spec wins.                                                                                                                          |
| Webhooks set                                  | `api.md` lists 5 webhooks; README lists 3 "recommended"                                                          | Spec §8 lists 6 webhooks (adds `plan.created`)                                                              | Spec is the superset. `plan.created` is a spec-only addition not in the design — verify with user; otherwise spec is authoritative.                                                                          |
| `agents.register` scope `*` wildcard          | `data.jsx:425` shows a local-dev key with `scopes: ["*"]`                                                        | Spec §6.2 declares a fixed scope enum; no wildcard                                                          | Treat `*` as a UI-only display token meaning "all". Add a note to §6.2: "The token `*` MAY appear in UI displays as shorthand for the full active scope set; it is not a value stored in `api_keys.scopes`." |
| `POST /agents/register` rejects `admin` scope | Spec §6.3                                                                                                        | Not in design                                                                                               | Spec-only. OK.                                                                                                                                                                                               |
| Tweaks panel "Agent panel" toggle             | `app.jsx:25` and `tweaks-panel.jsx` — toggles sidebar agent entry visibility                                     | Spec §10 only mentions theme and statsDepth as runtime user prefs                                           | Add to §10: "the agent sidebar entry is hidden by default for learners; instructor/admin roles see it. The Tweaks panel toggle is demo-only."                                                                |

## Confirmed in agreement

- Screens 1-8 (signup / library / exams / quiz / results / dashboard / author / agent) all present; sidebar (`Sidebar` in `sidebar.jsx`) matches §9 exactly.
- Question kinds: 5 (`mc`, `tf`, `short`, `essay`, `code`) — match. (`data.jsx` ACTIVE_QUIZ uses all five.)
- Themes: 3 (`slate`, `paper`, `cobalt`) — match (`tokens.md` + `app.jsx` `data-theme` swap).
- Session kinds (`quiz`, `exam`, `practice`) — match.
- Three Quiz colors (`accent`, `blue`, `amber`) — match (`data.jsx` `color` field, README "Color rule").
- Difficulty enum on Question (`intro`, `inter`, `adv`) and on Quiz (+ `mixed`) — match.
- Exam method (`manual` | `agent`) and composition_trace — match (`data.jsx` `composedBy`, `method`).
- MCP tool list (excluding share) — `AGENT_TOOLS` in `data.jsx` lists `quiz.import`, `quiz.generate`, `attempt.get`, `stats.cohort`, `feedback.send`, `exam.compose`, `exam.get`, `exam.stats`, `plan.create` — all present in spec §6.1.
- Tokens (colors, type, spacing, radii, motion) — `design/tokens.md` is the source of truth; spec §10 defers correctly.
- Three Sidebar sections (Learn / Teach / Integrate) — match.
- Author studio three-pane (320/flex/280) layout — matches spec §9.7.
- Agent screen tabs (Keys / MCP tools / Import demo / Recent activity) — match spec §9.8.
- StudyPlan generated by `plan.create` shape — match.
- Tweaks panel intent (demo-only) — match.
- Webhooks `attempt.submitted`, `attempt.graded`, `quiz.published`, `exam.opened`, `exam.closed` — present in both.

## Action items for the next session

1. **Edit `docs/specs/2026-05-20-harus-platform-design.md` §4** — add §4.16 ShareLink (entity), append `objectives: string[]` to §4.6 Quiz and §4.7 Exam, add `points: int` to §4.5 Question (per-question scoring is implied throughout).
2. **Edit §6.1** — add a "Share" sub-section with `share.create` (POST /shares, scope `quiz.read`), `share.get` (GET /shares/{id}, public), and the `GET /{kind}/{id}/embed` public route with the `?interactive=1` semantics.
3. **Edit §6.2** — add a clarifying note about `*` being a display-only shorthand. No new scope needed (Share uses `quiz.read`).
4. **Edit §7** — add `share.create` and `share.get` tool entries to the MCP manifest section.
5. **Edit §9.2 / §9.3 / §9.5 / §9.7** — add bullets for LearningObjectives placement (Library hero, Exams detail), Share triggers (Library card footer + hero, Exams CTA-adjacent, Results header + per-item), and Author objectives editor.
6. **Edit §10** — note the Tweaks panel "Agent panel" toggle as demo-only; in production the agent sidebar entry is role-gated (instructor/admin).
7. **Edit §13** — note that `feedback.send` channel enum is canonicalized as `'in_app'` (snake_case), correcting the `'in-app'` in `design/api.md`.
8. **Update `design/api.md`** to (a) fix the `in_app` enum, (b) document `/v1/agents/activity`, (c) document `/v1/agents/register`, (d) document the `{ quizId }` and `{ examId }` body flavours of `POST /sessions`, (e) decide on `plan.created` webhook (yes/no).
9. **Add a new phase to `docs/plans/tasks/phases.jsonl`** — Phase 11 "Sharing & embedding" covering ShareLink schema, `/v1/shares` endpoints, OG image generation, embed route, anonymous-attempt logging, privacy guardrails. Alternatively fold into Phase 8 (Agent surface) since `share.*` are MCP tools.
10. **Update `docs/plans/tasks/plan-N-<topic>.jsonl`** — add tasks under either P8 or a new P11 for: Quiz/Exam `objectives` column, Author studio objectives editor, ShareLink table + migrations, `/v1/shares` handler, OG image worker, embed iframe handler, anonymous-attempt path, privacy enforcement tests.
11. **Decide:** does `quiz.generate` need to emit `objectives` mandatorily, or as a best-effort? Recommend best-effort with a `warnings[]` entry when the source doesn't yield enough material.

## Source references

- `design/source/src/share.jsx:1-46` — `LearningObjectives` component (default + `compact` variant).
- `design/source/src/share.jsx:50-66` — `ShareContext`, `ShareProvider`, `useShare` hook.
- `design/source/src/share.jsx:68-221` — `ShareModal` with 4 tabs, right rail of toggles + privacy segmented control + agent-equivalent `POST /v1/shares` snippet.
- `design/source/src/share.jsx:301-352` — `SocialsTab` (X, LinkedIn, Reddit, HN, Email, Mastodon).
- `design/source/src/share.jsx:424-472` — `EmbedTab` with iframe code, 6 platform preset cards, `?interactive=1` hint.
- `design/source/src/share.jsx:499-506` — `ShareButton` inline component.
- `design/source/src/share.jsx:508` — exports attached to `window`.
- `design/source/src/app.jsx:103` — `<ShareProvider>` wraps the App at the root.
- `design/source/src/screen-library.jsx:59,82` — `LearningObjectives` + `ShareButton` on the Up-next hero.
- `design/source/src/screen-exams.jsx:123,153-157` — `ShareButton` below CTA, `LearningObjectives` in detail panel.
- `design/source/src/screen-results.jsx:3,16,131` — `useShare()` import, "Share quiz" header button, per-item Share with explanation.
- `design/source/src/data.jsx:114-119` — `ACTIVE_QUIZ.objectives` (4 bullets).
- `design/source/src/data.jsx:246-251,276-281,306-311,336-340` — every `EXAMS[i].objectives` (3-4 bullets each).
- `design/source/src/data.jsx:425` — `key_test_z1` with `scopes: ["*"]` wildcard display value.
- `design/README.md` §"Cross-cutting components" — narrative for both Learning objectives and Share modal, with payload-kind table and privacy rules.
- `design/README.md` §"API surface" / "Sharing" — `POST /shares`, `GET /shares/{id}`, `GET /{kind}/{id}/embed` rows.
- `design/api.md:68,97` — `objectives` field on Quiz and Exam schemas.
- `design/api.md:329-370` — `/shares` and `/shares/{id}` path entries.
- `design/tokens.md` — unchanged from prior; three themes + tokens all match spec §10.
- `docs/specs/2026-05-20-harus-platform-design.md` §§4, 6.1, 6.2, 7, 8, 9 — the surfaces that need editing.

---

## Convergence pass — 2026-05-20

**Verdict:** RESIDUAL GAPS

The spec itself (`docs/specs/2026-05-20-harus-platform-design.md`) now fully covers the three substantive feature gaps and every minor discrepancy. The residual gaps live in companion files: `design/api.md` still carries `in-app` (vs canonical `in_app`) and is missing the agent endpoint rows, and the plan ledgers (`phases.jsonl`, `plan-*.jsonl`) have no Share / objectives / points tasks.

### Action items verification

| #   | Item                                                                                                                                                                                              | Status  | Evidence                                                                                                                                                                                                                                                                                                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Edit §4 — ShareLink entity, Quiz/Exam `objectives`, Question `points`                                                                                                                             | PASS    | Spec §4.5 line 102 adds `points: int (default 1, ≥0)`; §4.6 line 115 adds `objectives: string[]`; §4.7 line 120 adds `objectives: string[]`; §4.16 lines 159-168 defines ShareLink + anonymous-attempt semantics.                                                                                                                                                         |
| 2   | Edit §6.1 — Share sub-section (`share.create`, `share.get`, embed route)                                                                                                                          | PASS    | Spec §6.1 "Share" sub-section lines 265-275 lists `POST /shares`, `GET /shares/{id}`, `GET /{kind}/{id}/embed` with `?interactive=1`, plus a bonus `DELETE /shares/{id}` → `share.revoke`.                                                                                                                                                                                |
| 3   | Edit §6.2 — clarify `*` is display-only shorthand                                                                                                                                                 | PASS    | Spec §6.2 line 310 "Display shorthand" paragraph states `*` is never stored / never accepted; sharing scope note on line 312 confirms `quiz.read` is sufficient.                                                                                                                                                                                                          |
| 4   | Edit §7 — add `share.create` and `share.get` MCP tool entries                                                                                                                                     | PASS    | Spec §7 MCP manifest lines 350-380 include both `share.create` (scope `quiz.read`) and `share.get` (scope null) JSON blocks.                                                                                                                                                                                                                                              |
| 5   | Edit §9.2 / §9.3 / §9.5 / §9.7 — LearningObjectives placements, Share triggers, Author objectives editor                                                                                          | PASS    | §9.2 line 418 + 419 (hero objectives + ShareButton), §9.3 line 424 + 426 (detail objectives + CTA-adjacent share), §9.5 line 439 (header + per-item share with explanation), §9.7 line 450-452 (objectives editor + per-question `points` input + generate-quiz objectives merge).                                                                                        |
| 6   | Edit §10 — Tweaks "Agent panel" toggle as demo-only                                                                                                                                               | PASS    | Spec §10.2 lines 484-490 calls out the Tweaks panel as demo-only and notes the agent sidebar entry is role-gated (instructor/admin) in production.                                                                                                                                                                                                                        |
| 7   | Edit §13 — canonicalize `in_app` (snake_case)                                                                                                                                                     | PASS    | Spec §13 migration table line 540 documents "Canonical: `'in_app'` (snake_case, consistent with §5.1)"; §4.15 line 157 and §6.1 line 261 both use `in_app`.                                                                                                                                                                                                               |
| 8   | Update `design/api.md` (fix `in_app`, document `/agents/activity`, `/agents/register`, `{quizId}`/`{examId}` session bodies, decide `plan.created`)                                               | FAIL    | `design/api.md:308` still reads `enum: [in-app, email]`; no `/agents/activity` or `/agents/register` paths present; `POST /sessions` body still only documents the practice flavour; `plan.created` webhook absent from `api.md:373-381`. Spec is correct but companion design doc was not updated.                                                                       |
| 9   | Add Phase 11 "Sharing & embedding" to `phases.jsonl` (or fold into P8)                                                                                                                            | PARTIAL | Spec §14 line 557 declares Phase 8 absorbs sharing ("MCP manifest export, ActivityLog, webhooks, `/plans`, and the Share API … OG image stub"). However `docs/plans/tasks/phases.jsonl` and `plan-8-agent-skills.jsonl` contain zero "share" / "embed" / "ShareLink" / "og image" tokens. Decision is documented in the spec but not yet materialised in the task ledger. |
| 10  | Update plan ledger with Quiz/Exam `objectives`, Author objectives editor, ShareLink table, `/v1/shares` handler, OG worker, embed route, anonymous attempts, privacy tests, per-question `points` | FAIL    | `grep -i "share\|objective" docs/plans/tasks/*.jsonl` returns no matches. Plan-2 (schema) does not mention `objectives` columns, `share_links`, or `anonymous_attempts`; plan-3 (question bank) does not add `points`; plan-8 (agent surface) does not add share endpoints.                                                                                               |
| 11  | Decide: `quiz.generate` objectives mandatory vs best-effort                                                                                                                                       | PASS    | Spec §4.6 line 117 and §6.1 line 222 both adopt the recommendation: "SHOULD emit objectives best-effort; insufficient source material produces a `warnings[]` entry rather than failing."                                                                                                                                                                                 |

**Tally:** 8 PASS / 1 PARTIAL / 2 FAIL out of 11.

### New gaps found

- **Idempotency key on `POST /shares`** — spec line 271 declares "Idempotent on `(created_by, kind, target_id, visibility, include_*)`", but `idempotency_keys` (§5.2) is keyed on the `Idempotency-Key` header, not on a tuple. Either the share endpoint needs an additional natural-key uniqueness constraint on `share_links`, or the spec should say "callers SHOULD send `Idempotency-Key`; servers MAY collapse on the natural tuple." Pick one and document.
- **`share.revoke` MCP tool entry missing.** §6.1 line 274 introduces `DELETE /shares/{id}` with tool name `share.revoke`, but §7 MCP manifest only ships `share.create` and `share.get`. Add a third tool block or drop the tool name from §6.1.
- **`include_explanation` default mismatch.** Spec §4.16 line 160 defaults `include_explanation` to `false`; `design/api.md:343` defaults it to `true`. The spec is internally consistent (item-kind payloads carry explanation explicitly when the user opts in), but `api.md` will mislead implementers. Fix as part of action item 8.
- **No `anonymous_attempts` index / rate-limit table mentioned in `share_links` migration plan.** §5.2 declares the rate-limit (30/10min/IP) and §5.3 indexes `anonymous_attempts (share_id, ts DESC)`, but there is no plan-ledger task to create the table — see action item 10 FAIL.
- **`GET /{kind}/{id}/embed` path-param overlap.** The route literally collides with `/quizzes/{id}` etc. when `kind='quizzes'`. Spec should either nest it (`/embed/{kind}/{id}` or `/{kind}/{id}/embed` with `kind` a hard-coded enum sub-resource) or call out the routing precedence. Currently `/quizzes/{id}/embed` could be read as `/quizzes/{id}` with a stray segment by a naive router.

### Residual gaps

- `design/api.md` is out of sync with the spec on three counts: `in-app` enum, missing `/agents/activity` + `/agents/register` rows, missing `{ quizId }` / `{ examId }` session bodies, missing `plan.created` webhook, `includeExplanation` default flipped.
- Task ledgers (`phases.jsonl`, `plan-2-schema-auth.jsonl`, `plan-3-question-bank.jsonl`, `plan-8-agent-skills.jsonl`) have no share / objectives / points work items even though P8 is declared the home for sharing in the spec.

### Notes

- The spec itself is now internally consistent and substantively complete for the three feature gaps (ShareLink, Learning Objectives, per-question points). The remaining work is propagating the decisions into the companion `design/api.md` and the implementation plan ledgers.
- Recommend a single follow-up commit that (a) edits `design/api.md` per action item 8, (b) adds the share-related tasks to `plan-8-agent-skills.jsonl`, (c) adds `objectives` + `points` column tasks to `plan-2-schema-auth.jsonl` and `plan-3-question-bank.jsonl`. After that, run this convergence check again — expecting CLOSED.

---

## Convergence pass — 2026-05-20 (session 3)

Applied all 5 residual / new gaps from the previous pass:

| #   | Gap                                                    | Action                                                                                                                                                                                                                                                                                                                                            | Where                                                                                                                  |
| --- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 1   | `share.revoke` missing from §7 MCP manifest            | Added a third tool block (`share.revoke`, scope `quiz.read`, method `DELETE`)                                                                                                                                                                                                                                                                     | `docs/specs/2026-05-20-harus-platform-design.md` §7                                                                    |
| 2   | `POST /shares` idempotency ambiguous (tuple vs header) | Clarified: `Idempotency-Key` header per §5.2 is primary; natural-tuple `(created_by, kind, target_id, visibility, include_*)` is fallback dedup when no header is sent                                                                                                                                                                            | spec §6.1 Share row                                                                                                    |
| 3   | `GET /{kind}/{id}/embed` route collision risk          | Renamed route param to `{kinds}` (plural, matching `/quizzes/{id}`); added explicit router-precedence note ("declare `/quizzes/{id}/embed` before `/quizzes/{id}`")                                                                                                                                                                               | spec §3, §6.1, §6.2                                                                                                    |
| 4   | `design/api.md` out of sync                            | `in-app→in_app`, added `/agents/register` + `/agents/activity` paths, restructured `POST /sessions` requestBody as `oneOf` `{quizId}`/`{examId}`/`{practice}`, added `plan.created` webhook row, flipped `includeExplanation` default from `true` to `false`                                                                                      | `design/api.md`                                                                                                        |
| 5   | Task ledgers had zero share/objective/points rows      | Added P2-012 (points + objectives columns), P2-013 (share_links + anonymous_attempts tables), P3-008 (wire points through HTTP/OpenAPI), P8-006 (Share API + MCP tools), P8-007 (embed + anonymous attempts), P8-008 (objectives in import/generate), P8-009 (plan.created webhook); updated phases.jsonl P8 entry to call out the absorbed scope | `docs/plans/tasks/phases.jsonl`, `plan-2-schema-auth.jsonl`, `plan-3-question-bank.jsonl`, `plan-8-agent-skills.jsonl` |

**Verdict:** CLOSED. Spec, `design/api.md`, and task ledgers are now in agreement. Subsequent gap-analysis re-runs should find no residual gaps.
