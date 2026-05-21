## Task: Add email + password authentication to the Rust/Axum backend

Branch: `feat/design-catchup`
Workspace root: `/Users/yinchun.pang/Projects/project-github/ame/.claude/worktrees/agent-a211334da1d2918bf`

Before writing any code, read the following files to understand existing patterns:
- `AGENTS.md` at the project root
- `api/src/http/mod.rs` — router structure
- `api/src/http/agents.rs` — example handler pattern (unauthenticated POST)
- `api/src/auth/mod.rs` and `api/src/auth/token.rs` — existing token system
- `api/src/domain/user.rs` — User model
- At least one recent migration in `db/migrations/` — to understand the api_keys schema

### Step 1 — Migration

Create `db/migrations/20260521100000_auth_email.sql` with:
```sql
ALTER TABLE users ADD COLUMN IF NOT EXISTS email TEXT UNIQUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash TEXT;
```

### Step 2 — Add argon2 dependency

In `api/Cargo.toml`, add if not already present:
```toml
argon2 = "0.5"
```

### Step 3 — New handler file `api/src/http/auth.rs`

Implement two handlers. Study `api/src/http/agents.rs` (or `api/src/http/me.rs`) carefully for the exact return type, error handling, and Json extractor patterns before writing.

**POST /v1/auth/register**
- Request body: `{ "email": str, "name": str, "password": str, "role": "learner"|"instructor"|"agent" }`
- Hash the password with argon2
- Insert a new row into `users` with `email`, `name`, `role`, `password_hash`
- Create an API key for the user (insert into `api_keys` table; check agents.rs for how keys are created; use `is_session = true` if that column exists in api_keys)
- Return 201: `{ "token": "<api_key>", "user": { "id", "name", "email", "role" } }`
- Return 409 if email already exists (detect unique constraint violation)

**POST /v1/auth/login**
- Request body: `{ "email": str, "password": str }`
- Look up user by email; return 401 if not found
- Verify password with argon2; return 401 if wrong
- Look up an existing session API key for this user, or create one if none exists
- Return 200: `{ "token": "<api_key>", "user": { "id", "name", "email", "role" } }`

Also add `pub fn router` to `api/src/http/auth.rs`:
```rust
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .with_state(state)
}
```

### Step 4 — Wire into router

In `api/src/http/mod.rs`:
- Add `pub mod auth;` alongside the other `pub mod` declarations
- Merge `auth::router(state.clone())` into the main router (outside the authenticated middleware block — these endpoints are public)

### Step 5 — Verify

Run: `cd api && mise exec -- cargo build 2>&1 | tail -30`

Report whether the build passes or fails, and list all files modified.

## Done when
- `db/migrations/20260521100000_auth_email.sql` exists
- `api/src/http/auth.rs` exists with `register`, `login`, and `router`
- `api/src/http/mod.rs` declares `pub mod auth` and merges `auth::router`
- `api/Cargo.toml` includes `argon2 = "0.5"` (or equivalent)
- `cd api && mise exec -- cargo build 2>&1 | tail -30` exits with zero errors (warnings are OK)
- Do NOT commit — just make the changes and report
