# ame

A personal question collector and exam platform. Build a typed, versioned
question bank across diverse domains (Rust, Zig, Spanish, piano, …); take
quizzes and exams; let an Elo rating per tag tell you, by evidence, what
you actually know.

External LLM-driven agents (claude-code, scripts) feed questions in over a
scoped REST API and read back the user's weakest tags — closing a loop
between "what I'm bad at" and "more questions on exactly that."

## Status

**Pre-foundation.** The original Dioxus scaffold has been removed and the
design spec is locked in. The codebase will be rebuilt as:

- `api/` — Rust (Axum 0.8) backend talking to Postgres 18 via `sqlx`.
- `web/` — Next.js 15 App Router frontend (Bun runtime).
- `db/` — Postgres in Docker for local dev.
- `agents/` — agent skill bundles documenting the REST contract.

Foundation work tracked in `docs/plans/2026-05-19-foundation.md`.

## Documents

- `docs/specs/2026-05-19-question-exam-platform-design.md` — full design
  spec: schema, Elo rule, exam blueprints, API surface, frontend pages,
  testing strategy.
- `docs/plans/` — implementation plans (executed in order).
- `docs/v0/` — brainstorming notes, decision log, prior-art research.

## Deployment shape

Solo on a NAS, reachable over Tailscale. Friends-later is baked into the
schema (a `users` table from day one) but not into the UX yet.

## License

See `LICENSE`.
