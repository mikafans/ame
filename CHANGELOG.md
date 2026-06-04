# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project is pre-1.0; releases are not yet tagged.

## [Unreleased]

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
