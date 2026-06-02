# AME Project Rules

## Task Runner

**Always use `make` — never raw commands.**

- `make dev` — start the full stack (kills stale procs, migrates, starts API + frontend)
- `make stop` — stop everything
- `make check` — pre-commit gate (fmt-check + lint + test)
- `make e2e` — Playwright tests
- `make db-seed` — seed demo data (requires API running)

Never run `npm run dev`, `bun run dev`, `cargo run`, etc. directly. Always go through Make.

When running one-off JS/TS tools outside of Make targets, use `bunx` not `npx`/`npm`.
For Python tools, use `uvx`. For JSON processing, use `jq`.
For Playwright CLI specifically, use `bunx @playwright/cli`.

## Dev/Deploy Workflow

After making code changes, **always restart the dev server** via `make dev` to pick up changes.

## Stack

- **API**: Rust (axum) — `make dev` serves on `:28080` (override with `API_PORT`; the bare `ame-api` binary defaults to `AME_PORT` 8080)
- **Frontend**: Next.js (bun) — `make dev` serves on `:23000` (override with `WEB_PORT`)
- Access remotely via Tailscale: `make dev API_HOST=harus-mini` → http://harus-mini:23000
- **DB**: Postgres via docker/podman compose (`db/docker-compose.yml`)
- **Schema**: OpenAPI at `api/openapi.yaml`; regenerate with `make openapi`

## UI Conventions

- Use MUI Dialog for confirmations — never `window.confirm`
- Destructive actions (delete, discard) require a MUI Dialog with Cancel + red confirmed action button
- All pages use MUI components — no Tailwind for new UI work
- Placeholder buttons (no backend) must be removed, not left as no-ops

## Data Model Constraints

- MC question options are **bare strings** (`Vec<String>`) — never `{text: "..."}` objects. The grader and frontend both expect this format.
- Scheduled exams are not supported — no `opens_at`/`closes_at` fields in the API or DB.
- SSO/SAML is not implemented — login supports email + password only.
- Quiz lifecycle: draft → active (publish auto-promotes draft questions to live).
- **Admin is DB-granted only.** Registration always creates `role: user` (`POST /v1/auth/register` rejects `role: admin`); there is no in-app privilege escalation. Grant admin with `make db-admin` (a direct SQL write — `make db-admin ADMIN_EMAIL=...` to promote a specific account, preserving its password). `db-seed` depends on `db-admin` because seeding needs an admin to upgrade the primary user (Ada) to premium (premium unlocks the agent-creation quota that `make db-bulk` relies on). Re-login after promotion — token scopes are fixed at login.
- Sharing/public-visibility was removed — no `visibility` field or `public.publish` scope anywhere. Cross-account access is owner-scoped (sub-accounts), not public sharing.

## Shell Habits

- **Never `cd` before a command** — cwd is always the repo root. Use `make <target>` or absolute paths.
- **Search with `rg`, find with `fd`** — never `grep -r` or `find`.
- **Read files directly** — if `rg` gives you a path, use the Read tool. Never pipe into `xargs rg`.
- **Long `make` output** — redirect to a file (`make check > /tmp/check.log 2>&1`) then Read it. `grep` on rtk-truncated output silently misses content.
- **Migration checksum mismatch** (sqlfluff reformatted a migration after it was applied) — fix with `make db-reset` then `make dev`, not manual DB surgery.
- **JS runtime** — use `bun`, never `node`. Playwright: `bunx @playwright/cli`.
- **Verifier scripts** — write to `.claude/scripts/` for reuse, run with `bun .claude/scripts/<name>.js`.

## Conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `deploy:` — no emojis
- Ask before committing or pushing
- 2-space indentation for config files
