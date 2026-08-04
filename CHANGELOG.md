# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project is pre-1.0. Releases are tagged as `vX.Y.Z` starting with 0.4.0.

## [0.4.1] - 2026-08-03

### Added

- Runtime-served agent contracts at `/public/llms.txt`, `/public/skill.json`,
  `/public/openapi.yaml`, and `/public/learning-contract.json` through one
  unified `/public/*` runtime route.
- A downloadable, self-contained Python SDK at
  `/public/sdk/python/ame.py` for discovery, authentication, onboarding,
  generic typed-contract requests, and rate-limit status.
- Visible authenticated rate-limit status at `/api/v1/me/rate-limit`, standard
  `X-RateLimit-*` response headers, and dynamic `Retry-After` guidance.
- Free and VIP/Premium user plans, with admin-panel plan changes and separate
  owner-scoped rate-limit tiers. Free defaults are 600 burst / 10 tokens per
  second; Premium is 10x at 6000 / 100.
- A footer in the authenticated app shell (About, Agent API, GitHub,
  Self-hosting) — previously only reachable from the logged-out landing page,
  so a signed-in learner had no way back to the GitHub repo or self-hosting
  guide without logging out.

### Fixed

- Delegation handoff URLs now use the configured public origin or trusted
  request forwarding headers instead of hardcoded `localhost:28800`.
- `QuestionOption` and agent-authored content use the canonical lower camelCase
  wire convention, including `isCorrect`, `keyPoints`, `altText`, and
  `correctOptionId`; unknown content keys are rejected.
- Public unauthenticated traffic remains a strict per-IP limiter, while
  authenticated journey authoring uses the free/VIP owner bucket.
- An admin account could not start a catalog-onboarded journey ("Start this
  path" 500'd): the identity lookup behind it excluded any non-`learner`-role
  account. Admin accounts can now use the same self-serve onboarding flow as
  any learner.
- Catalog/prompt-bootstrapped journeys (every "Reviewed starting path" and
  freeform onboarding journey — anything short of the agent-authored
  course-revision workflow) showed a spurious "That item is no longer
  available" error banner on load. The 404 from a journey with no published
  course revision is expected there and already has a dedicated fallback
  view; it no longer surfaces as an error.
- Clicking a reviewed catalog path while already signed in silently
  redirected to `/agent`, discarding the selection. A signed-in learner can
  now start an additional journey on their existing account without
  re-registering.
- `/start?catalogId=...` showed a blank "what would you like to learn?" box
  with no indication of which reviewed path was picked. It now shows the
  catalog entry's title, description, and estimated time, and pre-fills the
  prompt (still editable).
- A relative link in `learning-principles.md` resolved to a different URL
  depending on which page rendered it (404'd at `/about`, e.g. from `/`).
  Fixed to an absolute `/public/...` path.
- Two admin endpoint doc summaries still said `/v1/...` instead of
  `/api/v1/...`, propagated into the served OpenAPI spec and generated
  TypeScript client types.
- `api/Dockerfile`, `deploy/k3s/Dockerfile.api`, and `api/Dockerfile.dev` (the
  `make dev` local stack) didn't copy the workspace `crates/` members and/or
  `sdk/python`, breaking every containerized build path for this release.
- `deploy/k3s/build-images.sh` now builds and imports images directly into
  local containerd instead of pushing to a registry, matching the local-first
  self-host workflow the k3s manifests already expected.

### Quality gates

- `docs/public/**/*.md` is now covered by Prettier (`proseWrap: never`,
  matching the no-manual-wrapping convention used elsewhere) and every Python
  file (`scripts/`, `sdk/python/`, `api_tests/`) by `ruff format`/`ruff
  check`. Previously neither had any automated formatting or linting, which
  is how the `learning-principles.md` link bug above went unnoticed.

### Breaking route documentation

The 0.4 route split is explicit: authenticated endpoints are under
`/api/v1/*`, unauthenticated endpoints are under `/public/v1/*`, and the old
`/v1/*` surface is not supported. Reverse proxies must send both `/api/` and
`/public/` to the API; discovery documents are served under `/public/`.

## [0.4.0] - 2026-08-03

### Added

- Agent-first course authoring: immutable source imports and snapshots,
  citation certification with grounding and licensing status, generation-run
  provenance, and course revisions (draft, publish, fork) built from
  objectives, chapters, activities, and versioned questions/assessments.
  Publish validation rejects a generic or incomplete shell as publishable.
- Delegated agent handoffs — a learner mints a scoped, revocable `dlg_*`
  capability from their own browser session (`/agent`) instead of ever
  sharing a raw learner bearer token with an agent.
- Two source-grounded reference courses (Apache Flink, Netty), authored and
  published end-to-end through the public agent workflow with no source-code
  access required.
- A course-scoped learner workspace: a "My Learning" multi-course library
  (In progress / Completed) where each course keeps its own Learn, Course,
  Progress, and Sources tabs, with explicit, isolated course switching.
- Study Atelier native catalog: eight reviewed starting journeys (learning
  science, Rust, Python, SQL, linear algebra, technical writing, Apache Flink,
  classical mechanics), four persistent semantic themes, and a learner-facing
  catalog with stable IDs, outcomes, and source/review summaries.
- Retention and source-grounded learning: FSRS spaced review scheduling,
  immutable source imports/snapshots, citation/licensing/grounding
  certification, version-anchored learner notes, reviewed explanation/
  example/difficulty variants, richer submissions with rubric-based revision
  review, journey history export/import, and learner retention analytics.
- An in-app About page rendering the learner-facing learning contract,
  reachable as a persistent nav tab alongside Learning desk and Admin.
- One containerized local dev stack behind Caddy (`make dev`) — replaces the
  previous split between a native host-process stack and a separate
  containerized stack, which had drifted into two independent databases.

### Fixed

- A reviews/due host-vs-database clock-skew race, a double-normalized
  analytics average score, and a citation-UUID-as-URL bug that hid a
  source-backed activity badge.
- Cross-shell navigation dead ends: Learning desk was unreachable from any
  `/admin/*` page, and a nav tab used a plain anchor instead of a client-side
  link, forcing a full page reload.
- A layout gap that silently rendered as zero pixels because the Tailwind
  utility never compiled, and the "start a new course" entry point
  disappearing once a learner had any course.
- Desktop sidebar staying above page content so its sign-out action remains
  reliably accessible.

### Changed

- Toolchain: the Nix devShell was replaced with mise-managed tools.

## [0.3.0] - 2026-07-02

### Added

- Deep Dives Personal KB Workspace: Transforms flat explanation requests into a personal, organized study knowledge base.
  - Evolved `/deep-dives` dashboard into a premium two-pane layout with category and tag filters, full-text query search, and Markdown-based ZIP export.
  - Refactored `/deep-dives/[id]` workspace into a split 70/30 layout with read-only agent content, a sticky study notes editor (with debounced autosaving), and a side-drawer showing revision timeline logs.
  - Implemented backend API routes for creating, updating, listing, and exporting deep dives:
    - `GET /v1/deep-dives` supporting `?category` and `?search` queries.
    - `PATCH /v1/deep-dives/{id}` supporting `userNote` and `status` updates.
    - `GET /v1/deep-dives/{id}/revisions` to retrieve explanation and notes revision snapshots.
    - `GET /v1/deep-dives/export` generating an Obsidian, Logseq, and Notion-compatible Markdown ZIP archive.
  - Created database migrations adding `category`, `user_note`, and `note_updated_at` to `tb_deep_dives`, alongside the snapshotting `tb_deep_dive_revisions` table.
  - Integrated agent tools `deepDive.request`, `deepDive.list`, and `deepDive.publish` with auto-categorization capabilities.

## [0.2.1] - 2026-06-23

### Changed

- k3s image build now defaults to `linux/amd64` (x64) and is platform-parameterized
  (`PLATFORM=linux/arm64 deploy/k3s/build-images.sh` still builds the arm64 edge
  image). `Dockerfile.api` derives its Rust musl target from `TARGETARCH`, so the
  same Dockerfile cross-builds either arch.

## [0.2.0] - 2026-06-15

### Added

- Learn Deeper: a per-question "Dive deeper" review surface on the session
  results page — author-written `deep_dive` study notes (sanitized markdown via a
  shared `MarkdownView` renderer with Prism + Mermaid), an external reference
  `source` URL, and related-questions-by-tag navigation with back-history. Shows
  an empty-state when a question has no extra material.
- `GET /v1/questions/{id}/deepen` — dual-surface read endpoint (plus an agent
  read tool) returning the question payload, deep-dive, source, and related
  questions, so a learner's own agent can consume the same deepen content.
- Authoring UI: `deep_dive` and `source` inputs with live markdown preview.
- Exposed new agent tool: `question.update` to support remote question modifications.
- Exposed new agent tools: `assessment.archive`, `assessment.publish`, and `assessment.delete` to the agent platform (run-door, scope `assessment.write`).
- Added keyboard shortcuts (`1`-`9`, `Enter`, `F` to flag) and a desktop sidebar navigation panel to the quiz session page.
- Designed a side-by-side comparative correction panel for incorrect answers on the session results review page, featuring PTS chip animations and code exemplar highlighting.
- Added operator-tunable settings and feature flags backed by DB/Valkey cache.

### Fixed

- Fixed question list pagination returning incorrect `next_cursor` leading to infinite loops.
- Fixed question list ignoring the `assessmentId` query parameter.
- Added a `Retry-After: 1` header to 429 Too Many Requests responses.
- Consolidated agent-runner error formats, mapping unknown tools to HTTP 400 Bad Request.
- Fixed average answer time tracking (`avgTimeMs`) in assessment statistics.
- Fixed correct answer leaks in attempts list and session response endpoints prior to session completion.
- Re-confined agent tokens to a read-only REST allowlist plus the `/v1/agents/run` dispatcher; all mutating tools are scope-checked at the dispatcher, closing a direct-REST scope-bypass on assessment writes and sub-account creation.

## [0.1.0] - 2026-06-08

### Added

- Assessment authoring + grading engine: unified `assessment` entity
  (structure + `practice`/`graded` mode), multi-section authoring, draft→active
  lifecycle, server-enforced session deadlines.
- Question bank: MC / TF / short / essay / code questions with full-text search
  and keyset pagination (benched at 1M rows).
- First-class agent surface: per-user agent sub-accounts (`POST /v1/me/agents`),
  scoped tokens, the `/v1/agents/run` dispatcher, and `skill.json` / `openapi.json`
  discovery.
- Admin console foundation: users (promote/demote with no-lockout guards), audit
  log, system health, assessment moderation (soft-delete + restore).
- Plans (`free` / `premium`) with per-plan quotas, premium data export
  (`GET /v1/me/export`), and tiered per-owner rate limiting (`tower_governor`,
  token-keyed; tunable via `AME_RATELIMIT_BURST` / `AME_RATELIMIT_PERIOD_SECS`).
- Windowed progress stats: `GET /v1/me/stats?window=last30d|last90d|all`.
- OSS substrate: GitHub Actions CI, PR template, `CONTRIBUTING.md`, `SECURITY.md`,
  `CODE_OF_CONDUCT.md`, `.env.example`.
- Container artifacts: API and web Dockerfiles, prod-shape `docker-compose.prod.yml`.

### Changed

- Collapsed roles to `{user, admin, agent}`; agents are token-only sub-accounts
  owned by a human user.
- Migrated the frontend to MUI 7; removed Tailwind.
- API token storage moved from Argon2 to `sha256(secret)` with constant-time
  compare; password hashing still uses Argon2.
- Auth now uses a session cookie (`Secure` in production); CSP and security headers
  shipped via `web/next.config.mjs`.
- Row-level security on `tb_questions`; cross-account access is owner-scoped.
- Security hardening: token-scope validation, hashed webhook secrets,
  trusted-proxy client IP, per-account login throttle, email canonicalization,
  admin-token revoke on demotion, and a router-level admin guard.
- CORS is env-gated via `AME_CORS_ORIGINS` (defaults to `http://localhost:3000`).
- Dev ports moved to API `:28080` / web `:23000` to avoid collisions.

### Removed

- Legacy `quiz` / `exam` entities — unified into `assessment` (schema, code, and
  scopes).
- Public sharing / link visibility (`visibility` field, `public.publish` scope,
  `tb_share_links`) — built, then removed; cross-account access is sub-accounts only.
- Public agent self-registration faucet (`POST /v1/agents/register` +
  `AME_AGENT_ACCESS_CODE`) — agents are now owner-minted via `POST /v1/me/agents`.
- `DEMO_MODE=1` runtime auth bypass — deleted entirely.
- Stale `ui/` Dioxus scaffold and the empty `api/src/stats` placeholder module.
