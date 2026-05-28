# AME — Principal-SWE Audit

**Date**: 2026-05-28
**Branch**: `feat/mui-migration` @ `6f0dd52`
**Scope**: full repo, open-source readiness, security, code health
**Reviewer**: claude-opus-4-7 (session)

## TL;DR

The codebase is technically solid for a solo project but reads as **internal-tool-published-on-GitHub**, not as a project an outside contributor can safely land work on. The biggest gaps are not bugs — they are the absence of the OSS substrate (CI, contributor docs, container artifacts, security policy) plus a handful of latent prod foot-guns (DEMO_MODE, argon2-per-request, unauthenticated agent registration, permissive CORS). Docs drift is widespread after the MUI migration.

---

## 1. OSS hygiene — mostly missing

| Item | State |
|---|---|
| LICENSE | ✅ GPLv3, 34 KB |
| README | ⚠️ Exists but stale (see §3) |
| CONTRIBUTING.md | ❌ |
| CODE_OF_CONDUCT.md | ❌ |
| SECURITY.md | ❌ — no disclosure channel |
| CHANGELOG.md | ❌ |
| `.github/` (workflows, ISSUE_TEMPLATE, PULL_REQUEST_TEMPLATE) | ❌ none |
| Releases / tags | none |
| Badges (CI, license, version) | none |

For a public repo this is the first thing maintainers and reviewers look for. `make validate` is enforced only by a local opt-in hook (`make hooks-install`); a drive-by PR can land broken.

## 2. CI/CD — entirely absent

No `.github/workflows`, no GitLab/Drone/Woodpecker config. The only quality gate is `.githooks/pre-commit` which contributors must explicitly install. Recommendations, in order of impact:

1. GitHub Actions matrix: `make check` + `make test-db` (services: postgres) + `make e2e`.
2. `cargo audit` / `cargo deny` advisory job — currently no dependency-vulnerability surface.
3. `openapi:check` job — guards the generated TS schema (`bun run api:check`) against drift.
4. Pin Rust / bun / uv versions via `mise.toml` already; surface them in workflow.

## 3. Documentation drift (post-MUI migration)

- `README.md:5` says **Tailwind 3** — repo now uses **MUI 7** (`@mui/material`, no Tailwind in `package.json`).
- `README.md:3` says **Next.js 16** — fine — but `AGENTS.md:7` says **Next.js 15**. Drift.
- `README.md:11–15` Quick start uses `make dev-env` — Makefile target is `make dev` (line 149). `make dev-env` doesn't exist.
- `README.md:11` says `make db-up` "requires Colima" — true on macOS, but project is dev'd on Linux/podman; the Makefile auto-detects podman (`Makefile:3`).
- `AGENTS.md:7` says "One deployment; `/admin/*` is role-gated" but route layout is `(learner)/...` — no `/admin` segment exists in `web/app/`. Either remove the claim or build the admin shell.
- `docs/plans/` has 17 plan docs through `2026-05-27`; none reference the MUI migration outcome. Add a short "what shipped" closeout for the migration plan.
- `docs/specs/2026-05-19-question-exam-platform-design.md` (38 KB) is marked historical only in AGENTS.md — move under `_archive/` to prevent agents from mining it.

## 4. Security findings

### 4a. `DEMO_MODE=1` env bypass — high severity
`api/src/auth/extractor.rs:32` — if `DEMO_MODE=1` is set at runtime, **every request** becomes an Instructor with the nil UUID, no token required. A misconfigured k8s manifest or a leaked dev secret turns the API into an anonymous admin console. Fixes:
- Gate behind a compile-time `#[cfg(feature = "demo")]`, not a runtime env var.
- At minimum, refuse to start in `--release` if `DEMO_MODE=1`.

### 4b. Argon2 verification on every request — perf + DoS
`api/src/auth/extractor.rs:80` + `auth/token.rs:26`. Every authenticated request runs `Argon2::verify_password` against the stored hash. Argon2 default params cost ~50–200 ms of CPU. This is:
- A per-request latency tax most apps don't pay.
- A trivially-exploited CPU-DoS vector (concurrent requests with garbage tokens still hit verify).

Standard pattern: API tokens use HMAC-SHA256 of the secret (constant-time compare), with Argon2 only for **password** hashing. Or maintain an in-memory verified-token LRU.

### 4c. Token cookie is browser-readable
`web/src/hooks/useAuth.ts:23` — `document.cookie = "ame_token=...; SameSite=Lax"`. No `Secure`, no `HttpOnly`. XSS exfiltration risk. `HttpOnly` requires moving fetches through Next.js route handlers; until then at least set `Secure` in prod and add a CSP header in `next.config.mjs` (currently empty).

### 4d. `POST /v1/agents/register` is unauthenticated
`api/src/http/agents.rs` registers in `auth::router` (logged/non-logged split in `http/mod.rs:71`). Anyone on the internet can mint an API key. AGENTS.md/spec calls this intentional, but for any public deployment this is an open key faucet. At minimum: rate-limit + require an invite/signup token.

### 4e. CORS = `Any/Any/Any`
`api/src/http/mod.rs:60–63`. Fine for dev, must be tightened (or env-gated) before prod.

### 4f. No rate limiting anywhere
`/v1/auth/login`, `/v1/auth/register`, `/v1/agents/register` all unlimited — brute-force exposure.

### 4g. No auth password recovery / no MFA
Schema (`tb_users.password_hash`) supports only email+password. Acceptable for v1, but document it in SECURITY.md.

## 5. API code health

- **Hotspot files**: `http/sessions.rs` 1176 LoC, `http/quizzes.rs` 862, `http/me.rs` 820, `http/agents.rs` 716. Each is doing routing + handlers + DTOs + DB queries. Extract per-route submodules; this will pay back the next time anyone touches them.
- **49 `.unwrap()` matches across 9 files** in `api/src` (TODO/FIXME count: 0 — good). Most are likely in tests or after `Scope::from_str` of constants, but worth a sweep + `#[deny(clippy::unwrap_used)]` on `src/` (allow on `tests/`).
- **`stats/mod.rs` is 1 byte** — dead module declaration. Remove or populate.
- **`/healthz`** returns `{status: ok}` without touching the DB pool (`http/mod.rs:84`). Add `/readyz` that does a `SELECT 1` so k8s liveness/readiness can differ.
- **No request-id middleware** — `TraceLayer` is on, but logs have no correlation. Add `tower_http::request_id`.
- **`ApiError::Internal(e.into())`** discards source context. Wire `tracing::error!(error=?e)` in the `IntoResponse` impl.
- **`tokio::spawn` for `last_used_at` update** (`extractor.rs:90`) silently drops failures; fine but mark with a comment so a future reader doesn't think the write is observed.
- **Background tasks** generally are spawn-and-forget — no JoinSet, no shutdown drain. On SIGTERM, in-flight `last_used_at` writes are lost. Document trade-off or wire a shutdown channel.
- **`api/_archive/`** — dead directory in repo root. Remove.
- **`reqwest` is in both `[dependencies]` and `[dev-dependencies]`** (`api/Cargo.toml:36, 44`) — collapse to a single entry.

## 6. Database & migrations

- Single migration `20260521200000_squash.sql` (squashed baseline) + one FTS follow-up — fine.
- `uuid_generate_v7()` defined in plpgsql instead of using the `pg_uuidv7` extension. Works, but adds runtime overhead; Postgres 18 has it natively via `uuidv7()`. Worth a one-line swap.
- No documented migration policy in AGENTS.md (only an inline shell rule). Add: "additive only after squash; never edit a migration once applied; sqlfluff format must run **before** sqlx records the checksum."
- No backup/restore docs. Production runs on k8s — where are dumps stored, retention?
- `tb_api_tokens.scopes` CHECK constraint enumerates scopes inline; if scopes grow, every change requires a migration. Acceptable, but document the source-of-truth split with `domain/user.rs::Scope`.

## 7. Frontend

- **Auth is pure client-side**. `useAuth.ts` runs in a `useEffect`, fetches `/v1/me`. Means every protected page shows a flash of unauthenticated content (or "Loading"). Move auth to a Server Component layout that reads the cookie server-side; this also unlocks `HttpOnly`.
- **`app/(learner)/` group holds non-learner pages** (`author/`, `grading/`, `questions/`). Misleading name now that instructor flows live there. Rename the group.
- **`fetch` is mixed with `openapi-fetch`** — `useAuth.ts` uses raw `fetch`; the rest uses the generated client. Standardize.
- **No global error boundary, no toaster** — destructive actions use MUI Dialogs (per CLAUDE.md) but failure UX is unclear.
- **`tsconfig.tsbuildinfo` is 240 KB and checked in?** Verify it's gitignored (`.gitignore:9` is `*.tsbuildinfo` — good, just confirm).

## 8. Testing

- `api/tests/` integration tests gated behind `AME_RUN_DB_TESTS=1`. Good. But `make check` silently skips them — a contributor running `make check` may believe everything passed when DB tests never ran. Print a clear `[skipped: AME_RUN_DB_TESTS=1 not set]` line.
- `api_tests/` (pytest, black-box) exists but isn't part of `ci` target. Wire it.
- No property tests beyond `proptest` dep declared in `Cargo.toml` — used in `engine/elo.rs` only; consider if the dep is paying for itself.
- E2E auth race was fixed in commit `c5e244f` — capture the lesson in `docs/` not just commit message; the memory entry on this is gold but invisible to outside contributors.
- No load / perf tests. For a quiz platform, even a one-shot `k6` script against `/v1/sessions/{id}/answer` would surface the argon2 issue.

## 9. Build, deploy, runtime

- **No `Dockerfile` for the API** and **no Dockerfile for the web** — README says "production runs in Kubernetes" but the repo can't be built into an image. This is the single biggest blocker for any external user to try the platform. Add multi-stage Dockerfiles + a `docker-compose.prod.yml` or k8s manifests under `deploy/`.
- `make dev` pipes API logs through `tee` while running in the background — works locally, but the chained `&` + `tee` will keep the foreground attached to the frontend. Consider `tmux`/`overmind` style, or document `make stop` more loudly.
- `.envrc` / `.env.example` missing. Contributors must guess env vars (`DATABASE_URL`, `NEXT_PUBLIC_API_URL`, `DEMO_MODE`, `API_HOST`, `NEXT_ALLOWED_ORIGINS`, `RUST_LOG`, `AME_RUN_DB_TESTS`). Add `.env.example` with all of them.
- No `flake.nix` despite "Nix-first" personal default — `mise.toml` only. Either go full Nix (devShell) or drop the Nix-first language from the user-level CLAUDE.md.

## 10. Observability

- Tracing → stdout only. No metrics (`/metrics` Prometheus endpoint), no OpenTelemetry exporter, no structured JSON option for prod.
- No request log sampling; chatty endpoints (e.g. `/v1/me` every page load) will drown signal.
- No SLO docs.

## 11. Process / repo signals

- `feat/mui-migration` is 2 commits ahead of origin, untouched PR. The branch name + size suggests it should land via PR for visibility — but with no CI and no PR template, the project loses the review surface.
- 17 plan docs in `docs/plans/` — useful history but no `INDEX.md`. After 6 months an agent will not know which plan is current. Add a one-line status table.
- `.agents/` and `.claude/` are session scratch; `.gitignore` excludes some but not all (`.claude/` is untracked, OK; `.agents/sessions/` is checked in via `.agents/sessions/question-bank-fts.md`). Decide and document.

---

## Phased remediation plan

1. **OSS substrate** — `.github/workflows/ci.yml` running `make check` + `make test-db` + `make e2e`, plus `cargo audit`. `CONTRIBUTING.md`, `SECURITY.md`, `.env.example`, PR template. Update README to match current stack.
2. **Container & deploy artifacts** — API + web Dockerfiles, `docker-compose.prod.yml`, smoke-deploy doc.
3. **Security hardening** — kill `DEMO_MODE` runtime env, swap argon2-on-every-request for HMAC token check, set `Secure` cookie + CSP, env-gated CORS, rate-limit auth + agent registration.
4. **Auth-on-the-server refactor** — move `useAuth` to a server-side cookie read; enables `HttpOnly` and removes the loading-flash.
5. **Code health pass** — split `sessions.rs` / `quizzes.rs` / `me.rs`, kill empty `stats/mod.rs` and `_archive/`, `#[deny(clippy::unwrap_used)]`.
6. **Observability** — `/readyz`, request-id, Prometheus `/metrics`, structured JSON logs behind `LOG_FORMAT=json`.
