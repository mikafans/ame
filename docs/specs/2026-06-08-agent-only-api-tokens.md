# Agent-only API tokens + token type marker

Status: draft
Date: 2026-06-08
Branch: feat/valkey-logins (or a follow-on branch)

## Motivation

After the login-sessions epic, a human's real authentication lives in
`tb_login_sessions` (Postgres source of truth + fail-open Valkey cache). The
older `tb_api_tokens` table now serves two unlike things:

- **agents** — `agent_id` set; the only live consumer (created via
  `POST /v1/me/agents`).
- **human personal access tokens (PATs)** — `user_id` set; minted via
  `POST /v1/me/keys`.

The human-PAT surface is **dead**: `/v1/me/keys` has zero callers (no
frontend, no scripts, no e2e). It only survives because the *integration test
harness* fabricates an authenticated human by inserting directly into
`tb_api_tokens (user_id, …)` — the pre-login-sessions shortcut.

Two costs follow from the untyped, dual-purpose design:

1. **Wasted resolution work.** Tokens are `{uuid}_{secret}` with no type marker
   (`api/src/auth/token.rs`). The extractor can't tell a login session from an
   agent token, so every request probes in order: `ame:login` cache →
   `ame:token` cache → `tb_login_sessions` → `tb_api_tokens`. Every agent
   request pays a wasted Valkey GET and, on a cold cache, a wasted
   `tb_login_sessions` query before reaching its real row.

2. **Tests don't exercise the real human path.** ~30 sites mint humans through
   `tb_api_tokens`, a path the product no longer exposes.

## Target model

- **`tb_api_tokens` is agent-only.** `agent_id NOT NULL`; `user_id` column,
  the polymorphic CHECK, and the `user_id`-based partial index are gone.
- **Humans authenticate only via `tb_login_sessions`.** No human PAT path.
- **Tokens carry a type marker.** Format becomes `lgn_{uuid}_{secret}` for a
  login session and `agt_{uuid}_{secret}` for an agent token. The extractor
  parses the marker and routes directly to the one cache key + one table:
  - `lgn_` → `ame:login:{id}` cache, `tb_login_sessions`
  - `agt_` → `ame:token:{id}` cache, `tb_api_tokens`
  No cross-probing. The two cache namespaces stay (different TTL/lifecycle:
  login = sliding `login.ttl_seconds`; agent = 30 s read cache).
- **`/v1/me/keys` (list/create/rotate/delete) is removed** — handlers, DTOs,
  routes, and OpenAPI registration.

## Decisions

- Prefix marker on the token string (user-approved over a single cache
  namespace or no marker). Clean break — there are no real users yet, so no
  legacy-token compatibility window. The parser **requires** a known prefix;
  unprefixed tokens are rejected (force re-login / re-mint).
- Secrets are hex (no `_`); the parser still tolerates `_` in the secret
  segment by splitting only the first two `_` (`kind`, then `uuid`, then the
  rest is the secret).

## Scope of change

| Area | Change |
| --- | --- |
| `api/src/auth/token.rs` | `ParsedToken { kind: TokenKind, id, secret }`; `parse_token_value` requires `lgn_`/`agt_`; format helpers emit the prefix. |
| `api/src/http/auth.rs` | `issue_token` emits `lgn_…`. |
| `api/src/http/me.rs` | agent token mint emits `agt_…`; **delete** `list_keys`/`create_key`/`rotate_key`/`revoke_key` + their DTOs + routes. |
| `api/src/auth/extractor.rs` | route on `ParsedToken.kind`; drop the dual-probe; `tb_api_tokens` query becomes agent-only (no `COALESCE(t.user_id, t.agent_id)`, no `LEFT JOIN tb_users u ON t.user_id`). |
| `api/src/http/admin.rs` | token-audit list/count is agent-only (join `tb_agents`, drop the `user_id` COALESCE/join). |
| `api/src/http/openapi.rs` | drop the `/v1/me/keys` paths + the `*Key*` schemas. |
| `api/openapi.yaml`, `web/src/api/generated/schema.d.ts` | regenerate (drift gate; stage together with the Rust change). |
| `db/migrations/<new>.sql` | `agent_id SET NOT NULL`; drop `user_id` (and FK), drop `chk_api_tokens_polymorphic`, drop `tb_api_tokens_user_active`; add an agent-active index. |
| `api/tests/*` (~30 sites) | humans authenticate via a `tb_login_sessions` insert (shared helper); agent inserts keep `agent_id`. |

## Execution — TDD, sequential

Red-first per slice; one agent at a time; lead reviews each diff; commit per
slice. Order smallest → biggest, dependencies respected.

1. **Token marker** (lib-level). Unit tests: `parse_token_value` accepts
   `lgn_…`/`agt_…` and rejects unprefixed. Implement `TokenKind`, update mint
   sites to emit prefixes.
2. **Remove `/v1/me/keys`** (independent). Test: `POST /v1/me/keys` → 404.
   Delete handlers/DTOs/routes/openapi; regenerate spec + schema.
3. **Extractor routing on kind.** Test: a login token never resolves against
   `tb_api_tokens`; an agent token never against `tb_login_sessions`. Drop the
   dual-probe and the human branch of the `tb_api_tokens` query.
4. **Test-harness migration.** Switch every human `tb_api_tokens(user_id)`
   insert to a login session via a shared helper. Suite stays green.
5. **DB migration + agent-only queries.** Drop `user_id`/CHECK/index, set
   `agent_id NOT NULL`, simplify extractor + admin token-audit. `make ci`
   green end to end.

## Non-goals

- No new human API-key UI. If humans ever need scripting tokens, that is a
  separate, deliberate feature on top of `tb_login_sessions`, not a revival of
  `tb_api_tokens.user_id`.
