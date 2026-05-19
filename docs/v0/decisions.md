# Design Decisions Log

Brainstorming session: 2026-05-19. Recap of Q→A choices that locked in the design.

| # | Decision | Choice | Notes |
|---|---|---|---|
| 1 | MVP scope | Bank → quiz → agent gen → level readout (items 1–4 of 6-step ladder) | Items 5 (spaced repetition) and 6 (adaptive coach) explicit follow-ups |
| 2 | Old Dioxus scaffold | Scrap it | User: "the old project was not my concern anymore" |
| 3 | Targets | Backend + web only | No desktop/mobile MVP |
| 4 | Stack | Axum (Rust) backend + Next.js (TypeScript) SPA | Standard SPA stack chosen over Dioxus fullstack |
| 5 | Frontend framework | Next.js (React) | Largest ecosystem, best LLM training coverage |
| 6 | Deployment | Solo on NAS, accessed via Tailscale; admin panel + main app share one Next.js deployment with `/admin/*` role-gated | Friends-later baked into schema (users table from day 1) |
| 7 | Storage | Postgres (Docker on NAS) | User chose Postgres over SQLite |
| 8 | Question content types | MCQ + free-text + cloze; "code questions" = regular kinds with optional `code_snippet` field | Avoids duplicating mcq/free_text logic for code variant |
| 9 | Topic organization | Flat tags with `:` namespace (`rust:async`, `spanish:a2`) | Pseudo-hierarchy without committing to a tree |
| 10 | Agent integration | REST ingestion only (backend never holds Claude API key) | External agents POST questions; `agents/` skill bundle in repo |
| 11 | Level model | Elo per tag + curriculum-label mapping (Approach C of A/B/C) | Self-calibrating + interpretable |
| 12 | Token hashing | argon2id | User confirmed |
| 13 | Schema discipline | Pragmatic — 4 CHECK constraints only (kind, status, scopes, lowercased tag name) | Dropped paranoid range checks |
| 14 | Numeric types | `double precision` for Elo (not `numeric`); uuid v7 for PKs | Performance + index locality |
| 15 | Exam feature | First-class object with static or dynamic blueprint, time limit optional, hides per-answer feedback by default, updates Elo by default | Closer to user's "know my real level" motivation than quizzes |
| 16 | Versioning | Question versioning (snapshot prior version on edit; attempts pin version) | From Moodle prior art |
| 17 | Per-attempt presentation | Record shuffled option order in `attempts.presentation` jsonb | Required for stable grading of shuffled MCQ |
| 18 | Calibration phase | First 20 attempts on a new question: only question rating moves, K=48; protects user rating from agent-misestimated difficulty | From Pelánek/Antal research |
| 19 | Module split | `bank/` `engine/` `assess/` `stats/` `auth/` `http/` `domain/` | Moodle-inspired internal layering |
| 20 | Deferred features | FSRS, Glicko-2, xAPI emission, QTI import/export, LLM-judge, magic-link auth, hints, per-question timers, audit log | Schema is forward-compatible with each |

## Open questions (left for spec to make decisions on, not raised in chat)

- Default question count per quiz: 10
- Default time limit for exams when unset: none
- Idempotency-Key TTL: 24h
- Quiz session "stale" cutoff: auto-abandon after 24h with no answers
- Max batch size for `POST /questions`: 50 per call
- Static exam blueprint behavior when a pinned question is archived: exam auto-marked `status='draft'` until re-curated

These were called out during brainstorming as design questions; defaults locked in the spec.
