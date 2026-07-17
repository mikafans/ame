# AME Project Rules

## Working Discipline

Three rules from the 2026-06 rework retro (we built ~1.5k lines of spec'd-but-unneeded
sharing/visibility code then deleted it, and landed the quiz→assessment refactor broken
then fixed it with two "repair" commits). The lesson was **not** "spec harder" — the
specs were fine. It was *validate the need* and *test-first the refactor*.

1. **Kill-gate before any new user-facing surface.** Before planning or dispatching
   tasks, answer: *"What breaks for a real user if we ship without this?"* If you can't
   answer it, defer — don't build it. Validate necessity **before** the task board, not
   after.
2. **Refactors are test-first.** Pin the contract/e2e tests against the *target* shape
   first, then migrate until green. Never land a refactor broken and clean up with
   "repair" commits afterward.
3. **Let the harness catch what discipline misses.** `make check` before every commit,
   `make ci`/`make validate` before a PR, and the OpenAPI/schema drift gate are
   non-negotiable — they exist so broken work can't land as a feature commit.

## Task Runner

**Always use `make` — never raw commands.**

- `make init-env` — one-time setup on a fresh checkout: `mise install` (rust, bun, uv) + `sqlx-cli` + web deps + Playwright browsers
- `make dev` — start the full stack (kills stale procs, migrates, starts API + frontend)
- `make stop` — stop everything
- `make check` — pre-commit gate (fmt-check + lint + test)
- `make ci` — full gate: `make check` + DB tests + e2e (resets the DB between them)
- `make test-db` — DB-backed API integration tests (requires `make db-up`; `make check` skips these unless `AME_RUN_DB_TESTS=1`)
- `make e2e` — Playwright tests
- `make db-up` / `make db-down` — start / stop Postgres
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
- **DB**: Postgres 18 via docker/podman compose (`db/docker-compose.yml`)
- **Schema**: OpenAPI at `api/openapi.yaml`; regenerate with `make openapi`
- **Toolchain**: Rust edition 2024, Axum 0.8, Tokio, sqlx; Next.js 16 / React 19 / MUI 7. `mise` provides language runtimes (rust, bun, uv). Migrations: `sqlx migrate run`; SQL lint: `uvx sqlfluff`; SQL client: `uvx pgcli postgres://postgres:postgres@localhost:5432/ame`.
- **Canonical spec**: `docs/specs/2026-05-20-harus-platform-design.md` (replaces the `2026-05-19-question-exam-platform-design.md`, kept only as historical reference). Implementation plans in `docs/plans/`.

## Architecture

- **Module boundaries** (`api/src/`): `engine/` and `assess/` may depend on `bank/` and `domain/`; the reverse is forbidden. `bank/` does not know that attempts exist.
- One deployment; role gating is enforced per-route in the API (no separate admin service).

## UI Conventions

- Use MUI Dialog for confirmations — never `window.confirm`
- Destructive actions (delete, discard) require a MUI Dialog with Cancel + red confirmed action button
- All pages use MUI components — no Tailwind for new UI work
- Placeholder buttons (no backend) must be removed, not left as no-ops

## Data Model Constraints

- MC question options are **bare strings** (`Vec<String>`) — never `{text: "..."}` objects. The grader and frontend both expect this format.
- Scheduled exams are not supported — no `opens_at`/`closes_at` fields in the API or DB.
- SSO/SAML is not implemented — login supports email + password only.
- Assessment lifecycle: draft → active (publish auto-promotes draft questions to live). Quizzes and exams are unified into the `Assessment` entity (modes: `practice` vs `graded`); endpoints consolidated under `/v1/assessments`, governed by `assessment.read`/`assessment.write` scopes.
- **Env vars** use the `AME_` prefix (`AME_DATABASE_URL`, `AME_PORT`, `AME_LOG_FORMAT`, …).
- **Timestamps**: API timestamps are RFC3339, converted to JST (UTC+9) in the HTTP layer.
- **MC feedback**: results show both the selected and correct option text with their labels (e.g. `"D: O(log n)"`).
- **Code questions**: fall back to `pending_manual` grading unless `payload.exemplar` is set; exemplar match is exact (whitespace-trimmed).
- **First admin is DB-granted; further admins can be granted in-app by an existing admin.** Registration always creates `role: user` (`POST /v1/auth/register` rejects `role: admin`) — no user can self-escalate. The *root* admin is seeded via `make db-admin` (a direct SQL write — `make db-admin ADMIN_EMAIL=...` to promote a specific account, preserving its password). Once an admin exists, they may promote/demote other users via `PATCH /v1/admin/users/{id}` (`role`); the endpoint blocks self-demotion, self-disable, and demoting/disabling the last remaining active admin (no lockout). `db-seed` depends on `db-admin` because seeding needs an admin to upgrade the primary user (Ada) to premium (premium unlocks the agent-creation quota that `make db-bulk` relies on). Re-login after a role/scope change — token scopes are fixed at login.
- Sharing/public-visibility was removed — no `visibility` field or `public.publish` scope anywhere. Cross-account access is owner-scoped (sub-accounts), not public sharing.

## Shell Habits

- **Never `cd` before a command** — cwd is always the repo root. Use `make <target>` or absolute paths.
- **Never echo exit status** — no `; echo EXIT=$?`, no `echo "exit=$status"`, no `echo $pipestatus`. The Bash tool already reports exit codes; judge success from the command's real output. Avoid shell expansions (`$?`, `$status`, `$(...)`, backticks) and `;`-chains — they defeat the permission allowlist and prompt every time.
- **No decorative `echo`** — never insert `echo "=== label ==="` separators or commentary between commands. Run the real commands plainly (one per Bash call when they're unrelated); the tool output is already labeled. Echo only when the literal text is the actual deliverable.
- **Search with `rg`, find with `fd`** — never `grep -r` or `find`.
- **Read files directly** — if `rg` gives you a path, use the Read tool. Never pipe into `xargs rg`.
- **Long `make` output** — redirect to a file (`make check > /tmp/check.log 2>&1`) then Read it. `grep` on rtk-truncated output silently misses content.
- **Migration checksum mismatch** (sqlfluff reformatted a migration after it was applied) — fix with `make db-reset` then `make dev`, not manual DB surgery.
- **JS runtime** — use `bun`, never `node`. Playwright: `bunx @playwright/cli`.
- **Verifier scripts** — write to `.claude/scripts/` for reuse, run with `bun .claude/scripts/<name>.js`.

## E2E & Visual Audit

- Full suite: `E2E_API_TOKEN=<learner-token> E2E_BASE_URL=http://localhost:23000 bunx @playwright/test test --project=chromium`.
- Interactive audit uses `@playwright/cli` (**not** `playwright-cli` — that 404s). Run from `.tmp/` (gitignored) so snapshots land there.
- **Auth pattern**: cookies don't survive `goto` — re-set on each new page: `bunx @playwright/cli cookie-set ame_token "<token>" --domain=localhost` → `reload` → `sleep 3` (async `useAuth`; screenshotting too early catches the Loading state).
- Mint a learner token: `curl -s -X POST http://localhost:28080/v1/auth/login -H "Content-Type: application/json" -d '{"email":"ada@example.com","password":"password123"}' | jq -r .token`.
- **fish + jq**: a multi-line variable piped into `jq` breaks — write to a temp file first (`curl -s <url> -o /tmp/out.json && jq '.field' /tmp/out.json`).

## Agent Surface

- `docs/public/llms.txt` is the canonical agent guide (served entry doc with the worked playbooks); reference client at `agents/client.py`.
- Discovery: `GET /llms.txt` (public) → `GET /skill.json` (public manifest).
- Mint a key: an authenticated human owner calls `POST /v1/me/agents` (token-only sub-account; `apiKey` shown once). The old `POST /v1/agents/register` faucet was removed.
- Write tools run via `POST /v1/agents/run`; read tools are called directly at their advertised method/path.

## Conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `deploy:` — no emojis
- Ask before committing or pushing
- Indentation: 2-space for config (YAML, TOML, JSON, MD); 4-space for Rust/Python; tabs for Makefile (enforced by `.editorconfig`)
- Staging discipline: `git add <specific files>` — never `git add -A` or `git add .`
