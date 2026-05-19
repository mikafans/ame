# ame

Personal question collector + exam platform. Rust (Axum) backend, Next.js frontend, Postgres in Kubernetes for production.

## Quick start

```bash
mise install                       # rust + bun
nix develop                        # everything else (or use direnv)
make db-up                         # start Postgres
cd api && cargo run &              # backend on :8080
cd web && bun install && bun run dev   # frontend on :3000
```

## Where things are

- `docs/specs/` — design specs.
- `docs/plans/` — implementation plans (executed sequentially).
- `api/` — Rust backend.
- `web/` — Next.js frontend.
- `db/` — docker-compose + sqlx migrations.

See `AGENTS.md` for the working conventions.