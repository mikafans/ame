# AI-1.2 — Act-as-owner resolution in auth

**Phase:** 1 · **Lane:** api · **Agent:** sonnet · **Depends on:** AI-1.1, AI-0.2
**Spec:** §"Authorization → Ownership rollup / Shared truth read"
**Paths (edit only here):** `api/src/auth/`

## Objective
Give every authenticated request a single **owner id** to aggregate over:
- a human/admin: `owner_id == user.id`;
- an agent: `owner_id == its owner_user_id`.
Downstream code uses `owner_id` for "my content", quotas, and shared-truth reads.

## Read first
- `api/src/auth/extractor.rs` — `AuthenticatedUser` and its query (joins
  `tb_api_tokens` → `tb_users`). You'll extend the SELECT to also fetch
  `u.owner_user_id`.

## Do
- Add `owner_id: Uuid` to `AuthenticatedUser` (computed:
  `owner_user_id.unwrap_or(user.id)`).
- Provide a small helper the handlers can call, e.g. `fn owner_id(&self) -> Uuid`.
- Keep the existing token verification (sha256 constant-time) untouched.

## Gotchas
- This only **adds** a field; do not change how scopes are parsed or how tokens
  verify. AI-1.3 builds the create-agent endpoint on top of this; AI-3.x and
  AI-4.x consume `owner_id`.
- An agent reading level/progress must read the **owner's** rows — bake the
  intent into the helper name/doc so AI-2.2 uses it correctly.

## Done when
- `AuthenticatedUser.owner_id` resolves correctly for human and agent tokens.
- A unit test covers the agent→owner mapping. `make check` passes.
