# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project is pre-1.0; releases are not yet tagged.

## [Unreleased]

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
