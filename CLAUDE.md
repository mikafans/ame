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

- `make init-env` — one-time setup on a fresh checkout: web deps + Playwright browsers (language runtimes and `sqlx-cli` come from mise, not this target)
- `make dev` — start the one local stack: Postgres, Valkey, API, and web, all containerized behind Caddy at `:28800`. Auto-seeds demo accounts (`haru@example.com`, `admin@example.com`, both `password123`) on first boot. Hot-reloads on source edits (web instantly; the API container recompiles in place, cached).
- `make stop` — stop the local stack; keeps the named Postgres volume
- `make dev-reset` — wipe the local stack's Postgres volume and restart clean (use this for a migration-checksum mismatch, not `make db-reset`)
- `make check` — pre-commit gate (fmt-check + lint + test)
- `make ci` — full gate: `make check` + DB tests + e2e (resets the DB between them)
- `make test-db` — DB-backed API integration tests against a separate, bare Postgres (`make db-up`) used only by this target and `make ci` — unrelated to the `make dev` stack/database. `make check` skips these unless `AME_RUN_DB_TESTS=1`
- `make e2e` — Playwright tests (self-contained; spins up its own temporary API/web processes, independent of `make dev`)
- `make db-up` / `make db-down` — start / stop the bare Postgres used only by `make test-db`/`make ci`
- `make local-seed` / `make local-reference-courses` / `make local-haru-simulation` / `make local-learner-workspace` — extra fixtures against the running `make dev` stack (Flink/Netty reference courses, disposable simulations); `make dev` already seeds the base demo accounts on its own

Never run `npm run dev`, `bun run dev`, `cargo run`, etc. directly. Always go through Make.

When running one-off JS/TS tools outside of Make targets, use `bunx` not `npx`/`npm`.
For Python tools, use `uvx`. For JSON processing, use `jq`.
For Playwright CLI specifically, use `bunx @playwright/cli`.

## Dev/Deploy Workflow

After making code changes, **always restart the dev server** via `make dev` to pick up changes.

## Stack

- **API**: Rust (axum) — containerized by `make dev`, reachable through Caddy at `:28800` (not a separate host port; the bare `ame-api` binary defaults to `AME_PORT` 8080 if run directly, which `make dev` no longer does)
- **Frontend**: Next.js (bun) — containerized by `make dev`, also served through Caddy at `:28800` (same origin as the API and the static `/public/*` agent docs)
- Access remotely via Tailscale: the container's Caddy binds all interfaces, so a host running `make dev` is reachable at `http://harus-mini:28800` (no `API_HOST` override needed)
- **DB**: Postgres 18, containerized alongside the API/web/Caddy in `docker-compose.local.yml` for `make dev`; a separate bare Postgres via `db/docker-compose.yml` backs `make test-db`/`make ci` only
- **Schema**: OpenAPI at `api/openapi.yaml`; regenerate with `make openapi`
- **Toolchain**: Rust edition 2024, Axum 0.8, Tokio, sqlx; Next.js 16 / React 19 / Tailwind CSS 4 + shadcn/ui. `mise.toml` provides the toolchain — rust (+ rustfmt/clippy), bun, uv, python 3.14, sqlx-cli, podman-compose. Run `mise install` once; tools then activate automatically on `cd` into the repo (mise's shell hook), or explicitly via `mise x -- <command>` in non-interactive shells (e.g. `mise x -- make <target>`). `rust-analyzer` is not mise-managed — install it via your editor's Rust extension or `rustup component add rust-analyzer`. Migrations: `sqlx migrate run`; SQL lint: `uvx sqlfluff`; SQL client: `uvx pgcli postgres://postgres:postgres@localhost:5432/ame`.
- **Canonical spec**: `docs/plans/2026-07-19-agent-first-learning-rework.md` (design, user stories, contracts, and TDD matrix for the current 0.3 agent-first rework; see `docs/ROADMAP.md`). The earlier `docs/specs/2026-05-20-harus-platform-design.md` was retired once its content landed in code. Implementation plans in `docs/plans/`.

## Architecture

- **Module boundaries** (`api/src/`): `engine/` and `assess/` may depend on `bank/` and `domain/`; the reverse is forbidden. `bank/` does not know that attempts exist.
- One deployment; role gating is enforced per-route in the API (no separate admin service).

## UI Conventions

- Build UI with the shadcn/ui + Radix primitives in `web/src/components/ui/` and Tailwind CSS 4 — no MUI.
- Confirmations use a modal dialog built from those primitives — never `window.confirm`.
- Destructive actions (delete, discard) require a confirmation dialog with a Cancel and a red confirmed action button.
- Placeholder buttons (no backend) must be removed, not left as no-ops.

## Data Model Constraints

- MC question options are **bare strings** (`Vec<String>`) — never `{text: "..."}` objects. The grader and frontend both expect this format.
- Scheduled exams are not supported — no `opens_at`/`closes_at` fields in the API or DB.
- SSO/SAML is not implemented — login supports email + password only.
- Learning lifecycle: onboarding creates an owner-scoped goal, journey, objectives, and starter activities. The web app and agents use the same learner endpoints; there are no per-client capability scopes.
- **Env vars** use the `AME_` prefix (`AME_DATABASE_URL`, `AME_PORT`, `AME_LOG_FORMAT`, …).
- **Timestamps**: API timestamps are RFC3339, converted to JST (UTC+9) in the HTTP layer.
- **MC feedback**: results show both the selected and correct option text with their labels (e.g. `"D: O(log n)"`).
- **Code questions**: fall back to `pending_manual` grading unless `payload.exemplar` is set; exemplar match is exact (whitespace-trimmed).
- **First admin is DB-granted; further admins can be granted in-app by an existing admin.** Registration always creates a learner (`POST /public/v1/auth/register`) — no user can self-escalate. The *root* admin is seeded via `make db-admin` (a direct SQL write — `make db-admin ADMIN_EMAIL=...` to promote a specific account, preserving its password). Once an admin exists, they may promote/demote other users via `PATCH /api/v1/admin/users/{id}` (`role`); the endpoint blocks self-demotion, self-disable, and demoting/disabling the last remaining active admin (no lockout).
- Sharing/public-visibility is not part of the current baseline. Cross-account access is not exposed.

## Shell Habits

- **Never `cd` before a command** — cwd is always the repo root. Use `make <target>` or absolute paths.
- **Never echo exit status** — no `; echo EXIT=$?`, no `echo "exit=$status"`, no `echo $pipestatus`. The Bash tool already reports exit codes; judge success from the command's real output. Avoid shell expansions (`$?`, `$status`, `$(...)`, backticks) and `;`-chains — they defeat the permission allowlist and prompt every time.
- **No decorative `echo`** — never insert `echo "=== label ==="` separators or commentary between commands. Run the real commands plainly (one per Bash call when they're unrelated); the tool output is already labeled. Echo only when the literal text is the actual deliverable.
- **Search with `rg`, find with `fd`** — never `grep -r` or `find`.
- **Read files directly** — if `rg` gives you a path, use the Read tool. Never pipe into `xargs rg`.
- **Long `make` output** — redirect to a file (`make check > .tmp/check.log 2>&1`) then Read it. `grep` on terminal-truncated output silently misses content.
- **Migration checksum mismatch** (sqlfluff reformatted a migration after it was applied) — fix with `make dev-reset` (wipes the `make dev` stack's own Postgres volume) for the local stack, or `make db-reset` then `make test-db` for the separate bare test Postgres. Not manual DB surgery.
- **JS runtime** — use `bun`, never `node`. Playwright: `bunx @playwright/cli`.
- **Verifier scripts** — write to `.claude/scripts/` for reuse, run with `bun .claude/scripts/<name>.js`.

## E2E & Visual Audit

- Full suite (against a running `make dev`): `E2E_API_TOKEN=<learner-token> E2E_BASE_URL=http://localhost:28800 bunx @playwright/test test --project=chromium`.
- Interactive audit uses `@playwright/cli` (**not** `playwright-cli` — that 404s). Run from `.tmp/` (gitignored) so snapshots land there.
- **Auth pattern**: cookies don't survive `goto` — re-set on each new page: `bunx @playwright/cli cookie-set ame_token "<token>" --domain=localhost` → `reload` → `sleep 3` (async `useAuth`; screenshotting too early catches the Loading state).
- Mint a learner token: `curl -s -X POST http://localhost:28800/public/v1/auth/login -H "Content-Type: application/json" -d '{"email":"ada@example.com","password":"password123"}' | jq -r .token`.
- **fish + jq**: a multi-line variable piped into `jq` breaks — write to a temp file first (`curl -s <url> -o /tmp/out.json && jq '.field' /tmp/out.json`).

## Agent Surface

- `docs/public/llms.txt` is the canonical agent guide for the shared learner API.
- Discovery: `GET /public/llms.txt` (public) → `GET /public/skill.json` (public manifest).
- An agent calls `POST /public/v1/onboarding/start` with an email identifier and prompt, then uses the returned learner bearer token against `/api/v1/*`.
- Journey reads and writes are direct calls to the endpoints advertised in `/public/skill.json`; there is no run-door or agent key.

## Conventions

- Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `deploy:` — no emojis
- Ask before committing or pushing
- Indentation: 2-space for config (YAML, TOML, JSON, MD); 4-space for Rust/Python; tabs for Makefile (enforced by `.editorconfig`)
- Staging discipline: `git add <specific files>` — never `git add -A` or `git add .`
