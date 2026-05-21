# ame

Personal question collector + exam platform. Rust (Axum) backend, Next.js frontend, Postgres in Kubernetes for production.

## Quick start

```bash
mise install                            # install rust + bun runtimes
make db-up                              # start Postgres in Docker (requires Colima)
make db-migrate                         # run pending migrations
make dev-env                            # start API on :8080 and frontend on :3000
uv run scripts/seed.py                  # (optional) seed demo users, quizzes, and questions
```

## Local Database & Docker Setup

Postgres requires a Docker daemon. On macOS, this project recommends [Colima](https://github.com/abiosoft/colima).

```bash
# Start colima (docker daemon)
colima start

# Start the local database
make db-up

# Check database status
docker compose -f db/docker-compose.yml ps

# When finished working, bring down the database and daemon to save resources
make db-down
colima stop
```

To clean up persistent database data and start entirely fresh:
```bash
make db-down
rm -rf db/data/
make db-up
```

- `docs/specs/` — design specs.
- `docs/plans/` — implementation plans (executed sequentially).
- `api/` — Rust backend.
- `web/` — Next.js frontend.
- `db/` — docker-compose + sqlx migrations.

See `AGENTS.md` for the working conventions.