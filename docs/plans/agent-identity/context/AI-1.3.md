# AI-1.3 — Owner-driven agent CRUD /v1/me/agents

**Phase:** 1 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-1.1, AI-0.2
**Spec:** §"API surface → Agents (owner-driven)"
**Paths (edit only here):** `api/src/http/me.rs`, `api/openapi.yaml`

## Objective
Let an owner self-serve agent sub-accounts + tokens (replacing the deleted
public faucet).

## Endpoints
- `POST /v1/me/agents {label, scopes, focusTags?}` → create the agent
  `tb_users` row (`role='agent'`, `owner_user_id=me`, email/password NULL) +
  issue a token. **Return the secret once**; persist only `sha256`.
- `GET /v1/me/agents` → list my agents (label, scopes, last_used, focus).
- `PATCH /v1/me/agents/{id}` → update label/focus/goal/next-target.
- `DELETE /v1/me/agents/{id}` → revoke token + disable the sub-account.

## Read first
- `api/src/http/me.rs` — existing token-issuing handlers; reuse the token
  helpers in `api/src/auth/token.rs` (`generate_secret`/`hash_secret`).
- `api/src/http/agents.rs` (pre-existing agent surface) for the run/tool shape,
  but do **not** edit it here — that's AI-2.2's file.

## Gotchas
- `tb_users_agent_shape` CHECK (AI-1.1) will reject an agent row that carries an
  email/password or lacks `owner_user_id` — insert accordingly.
- Scope-ceiling enforcement (agent scopes ⊆ owner plan) is **AI-4.3**, layered
  on later; here just store the requested scopes.
- camelCase JSON via `rename_all="camelCase"` (matches existing structs).
- One active token per user is the existing model — fine for agents too.

## Done when
- All four routes work; secret shown once; `make openapi` regenerated.
- `make check` passes.
