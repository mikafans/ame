# Plan 2: Schema & Auth

**Goal:** Establish the Postgres database schema using `sqlx`, set up the connection pool, and implement the Bearer token authentication + idempotency middlewares in Axum.

**Prerequisites:** Plan 1 (Foundation) completed.

---

## Task 1: Sqlx setup & Initial Migration

- [ ] **Step 1: Add DB dependencies to `api/Cargo.toml`**

  ```toml
  [dependencies]
  sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "time", "json"] }
  uuid = { version = "1", features = ["v7", "serde"] }
  time = { version = "0.3", features = ["serde"] }
  argon2 = "0.5"
  async-trait = "0.1"
  ```

- [ ] **Step 2: Create initial migration**
  Run `mise exec -- sqlx migrate add init --source db/migrations`.
  Copy the exact schema from `docs/specs/2026-05-19-question-exam-platform-design.md` (the "Data Model" section) into `db/migrations/<timestamp>_init.sql`.
- [ ] **Step 3: Run the migration**

  ```bash
  export DATABASE_URL=postgres://postgres:postgres@localhost:5432/ame
  mise exec -- sqlx migrate run --source db/migrations
  ```

- [ ] **Step 4: Application State**
  In `api/src/http/mod.rs`, define `AppState` containing the `sqlx::PgPool`. Update `router()` to accept `pool` and `.with_state(AppState { pool })`.

## Task 2: Domain Types (Auth & Shared)

- [ ] **Step 1: `domain/user.rs`**
  Define `User`, `Role` (enum: Admin, User), `ApiToken`, `Scope`.
- [ ] **Step 2: `domain/error.rs`**
  Implement the `ApiError` enum mapping to HTTP status codes exactly as specified in the spec. Implement `axum::response::IntoResponse` for it.

## Task 3: Auth Middleware

- [ ] **Step 1: `auth/token.rs`**
  Write utilities to parse `Authorization: Bearer <token>` and verify `argon2id` hashes against the `api_tokens` table.
- [ ] **Step 2: Axum Extractor (`auth/extractor.rs`)**
  Implement `axum::extract::FromRequestParts` for an `AuthenticatedUser` struct. It should extract the bearer token, query the DB, update `last_used_at` (async/background task to prevent blocking, or direct), and return the User and Token scopes.
- [ ] **Step 3: Scope Guard Middleware**
  Create a middleware/extractor `RequireScope<const S: &'static str>` that ensures the `AuthenticatedUser` has the required scope.

## Task 4: Idempotency Key Middleware

- [ ] **Step 1: `http/idempotency.rs`**
  Implement an Axum middleware that looks for the `Idempotency-Key` header.
  - If missing, proceed normally.
  - If present, check `idempotency_keys` table.
  - Same key + same hash -> return stored `response_body` and `response_status`.
  - Same key + different hash -> return `409 idempotency_conflict`.
  - On successful downstream response, save to `idempotency_keys`.

## Task 5: Testing & Validation

- [ ] **Step 1: Unit/Integration tests**
  Write tests in `api/tests/auth.rs` verifying token rejection, scope rejection, and idempotency key replay behavior.
- [ ] **Step 2: Verify `make check`**
  Run `make check` to ensure formatting and linting pass.
