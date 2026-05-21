## Tasks

You are implementing Task #7 in the ame project: add two quiz endpoints to the Rust/Axum backend.

### Context
- Branch: `feat/design-catchup`
- Stack: Rust (edition 2021), Axum 0.8, sqlx, Postgres 18
- Read `AGENTS.md` at the project root for conventions
- Read `api/src/http/quizzes.rs` — this is the file you will edit (add two new handlers to it)
- Read `api/src/http/mod.rs` to see how routers are wired
- The `quizzes` table schema: read the latest migration files in `db/migrations/` that mention `quizzes`

### 1. POST /v1/quizzes — create a quiz (add to `api/src/http/quizzes.rs`)

Handler `create_quiz`:
- Requires auth (use the same `AuthUser` extractor as other handlers in quizzes.rs)
- Body: `{ "title": str, "course"?: str, "difficulty"?: str, "duration"?: i32, "objectives"?: [str] }`
- Insert a new row into `quizzes` with `status = 'draft'`, `created_by = user.id`, and the provided fields
- Return 201: `{ "quizId": uuid, "quiz": { ...full quiz row } }`
- Only instructors/admins may create quizzes; return 403 for learners

### 2. GET /v1/quizzes/count — live item count for quiz setup (add to `api/src/http/quizzes.rs`)

Handler `count_quizzes`:
- Query params: `cats` (comma-separated), `tags` (comma-separated), `diff` (difficulty), `types` (comma-separated question kinds)
- Runs a `SELECT COUNT(*) FROM quizzes WHERE ...` using the same filter logic already present in the list handler (look at how `list_quizzes` or similar filters — copy the WHERE clause pattern)
- Returns 200: `{ "count": N }`
- Auth required (same token check)

### 3. Wire both into the quiz router

In `quizzes::router(...)`, add:
```rust
.route("/v1/quizzes", post(create_quiz))        // alongside existing GET
.route("/v1/quizzes/count", get(count_quizzes)) // NEW — MUST be declared before /v1/quizzes/:id
```

## Important notes
- Read the FULL `api/src/http/quizzes.rs` before editing — understand existing patterns
- The count route `/v1/quizzes/count` must be registered BEFORE any `/v1/quizzes/:id` catch-all route to avoid routing conflicts in Axum
- Check `db/migrations/` for the quizzes table columns before writing the INSERT — only insert columns that exist
- If `objectives` column doesn't exist in `quizzes`, skip it silently (don't fail the migration; just omit the field from the INSERT)
- Do NOT commit. Run `cd api && mise exec -- cargo build 2>&1 | tail -30` and report changes + any errors.

## Done when
- `create_quiz` handler is implemented in `api/src/http/quizzes.rs`
- `count_quizzes` handler is implemented in `api/src/http/quizzes.rs`
- Both routes are wired in the quiz router (count route before /:id catch-all)
- `cd api && mise exec -- cargo build 2>&1 | tail -30` passes with zero errors
