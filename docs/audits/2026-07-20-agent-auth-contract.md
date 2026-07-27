# Agent account-operation contract

This slice certifies that an external agent can discover the existing
self-host account operations before entering the unified learner API.

## Contract

The generated `docs/public/skill.json` advertises:

```text
POST /public/v1/auth/register
POST /public/v1/auth/login
```

`identity.register` accepts `email`, `name`, and a password of at least eight
characters. `identity.login` accepts `email` and `password`. Both operations
return the existing `token` and `user` response; agents use the token as
`Authorization: Bearer <token>` for `/api/v1/*` learner operations.

The canonical `docs/public/llms.txt` contains the same operations and payload
examples. Static discovery remains Caddy-served at `/public/*`; only the
`/public/v1/*` account operations are API-backed. No migration or separate
agent identity was introduced.

## TDD evidence

The red contract in commit `0c462e7` failed because the manifest and public
entry document omitted both account operations. Commit `ba05d57` added the
existing operations to the manifest, regenerated `docs/public/skill.json`,
and aligned `docs/public/llms.txt`.

Passing evidence on 2026-07-20:

- `mise exec -- cargo test --manifest-path api/Cargo.toml --test agent_manifest`
  — 2 passed;
- `uv run pytest api_tests` — 9 passed;
- `make check` — 68 Rust tests, 24 frontend tests, and generated OpenAPI
  schema parity passed;
- `make local-uiux` — 4 browser behavior tests passed.

The existing black-box registration and authenticated API test also verifies
that the returned learner token reaches `/api/v1/me`, while the retired `/v1/*`
surface and static-document-as-API path remain unavailable.
