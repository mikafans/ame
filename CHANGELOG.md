# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project is pre-1.0; releases are not yet tagged.

## [Unreleased]

### Added

- UI Styling: Added sticky header and skeleton loading to Results table.
- Sidebar: Updated branding title to "AME".

### Changed
...
- OSS substrate: GitHub Actions CI, PR template, `CONTRIBUTING.md`, `SECURITY.md`,
  `CODE_OF_CONDUCT.md`, `.env.example`.
- Container artifacts: API and web Dockerfiles, prod-shape `docker-compose.prod.yml`.
- Per-IP rate limiting (`tower_governor`) on `/v1/auth/*` and `/v1/agents/register`;
  tunable via `AME_RATELIMIT_BURST` / `AME_RATELIMIT_PERIOD_SECS`.
- Windowed progress stats: `GET /v1/me/stats?window=last30d|last90d|all`.

### Changed

- Migrated the frontend to MUI 7; removed Tailwind.
- API token storage moved from Argon2 to `sha256(secret)` with constant-time compare;
  password hashing still uses Argon2.
- Auth now uses a session cookie (`Secure` in production); CSP and security headers
  shipped via `web/next.config.mjs`.
- `POST /v1/agents/register` gated behind the `AME_AGENT_ACCESS_CODE` secret;
  registration is disabled when the var is unset.
- CORS is env-gated via `AME_CORS_ORIGINS` (defaults to `http://localhost:3000`).
- Dev ports moved to API `:28080` / web `:23000` to avoid collisions.

### Removed

- `DEMO_MODE=1` runtime auth bypass — deleted entirely.
- Stale `ui/` Dioxus scaffold and the empty `api/src/stats` placeholder module.
