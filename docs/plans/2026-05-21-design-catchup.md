# Design Catchup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every gap between `design/` reference files and the live frontend/backend so the running app pixel-faithfully matches the design.

**Architecture:** Backend adds auth endpoints (register/login via email, no password for MVP) and a quiz-creation endpoint. Frontend is rebuilt screen-by-screen from the design JSX references using a shared atom library and correct CSS tokens.

**Tech Stack:** Rust/Axum (backend), Next.js 15 App Router + TypeScript + Tailwind (frontend), Playwright (e2e TDD), `bun test` (unit), `cargo test` (backend)

**Verification gate per task:** `make check` must pass (fmt + lint + tests). Frontend tasks add: `bun run type-check` inside `web/`.

---

## Gap Summary

| # | Area | Gap |
|---|------|-----|
| B1 | Backend | No human register/login — users table has no email; no auth endpoints |
| B2 | Backend | `POST /v1/quizzes` missing — author can't create new quizzes |
| B3 | Backend | `GET /v1/me/stats` missing — progress dashboard has no aggregated data |
| F1 | CSS | `--serif` / `--sans` / `--radius-lg` vars absent; Paper+Cobalt missing dim variants; wrong body font |
| F2 | Fonts | Source Serif 4, Inter, JetBrains Mono never loaded |
| F3 | Atoms | No shared Button/Tag/Card/Icon/Logo/KV/Stat/Divider — every page reinvents them inconsistently |
| F4 | Shell | Sidebar has no icons, no Logo, no footer avatar; Topbar component doesn't exist |
| F5 | Auth UI | Login is "paste API key" — design is two-column marketing + email/name/role form |
| F6 | Library | Wrong tab labels; no hero "Up next" card; card rows not the design's accent-stripe grid |
| F7 | Quiz | No setup screen before starting (mode/categories/tags/difficulty/count/duration) |
| F8 | Quiz | Active quiz missing: progress bar, palette rail, serif prompt, TF/Essay/Code input types |
| F9 | Results | Missing donut score card, cohort histogram, per-item Share + "See solution" row |
| F10 | Progress | Doesn't match design — needs 5-stat strip + area chart + bar chart + mastery map |
| F11 | Exams | No split list/detail; no weight bar; no composition trace panel |
| F12 | Author | 3-pane editor (questions list / editor / rail) not implemented |
| F13 | Agent | 4-tab interface (API keys / MCP tools / Import demo / Activity) not implemented |

---

## File Map

**New backend files:**
- `db/migrations/20260521100000_auth_email.sql` — add `email` column to users
- `api/src/http/auth.rs` — `POST /v1/auth/register`, `POST /v1/auth/login`

**Modified backend files:**
- `api/src/http/quizzes.rs` — add `POST /v1/quizzes` (create_quiz handler)
- `api/src/http/me.rs` — add `GET /v1/me/stats` handler
- `api/src/http/mod.rs` — wire auth router
- `api/openapi.yaml` — regenerate after changes

**New frontend files:**
- `web/src/components/ui/Button.tsx`
- `web/src/components/ui/Tag.tsx`
- `web/src/components/ui/Card.tsx`
- `web/src/components/ui/Icon.tsx`
- `web/src/components/ui/Logo.tsx`
- `web/src/components/ui/KV.tsx`
- `web/src/components/ui/Stat.tsx`
- `web/src/components/ui/Divider.tsx`
- `web/src/components/ui/index.ts`
- `web/src/components/Sidebar.tsx`
- `web/src/components/Topbar.tsx`
- `web/e2e/auth.spec.ts`
- `web/e2e/library.spec.ts`
- `web/e2e/quiz.spec.ts`
- `web/e2e/results.spec.ts`
- `web/e2e/progress.spec.ts`

**Modified frontend files:**
- `web/app/globals.css` — complete token set + font imports
- `web/app/layout.tsx` — load Google Fonts via `next/font`
- `web/app/login/page.tsx` — rebuild to two-column design
- `web/app/(learner)/layout.tsx` — swap in Sidebar component
- `web/app/(learner)/library/page.tsx` — hero card + grid rebuild
- `web/app/(learner)/practice/page.tsx` — quiz setup screen
- `web/app/(learner)/sessions/[id]/page.tsx` — active quiz rebuild
- `web/app/(learner)/sessions/[id]/results/page.tsx` — results rebuild
- `web/app/(learner)/progress/page.tsx` — dashboard rebuild
- `web/app/(learner)/exams/page.tsx` — split list/detail rebuild
- `web/app/(learner)/author/[quizId]/page.tsx` — 3-pane editor rebuild
- `web/app/(learner)/agent/page.tsx` — 4-tab interface rebuild

---

## Task 1: Backend — Auth endpoints (register + login)

**Files:**
- Create: `db/migrations/20260521100000_auth_email.sql`
- Create: `api/src/http/auth.rs`
- Modify: `api/src/http/mod.rs`
- Test: `api/tests/auth.rs` (add new test cases)

- [ ] **Step 1: Write failing test**

Add to `api/tests/auth.rs`:
```rust
#[tokio::test]
async fn test_register_and_login() {
    let Some(db) = test_db().await else { return };
    let app = ame_api::http::router(db);
    let client = TestClient::new(app);

    // Register
    let res = client.post("/v1/auth/register")
        .json(&serde_json::json!({
            "displayName": "Haru",
            "email": "haru@example.com",
            "role": "learner"
        }))
        .send().await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await;
    assert!(body["apiKey"].as_str().unwrap().len() > 10);

    // Login with same email returns a new key
    let res2 = client.post("/v1/auth/login")
        .json(&serde_json::json!({ "email": "haru@example.com" }))
        .send().await;
    assert_eq!(res2.status(), 200);
    let body2: serde_json::Value = res2.json().await;
    assert!(body2["apiKey"].as_str().unwrap().len() > 10);

    // Unknown email → 404
    let res3 = client.post("/v1/auth/login")
        .json(&serde_json::json!({ "email": "nobody@example.com" }))
        .send().await;
    assert_eq!(res3.status(), 404);
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd api && AME_RUN_DB_TESTS=1 cargo test --test auth test_register_and_login -- --nocapture 2>&1 | tail -10
```
Expected: compilation error (module not found) or 404/405.

- [ ] **Step 3: Write migration**

`db/migrations/20260521100000_auth_email.sql`:
```sql
ALTER TABLE users ADD COLUMN IF NOT EXISTS email text UNIQUE;
```

Apply:
```bash
docker exec -i ame-postgres psql -U postgres -d ame < db/migrations/20260521100000_auth_email.sql
```
Expected: `ALTER TABLE`

- [ ] **Step 4: Write auth handler**

`api/src/http/auth.rs`:
```rust
//! Auth routes: register and login for human users.

use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    domain::{error::ApiError, user::Role},
    http::AppState,
};
use super::me::generate_secret;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBody {
    pub display_name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginBody {
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user_id: Uuid,
    pub api_key: String,
}

pub async fn register(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let role: Role = body.role.parse().map_err(|_| ApiError::Validation(vec![
        crate::domain::error::FieldError { field: "role".into(), message: "unknown role".into() }
    ]))?;
    if matches!(role, Role::Agent) {
        return Err(ApiError::Validation(vec![
            crate::domain::error::FieldError { field: "role".into(), message: "use /agents/register for agent accounts".into() }
        ]));
    }

    let user_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO users (id, display_name, email, role) VALUES ($1, $2, $3, $4)"
    )
    .bind(user_id)
    .bind(&body.display_name)
    .bind(&body.email)
    .bind(role.as_str())
    .execute(&state.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
            ApiError::Validation(vec![crate::domain::error::FieldError {
                field: "email".into(),
                message: "email already registered".into(),
            }])
        } else {
            ApiError::Internal(e.into())
        }
    })?;

    let (token_id, secret, hash) = generate_token_parts();
    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, secret_hash, scopes) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(token_id)
    .bind(user_id)
    .bind("default")
    .bind(&hash)
    .bind(default_human_scopes())
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(AuthResponse { user_id, api_key: format!("{}:{}", token_id, secret) }))
}

pub async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let row = sqlx::query("SELECT id FROM users WHERE email = $1 AND role != 'agent'")
        .bind(&body.email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::NotFound)?;

    let user_id: Uuid = row.get("id");
    let (token_id, secret, hash) = generate_token_parts();
    sqlx::query(
        "INSERT INTO api_tokens (id, user_id, name, secret_hash, scopes) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(token_id)
    .bind(user_id)
    .bind("session")
    .bind(&hash)
    .bind(default_human_scopes())
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(AuthResponse { user_id, api_key: format!("{}:{}", token_id, secret) }))
}

fn generate_token_parts() -> (Uuid, String, String) {
    use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
    let token_id = Uuid::now_v7();
    let secret = generate_secret();
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash = Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .unwrap()
        .to_string();
    (token_id, secret, hash)
}

fn default_human_scopes() -> Vec<String> {
    vec![
        "quiz.read".into(), "quiz.write".into(),
        "attempt.read".into(), "attempt.write".into(),
        "stats.read".into(), "feedback.write".into(),
        "plan.read".into(), "plan.write".into(),
    ]
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
}
```

> Note: `generate_secret` is already in `me.rs` — move it to a shared `http/util.rs` and import from both, or `pub(crate)` it.

- [ ] **Step 5: Wire into mod.rs**

In `api/src/http/mod.rs`, add to the router merge chain:
```rust
.merge(auth::router())
```
And add `mod auth;` to the module list.

- [ ] **Step 6: Run test, confirm pass**

```bash
cd api && AME_RUN_DB_TESTS=1 cargo test --test auth test_register_and_login -- --nocapture
```
Expected: `test test_register_and_login ... ok`

- [ ] **Step 7: make check**

```bash
make check
```
Expected: all green.

- [ ] **Step 8: Commit**

```bash
git add db/migrations/20260521100000_auth_email.sql api/src/http/auth.rs api/src/http/mod.rs api/tests/auth.rs
git commit -m "feat: add human auth endpoints (register + login by email)"
```

---

## Task 2: Backend — `POST /v1/quizzes` (create quiz)

**Files:**
- Modify: `api/src/http/quizzes.rs`
- Test: `api/tests/bank.rs` (add quiz creation test)

- [ ] **Step 1: Write failing test**

Add to `api/tests/bank.rs`:
```rust
#[tokio::test]
async fn test_create_quiz() {
    let Some(db) = test_db().await else { return };
    let (token, _) = seed_user_token(&db, "instructor").await;
    let app = ame_api::http::router(db);
    let client = TestClient::new(app);

    let res = client.post("/v1/quizzes")
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "Test Quiz",
            "objectives": ["Understand X"]
        }))
        .send().await;
    assert_eq!(res.status(), 201);
    let body: serde_json::Value = res.json().await;
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["status"].as_str().unwrap(), "draft");
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd api && AME_RUN_DB_TESTS=1 cargo test --test bank test_create_quiz -- --nocapture 2>&1 | tail -5
```
Expected: FAIL with 405 Method Not Allowed.

- [ ] **Step 3: Implement create_quiz handler**

Add to `api/src/http/quizzes.rs`:
```rust
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuizBody {
    pub title: String,
    #[serde(default)]
    pub objectives: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuizResponse {
    pub id: Uuid,
    pub status: String,
}

async fn create_quiz(
    State(state): State<AppState>,
    auth: RequireAnyScope<QuizWriteScopes>,
    Json(body): Json<CreateQuizBody>,
) -> Result<impl IntoResponse, ApiError> {
    let id = Uuid::now_v7();
    let objectives = serde_json::to_value(&body.objectives).unwrap_or(serde_json::json!([]));
    sqlx::query(
        "INSERT INTO quizzes (id, title, status, objectives, created_by) VALUES ($1, $2, 'draft', $3, $4)"
    )
    .bind(id)
    .bind(&body.title)
    .bind(&objectives)
    .bind(auth.0.user.id)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreateQuizResponse { id, status: "draft".into() }),
    ))
}
```

Add route in the router function:
```rust
.route("/v1/quizzes", post(create_quiz).get(list_quizzes))
```

- [ ] **Step 4: Run test, confirm pass**

```bash
cd api && AME_RUN_DB_TESTS=1 cargo test --test bank test_create_quiz -- --nocapture
```
Expected: `test test_create_quiz ... ok`

- [ ] **Step 5: make check + commit**

```bash
make check
git add api/src/http/quizzes.rs api/tests/bank.rs
git commit -m "feat: add POST /v1/quizzes for quiz creation"
```

---

## Task 3: Backend — `GET /v1/me/stats`

**Files:**
- Modify: `api/src/http/me.rs`
- Test: `api/tests/stats.rs`

- [ ] **Step 1: Write failing test**

Add to `api/tests/stats.rs`:
```rust
#[tokio::test]
async fn test_me_stats() {
    let Some(db) = test_db().await else { return };
    let (token, _) = seed_user_token(&db, "learner").await;
    let app = ame_api::http::router(db);
    let client = TestClient::new(app);

    let res = client.get("/v1/me/stats")
        .header("Authorization", format!("Bearer {}", token))
        .send().await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await;
    // Fields required by progress dashboard
    assert!(body["totalAttempts"].is_number());
    assert!(body["avgScore"].is_number());
    assert!(body["currentStreakDays"].is_number());
    assert!(body["masteredTagCount"].is_number());
}
```

- [ ] **Step 2: Implement handler**

Add to `api/src/http/me.rs`:
```rust
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeStats {
    pub total_attempts: i64,
    pub avg_score: f64,       // 0.0–1.0
    pub current_streak_days: i64,
    pub mastered_tag_count: i64,
    pub hours_spent: f64,
}

pub async fn get_stats(
    State(state): State<AppState>,
    auth: RequireAnyScope<MeScopes>,
) -> Result<Json<MeStats>, ApiError> {
    let user_id = auth.0.user.id;

    let row = sqlx::query(
        "SELECT
            COUNT(*)::bigint                                         AS total_attempts,
            COALESCE(AVG(score::float / NULLIF(max_score,0)), 0)    AS avg_score,
            COALESCE(SUM(time_to_answer_ms)::float / 3600000, 0)    AS hours_spent
         FROM attempts
         WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let total_attempts: i64 = row.try_get("total_attempts").unwrap_or(0);
    let avg_score: f64 = row.try_get("avg_score").unwrap_or(0.0);
    let hours_spent: f64 = row.try_get("hours_spent").unwrap_or(0.0);

    // Streak: count consecutive days with at least one attempt, going back from today
    let streak_row = sqlx::query(
        "WITH daily AS (
            SELECT DISTINCT DATE(created_at AT TIME ZONE 'UTC') AS d
            FROM attempts WHERE user_id = $1
         ),
         numbered AS (
            SELECT d, ROW_NUMBER() OVER (ORDER BY d DESC) AS rn FROM daily
         )
         SELECT COUNT(*)::bigint AS streak
         FROM numbered
         WHERE d = CURRENT_DATE - (rn - 1) * INTERVAL '1 day'"
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let current_streak_days: i64 = streak_row.try_get("streak").unwrap_or(0);

    // Mastered tags: tags where correct_rate >= 0.8 across >= 5 attempts
    let mastered_row = sqlx::query(
        "SELECT COUNT(DISTINCT tag)::bigint AS mastered
         FROM (
             SELECT t.name AS tag,
                    AVG(CASE WHEN a.is_correct THEN 1.0 ELSE 0.0 END) AS rate,
                    COUNT(*) AS cnt
             FROM attempts a
             JOIN question_tag_map qtm ON qtm.question_id = a.question_id
             JOIN tags t ON t.id = qtm.tag_id
             WHERE a.user_id = $1
             GROUP BY t.name
             HAVING COUNT(*) >= 5 AND AVG(CASE WHEN a.is_correct THEN 1.0 ELSE 0.0 END) >= 0.8
         ) sub"
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let mastered_tag_count: i64 = mastered_row.try_get("mastered").unwrap_or(0);

    Ok(Json(MeStats { total_attempts, avg_score, current_streak_days, mastered_tag_count, hours_spent }))
}
```

Add route:
```rust
.route("/v1/me/stats", get(get_stats))
```

- [ ] **Step 3: Run test + make check + commit**

```bash
cd api && AME_RUN_DB_TESTS=1 cargo test --test stats test_me_stats -- --nocapture
make check
git add api/src/http/me.rs api/tests/stats.rs
git commit -m "feat: add GET /v1/me/stats for progress dashboard"
```

---

## Task 4: CSS tokens + Google Fonts

**Files:**
- Modify: `web/app/globals.css`
- Modify: `web/app/layout.tsx`

- [ ] **Step 1: Update globals.css**

Replace the full `:root` block and add missing vars. Final `globals.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  /* Surfaces */
  --bg:            #0e1116;
  --surface:       #161b22;
  --surface-2:     #1d232c;
  --surface-3:     #232a35;
  /* Borders */
  --border:        #262d38;
  --border-strong: #353e4c;
  /* Text */
  --text:          #e8ecf1;
  --text-2:        #b4bdc9;
  --muted:         #7c8696;
  /* Accent */
  --accent:        #22d3a8;
  --accent-dim:    rgba(34, 211, 168, 0.12);
  --accent-line:   rgba(34, 211, 168, 0.35);
  /* Semantic */
  --amber:         #f5b85a;
  --amber-dim:     rgba(245, 184, 90, 0.14);
  --red:           #e76e6e;
  --red-dim:       rgba(231, 110, 110, 0.14);
  --blue:          #6aa7f5;
  --blue-dim:      rgba(106, 167, 245, 0.14);
  /* Typography */
  --serif: "Source Serif 4", "Iowan Old Style", Georgia, serif;
  --sans:  "Inter", -apple-system, BlinkMacSystemFont, sans-serif;
  --mono:  "JetBrains Mono", "SF Mono", Menlo, monospace;
  /* Radii */
  --radius:    6px;
  --radius-lg: 10px;
}

[data-theme="paper"] {
  --bg:            #f6f3ec;
  --surface:       #fbf9f3;
  --surface-2:     #f1ecdf;
  --surface-3:     #e7e0cc;
  --border:        #d9d1bc;
  --border-strong: #b9ad8e;
  --text:          #1d1a14;
  --text-2:        #3a352a;
  --muted:         #6e6753;
  --accent:        #0f6b53;
  --accent-dim:    rgba(15, 107, 83, 0.12);
  --accent-line:   rgba(15, 107, 83, 0.35);
  --amber:         #a05c12;
  --amber-dim:     rgba(160, 92, 18, 0.14);
  --red:           #a83b3b;
  --red-dim:       rgba(168, 59, 59, 0.14);
  --blue:          #2c5b8f;
  --blue-dim:      rgba(44, 91, 143, 0.14);
}

[data-theme="cobalt"] {
  --bg:            #0b1530;
  --surface:       #11203f;
  --surface-2:     #182a52;
  --surface-3:     #213665;
  --border:        #243868;
  --border-strong: #355088;
  --text:          #eaf0ff;
  --text-2:        #b6c4e6;
  --muted:         #7c8cb4;
  --accent:        #f5c14a;
  --accent-dim:    rgba(245, 193, 74, 0.12);
  --accent-line:   rgba(245, 193, 74, 0.35);
  --amber:         #f08a3c;
  --amber-dim:     rgba(240, 138, 60, 0.14);
  --red:           #ef6e6e;
  --red-dim:       rgba(239, 110, 110, 0.14);
  --blue:          #8db4ff;
  --blue-dim:      rgba(141, 180, 255, 0.14);
}

*, *::before, *::after { box-sizing: border-box; }

html, body {
  padding: 0;
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: var(--sans);
  font-size: 14px;
  -webkit-font-smoothing: antialiased;
}

*:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
```

- [ ] **Step 2: Load fonts in layout.tsx**

```tsx
import { Source_Serif_4, Inter, JetBrains_Mono } from "next/font/google";
import "./globals.css";
import type { Metadata } from "next";

const serif = Source_Serif_4({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  variable: "--font-serif",
  display: "swap",
});

const sans = Inter({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  variable: "--font-sans",
  display: "swap",
});

const mono = JetBrains_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600"],
  variable: "--font-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Harus",
  description: "Assessment platform for learners and agents.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${serif.variable} ${sans.variable} ${mono.variable}`}>
      <body>{children}</body>
    </html>
  );
}
```

Add to globals.css (override CSS vars with loaded font variables):
```css
:root {
  --serif: var(--font-serif), "Iowan Old Style", Georgia, serif;
  --sans:  var(--font-sans), -apple-system, BlinkMacSystemFont, sans-serif;
  --mono:  var(--font-mono), "SF Mono", Menlo, monospace;
}
```

- [ ] **Step 3: Verify**

```bash
cd web && bun run type-check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add web/app/globals.css web/app/layout.tsx
git commit -m "feat: complete CSS design tokens and load Source Serif 4 / Inter / JetBrains Mono"
```

---

## Task 5: Atom component library

**Files:** `web/src/components/ui/` (all new)

Port the design's `atoms.jsx` to typed TypeScript components. Each component is a direct translation — no new props or behavior.

- [ ] **Step 1: Write type-check test (a11y smoke)**

`web/e2e/atoms.spec.ts`:
```typescript
import { test, expect } from "@playwright/test";
// Atoms are tested implicitly through pages; this spec just verifies
// the login page renders with correct font and token vars applied.
test("design tokens applied", async ({ page }) => {
  await page.goto("/login");
  const bg = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue("--bg").trim()
  );
  expect(bg).toBe("#0e1116");
  const fontFamily = await page.evaluate(() =>
    getComputedStyle(document.body).fontFamily
  );
  expect(fontFamily).toContain("Inter");
});
```

- [ ] **Step 2: Implement Button**

`web/src/components/ui/Button.tsx`:
```tsx
import React from "react";

type Variant = "primary" | "ghost" | "solid" | "danger" | "quiet";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  icon?: React.ReactNode;
}

const variantStyles: Record<Variant, React.CSSProperties> = {
  primary: { background: "var(--accent)", color: "#0b1410", borderColor: "var(--accent)" },
  ghost:   { background: "transparent", color: "var(--text-2)", borderColor: "var(--border)" },
  solid:   { background: "var(--surface-2)", color: "var(--text)", borderColor: "var(--border)" },
  danger:  { background: "transparent", color: "var(--red)", borderColor: "var(--border)" },
  quiet:   { background: "transparent", color: "var(--muted)", borderColor: "transparent" },
};

const sizeStyles: Record<Size, React.CSSProperties> = {
  sm: { padding: "6px 10px", fontSize: 12 },
  md: { padding: "8px 14px", fontSize: 13 },
  lg: { padding: "11px 18px", fontSize: 13 },
};

export function Button({ variant = "primary", size = "md", icon, children, disabled, style, ...props }: ButtonProps) {
  return (
    <button
      disabled={disabled}
      style={{
        fontFamily: "var(--sans)",
        fontWeight: 500,
        letterSpacing: 0.1,
        borderRadius: 6,
        border: "1px solid transparent",
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.55 : 1,
        transition: "background 120ms, border-color 120ms, color 120ms",
        ...variantStyles[variant],
        ...sizeStyles[size],
        ...style,
      }}
      {...props}
    >
      {icon && <span style={{ display: "inline-flex" }}>{icon}</span>}
      {children}
    </button>
  );
}
```

- [ ] **Step 3: Implement Tag**

`web/src/components/ui/Tag.tsx`:
```tsx
import React from "react";

type Tone = "default" | "accent" | "amber" | "red" | "blue" | "ghost";

interface TagProps { children: React.ReactNode; tone?: Tone; style?: React.CSSProperties; }

const tones: Record<Tone, { bg: string; fg: string; border: string }> = {
  default: { bg: "var(--surface-2)", fg: "var(--text-2)", border: "var(--border)" },
  accent:  { bg: "var(--accent-dim)", fg: "var(--accent)", border: "var(--accent-line)" },
  amber:   { bg: "var(--amber-dim)", fg: "var(--amber)", border: "var(--amber)" },
  red:     { bg: "var(--red-dim)", fg: "var(--red)", border: "var(--red)" },
  blue:    { bg: "var(--blue-dim)", fg: "var(--blue)", border: "var(--blue)" },
  ghost:   { bg: "transparent", fg: "var(--muted)", border: "var(--border)" },
};

export function Tag({ children, tone = "default", style }: TagProps) {
  const t = tones[tone];
  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: 4,
      padding: "2px 7px", fontSize: 11, fontWeight: 500,
      letterSpacing: 0.3, textTransform: "uppercase", borderRadius: 4,
      background: t.bg, color: t.fg, border: `1px solid ${t.border}`,
      ...style,
    }}>{children}</span>
  );
}
```

- [ ] **Step 4: Implement Card, KV, Stat, Divider**

`web/src/components/ui/Card.tsx`:
```tsx
import React, { useState } from "react";

interface CardProps { children: React.ReactNode; padding?: number; hoverable?: boolean; style?: React.CSSProperties; }

export function Card({ children, padding = 20, hoverable, style }: CardProps) {
  const [hover, setHover] = useState(false);
  return (
    <div
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        background: "var(--surface)",
        border: `1px solid ${hoverable && hover ? "var(--border-strong)" : "var(--border)"}`,
        borderRadius: "var(--radius-lg)",
        padding,
        transition: "border-color 140ms",
        ...style,
      }}
    >{children}</div>
  );
}
```

`web/src/components/ui/KV.tsx`:
```tsx
import React from "react";

export function KV({ k, v, mono }: { k: string; v: React.ReactNode; mono?: boolean }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", padding: "8px 0", borderBottom: "1px dashed var(--border)" }}>
      <span style={{ color: "var(--muted)", fontSize: 12 }}>{k}</span>
      <span style={{ color: "var(--text)", fontSize: 13, fontFamily: mono ? "var(--mono)" : "inherit", fontWeight: 500 }}>{v}</span>
    </div>
  );
}
```

`web/src/components/ui/Stat.tsx`:
```tsx
import React from "react";

export function Stat({ label, value, sub, tone }: { label: string; value: React.ReactNode; sub?: string; tone?: "up" | "down" }) {
  const subColor = tone === "up" ? "var(--accent)" : tone === "down" ? "var(--red)" : "var(--muted)";
  return (
    <div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{label}</div>
      <div style={{ fontFamily: "var(--serif)", fontSize: 30, fontWeight: 500, lineHeight: 1, color: "var(--text)", letterSpacing: -0.5 }}>{value}</div>
      {sub && <div style={{ marginTop: 6, fontSize: 12, color: subColor }}>{sub}</div>}
    </div>
  );
}
```

`web/src/components/ui/Divider.tsx`:
```tsx
import React from "react";

export function Divider({ vertical, style }: { vertical?: boolean; style?: React.CSSProperties }) {
  if (vertical) return <div style={{ width: 1, background: "var(--border)", alignSelf: "stretch", ...style }} />;
  return <div style={{ height: 1, background: "var(--border)", width: "100%", ...style }} />;
}
```

- [ ] **Step 5: Implement Icon**

`web/src/components/ui/Icon.tsx` — port the full Icon map from `design/source/src/atoms.jsx`:
```tsx
import React from "react";

export type IconName =
  | "library" | "take" | "results" | "dashboard" | "author" | "agent"
  | "exam" | "stack" | "settings" | "arrow" | "arrowL" | "check" | "x"
  | "plus" | "clock" | "search" | "filter" | "bell" | "key" | "copy"
  | "download" | "upload" | "flag" | "user" | "sparkle" | "book" | "code";

interface IconProps { name: IconName; size?: number; stroke?: number; color?: string; }

export function Icon({ name, size = 16, stroke = 1.6, color = "currentColor" }: IconProps) {
  const p = { width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: color, strokeWidth: stroke, strokeLinecap: "round" as const, strokeLinejoin: "round" as const };
  const icons: Record<IconName, React.ReactElement> = {
    library:   <svg {...p}><path d="M4 4v16M9 4v16M14 6l2 14M19 5l2 14"/></svg>,
    take:      <svg {...p}><path d="M4 6h16M4 12h10M4 18h7"/><circle cx="18" cy="16" r="3"/></svg>,
    results:   <svg {...p}><path d="M4 19V5M4 19h16M8 15v-4M12 15V7M16 15v-2"/></svg>,
    dashboard: <svg {...p}><rect x="3" y="3" width="7" height="9"/><rect x="14" y="3" width="7" height="5"/><rect x="14" y="12" width="7" height="9"/><rect x="3" y="16" width="7" height="5"/></svg>,
    author:    <svg {...p}><path d="M4 20h4l10-10-4-4L4 16v4z"/><path d="M14 6l4 4"/></svg>,
    agent:     <svg {...p}><rect x="3" y="6" width="18" height="14" rx="2"/><path d="M8 6V3M16 6V3M3 12h18"/><circle cx="9" cy="16" r="1"/><circle cx="15" cy="16" r="1"/></svg>,
    exam:      <svg {...p}><rect x="5" y="3" width="14" height="18" rx="1"/><path d="M9 8h6M9 12h6M9 16h4"/></svg>,
    stack:     <svg {...p}><path d="M12 3l9 5-9 5-9-5 9-5z"/><path d="M3 13l9 5 9-5M3 18l9 5 9-5"/></svg>,
    settings:  <svg {...p}><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4.9a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.5a7 7 0 0 0-2 1.2l-2.4-.9-2 3.4 2 1.6A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.6 2 3.4 2.4-.9c.6.5 1.3.9 2 1.2L10 21h4l.5-2.5c.7-.3 1.4-.7 2-1.2l2.4.9 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z"/></svg>,
    arrow:     <svg {...p}><path d="M5 12h14M13 6l6 6-6 6"/></svg>,
    arrowL:    <svg {...p}><path d="M19 12H5M11 6l-6 6 6 6"/></svg>,
    check:     <svg {...p}><path d="M5 12l4 4 10-10"/></svg>,
    x:         <svg {...p}><path d="M6 6l12 12M18 6L6 18"/></svg>,
    plus:      <svg {...p}><path d="M12 5v14M5 12h14"/></svg>,
    clock:     <svg {...p}><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>,
    search:    <svg {...p}><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></svg>,
    filter:    <svg {...p}><path d="M3 5h18M6 12h12M10 19h4"/></svg>,
    bell:      <svg {...p}><path d="M6 17V11a6 6 0 1 1 12 0v6l2 2H4l2-2zM10 21a2 2 0 0 0 4 0"/></svg>,
    key:       <svg {...p}><circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3"/></svg>,
    copy:      <svg {...p}><rect x="9" y="9" width="11" height="11" rx="1"/><path d="M5 15V5a1 1 0 0 1 1-1h10"/></svg>,
    download:  <svg {...p}><path d="M12 4v12M6 12l6 6 6-6M4 20h16"/></svg>,
    upload:    <svg {...p}><path d="M12 20V8M6 12l6-6 6 6M4 4h16"/></svg>,
    flag:      <svg {...p}><path d="M5 21V4l14 4-14 4"/></svg>,
    user:      <svg {...p}><circle cx="12" cy="8" r="4"/><path d="M4 21c1.5-4 4.5-6 8-6s6.5 2 8 6"/></svg>,
    sparkle:   <svg {...p}><path d="M12 4v4M12 16v4M4 12h4M16 12h4M6 6l3 3M15 15l3 3M18 6l-3 3M9 15l-3 3"/></svg>,
    book:      <svg {...p}><path d="M4 5a2 2 0 0 1 2-2h12v18H6a2 2 0 0 1-2-2V5z"/><path d="M4 19a2 2 0 0 1 2-2h12"/></svg>,
    code:      <svg {...p}><path d="M9 18l-6-6 6-6M15 6l6 6-6 6"/></svg>,
  };
  return icons[name] ?? null;
}
```

- [ ] **Step 6: Implement Logo**

`web/src/components/ui/Logo.tsx`:
```tsx
import React from "react";

export function Logo({ size = 22 }: { size?: number }) {
  return (
    <div style={{ display: "inline-flex", alignItems: "center", gap: 9 }}>
      <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
        <rect x="2" y="2" width="28" height="28" rx="6" fill="var(--accent)" />
        <path d="M10 9v14M22 9v14M10 16h12" stroke="#0b1410" strokeWidth="2.5" strokeLinecap="round" />
      </svg>
      <span style={{ fontFamily: "var(--serif)", fontSize: 19, fontWeight: 600, letterSpacing: -0.3, color: "var(--text)" }}>
        Harus
      </span>
    </div>
  );
}
```

- [ ] **Step 7: Barrel export**

`web/src/components/ui/index.ts`:
```ts
export { Button } from "./Button";
export { Tag } from "./Tag";
export { Card } from "./Card";
export { Icon } from "./Icon";
export type { IconName } from "./Icon";
export { Logo } from "./Logo";
export { KV } from "./KV";
export { Stat } from "./Stat";
export { Divider } from "./Divider";
```

- [ ] **Step 8: type-check + commit**

```bash
cd web && bun run type-check
git add web/src/components/ui/
git commit -m "feat: add atom component library (Button, Tag, Card, Icon, Logo, KV, Stat, Divider)"
```

---

## Task 6: Sidebar + Topbar components

**Files:**
- Create: `web/src/components/Sidebar.tsx`
- Create: `web/src/components/Topbar.tsx`
- Modify: `web/app/(learner)/layout.tsx`

- [ ] **Step 1: Write Playwright test**

`web/e2e/shell.spec.ts`:
```typescript
import { test, expect } from "@playwright/test";
// Assumes a seeded test user with token in cookie. In CI, seed via API first.
test("sidebar shows logo and nav icons", async ({ page }) => {
  // Set cookie for a pre-seeded token
  await page.context().addCookies([{ name: "ame_token", value: process.env.TEST_TOKEN ?? "", domain: "localhost", path: "/" }]);
  await page.goto("/library");
  await expect(page.getByText("Harus")).toBeVisible();
  await expect(page.getByText("Library")).toBeVisible();
  await expect(page.getByText("Progress")).toBeVisible();
});
```

- [ ] **Step 2: Implement Sidebar**

`web/src/components/Sidebar.tsx`:
```tsx
"use client";
import React from "react";
import { usePathname } from "next/navigation";
import { Logo } from "./ui/Logo";
import { Icon, IconName } from "./ui/Icon";

interface NavItem { href: string; label: string; icon: IconName; section: string; roles?: string[]; }

const NAV: NavItem[] = [
  { href: "/library",  label: "Library",       icon: "library",   section: "Learn" },
  { href: "/exams",    label: "Exams",          icon: "stack",     section: "Learn" },
  { href: "/practice", label: "Take quiz",      icon: "take",      section: "Learn" },
  { href: "/progress", label: "Progress",       icon: "dashboard", section: "Learn" },
  { href: "/author",   label: "Author studio",  icon: "author",    section: "Teach",      roles: ["instructor","admin"] },
  { href: "/agent",    label: "Agent API",      icon: "agent",     section: "Integrate",  roles: ["instructor","admin"] },
];

interface SidebarProps { role: string; displayName: string; onSignOut: () => void; }

export function Sidebar({ role, displayName, onSignOut }: SidebarProps) {
  const pathname = usePathname();
  const initials = displayName.split(" ").map(w => w[0]).join("").slice(0,2).toUpperCase();

  const canSee = (item: NavItem) => !item.roles || item.roles.includes(role);
  const isActive = (href: string) => href === "/library" ? pathname === "/library" : pathname.startsWith(href);

  const sections = ["Learn", "Teach", "Integrate"];

  return (
    <aside style={{ width: 232, flexShrink: 0, borderRight: "1px solid var(--border)", background: "var(--surface)", display: "flex", flexDirection: "column", height: "100vh", position: "sticky", top: 0 }}>
      <div style={{ padding: "20px 20px 16px", borderBottom: "1px solid var(--border)" }}>
        <Logo />
        <div style={{ marginTop: 4, fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: 1.2, textTransform: "uppercase" }}>
          Assessment Platform
        </div>
      </div>

      <nav style={{ flex: 1, overflowY: "auto", padding: "16px 12px" }}>
        {sections.map(sec => {
          const items = NAV.filter(n => n.section === sec && canSee(n));
          if (!items.length) return null;
          return (
            <div key={sec} style={{ marginBottom: 18 }}>
              <div style={{ padding: "6px 10px 8px", fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.4, color: "var(--muted)", textTransform: "uppercase" }}>{sec}</div>
              {items.map(item => {
                const active = isActive(item.href);
                return (
                  <a key={item.href} href={item.href} style={{
                    display: "flex", alignItems: "center", gap: 10,
                    padding: "9px 10px", borderRadius: 6, marginBottom: 2,
                    background: active ? "var(--accent-dim)" : "transparent",
                    border: `1px solid ${active ? "var(--accent-line)" : "transparent"}`,
                    color: active ? "var(--accent)" : "var(--text-2)",
                    fontSize: 13, fontWeight: active ? 600 : 500,
                    textDecoration: "none",
                    transition: "background 120ms, color 120ms",
                  }}>
                    <Icon name={item.icon} size={16} />
                    <span>{item.label}</span>
                  </a>
                );
              })}
            </div>
          );
        })}
      </nav>

      <div style={{ borderTop: "1px solid var(--border)", padding: 14 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <div style={{ width: 32, height: 32, borderRadius: "50%", background: "var(--surface-3)", border: "1px solid var(--border)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--serif)", fontSize: 13, color: "var(--text)", flexShrink: 0 }}>
            {initials}
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 12.5, fontWeight: 500, color: "var(--text)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{displayName}</div>
            <div style={{ fontSize: 11, color: "var(--muted)", textTransform: "capitalize" }}>{role}</div>
          </div>
          <button onClick={onSignOut} style={{ background: "transparent", border: "none", cursor: "pointer", color: "var(--muted)", padding: 4 }} title="Sign out">
            <Icon name="settings" size={14} />
          </button>
        </div>
      </div>
    </aside>
  );
}
```

- [ ] **Step 3: Implement Topbar**

`web/src/components/Topbar.tsx`:
```tsx
import React from "react";
import { Icon } from "./ui/Icon";

interface TopbarProps {
  title: string;
  subtitle?: string;
  breadcrumb?: string;
  actions?: React.ReactNode;
}

export function Topbar({ title, subtitle, breadcrumb, actions }: TopbarProps) {
  return (
    <header style={{ padding: "20px 36px", borderBottom: "1px solid var(--border)", background: "var(--bg)", display: "flex", alignItems: "center", justifyContent: "space-between", position: "sticky", top: 0, zIndex: 5 }}>
      <div>
        {breadcrumb && <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{breadcrumb}</div>}
        <h1 style={{ margin: 0, fontFamily: "var(--serif)", fontWeight: 500, fontSize: 26, letterSpacing: -0.3, color: "var(--text)" }}>{title}</h1>
        {subtitle && <div style={{ marginTop: 4, color: "var(--muted)", fontSize: 13 }}>{subtitle}</div>}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        {actions}
        <div style={{ display: "flex", alignItems: "center", gap: 10, paddingLeft: 14, borderLeft: "1px solid var(--border)" }}>
          <Icon name="bell" size={16} color="var(--muted)" />
          <Icon name="search" size={16} color="var(--muted)" />
        </div>
      </div>
    </header>
  );
}
```

- [ ] **Step 4: Wire Sidebar into learner layout**

Replace the hand-rolled `<nav>` in `web/app/(learner)/layout.tsx` with:
```tsx
import { Sidebar } from "@/components/Sidebar";
// …inside the return, replace the <nav> aside with:
<Sidebar
  role={user.role}
  displayName={user.displayName}
  onSignOut={() => {
    document.cookie = "ame_token=; path=/; max-age=0";
    router.replace("/login");
  }}
/>
```

The outer `<div>` keeps `display: grid; gridTemplateColumns: 232px 1fr`.

- [ ] **Step 5: type-check + commit**

```bash
cd web && bun run type-check
git add web/src/components/Sidebar.tsx web/src/components/Topbar.tsx web/app/"(learner)"/layout.tsx
git commit -m "feat: Sidebar with icons + Topbar component matching design"
```

---

## Task 7: Signup/Login screen rebuild

**Files:** `web/app/login/page.tsx`

- [ ] **Step 1: Write Playwright test**

`web/e2e/auth.spec.ts`:
```typescript
import { test, expect } from "@playwright/test";

test("login page has two-column layout and role picker", async ({ page }) => {
  await page.goto("/login");
  // Marketing column
  await expect(page.getByText("Quizzes that learners and agents can both read.")).toBeVisible();
  // Form column — tabs
  await expect(page.getByRole("button", { name: "Create account" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign in" })).toBeVisible();
  // Role picker visible on signup tab
  await expect(page.getByRole("button", { name: "learner" })).toBeVisible();
});

test("register creates account and redirects to library", async ({ page }) => {
  await page.goto("/login");
  await page.fill('[placeholder="Full name"]', "Test User");
  await page.fill('[placeholder="you@institution.edu"]', `test-${Date.now()}@example.com`);
  await page.getByRole("button", { name: "learner" }).click();
  await page.getByRole("button", { name: "Create account" }).last().click();
  await expect(page).toHaveURL(/\/library/, { timeout: 5000 });
});
```

- [ ] **Step 2: Implement the screen**

Full rebuild of `web/app/login/page.tsx` matching `design/source/src/screen-signup.jsx`. Key structure:

```tsx
"use client";
import { useState, FormEvent } from "react";
import { useRouter } from "next/navigation";
import { setAuthToken } from "@/hooks/useAuth";
import { Button } from "@/components/ui/Button";
import { Logo } from "@/components/ui/Logo";

type Tab = "signup" | "login";
type Role = "learner" | "instructor" | "agent";

export default function LoginPage() {
  const router = useRouter();
  const [tab, setTab] = useState<Tab>("signup");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<Role>("learner");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const endpoint = tab === "signup" ? "/v1/auth/register" : "/v1/auth/login";
      const body = tab === "signup"
        ? { displayName: name, email, role }
        : { email };
      const base = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
      const res = await fetch(`${base}${endpoint}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        setError(err?.errors?.[0]?.message ?? (res.status === 404 ? "No account found for that email." : "Something went wrong."));
        return;
      }
      const data = await res.json();
      setAuthToken(data.apiKey);
      router.push("/library");
    } catch {
      setError("Could not reach the API. Is the server running?");
    } finally {
      setLoading(false);
    }
  }

  const roleDescriptions: Record<Role, string> = {
    learner: "Take assigned quizzes and track your progress.",
    instructor: "Author quizzes, manage cohorts, and review attempts.",
    agent: "Get an API key, an OpenAPI schema, and MCP tool descriptors.",
  };

  return (
    <div style={{ minHeight: "100vh", display: "grid", gridTemplateColumns: "1.05fr 1fr", background: "var(--bg)" }}>
      {/* LEFT — marketing */}
      <div style={{ padding: "56px 64px", borderRight: "1px solid var(--border)", background: "linear-gradient(180deg, var(--surface) 0%, var(--bg) 70%)", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
        <Logo size={28} />
        <div style={{ maxWidth: 520 }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 1.6, textTransform: "uppercase", color: "var(--accent)", marginBottom: 18 }}>
            Assessment platform · est. 2025
          </div>
          <h1 style={{ fontFamily: "var(--serif)", fontSize: 52, lineHeight: 1.06, margin: 0, fontWeight: 500, letterSpacing: -1.2, color: "var(--text)" }}>
            Quizzes that learners and agents can both read.
          </h1>
          <p style={{ color: "var(--text-2)", fontSize: 15, lineHeight: 1.6, marginTop: 22, maxWidth: 460 }}>
            Harus is an assessment platform built for two audiences at once. Students get a focused test-taking experience and a real progress dashboard. Authors and AI agents share the same structured surface.
          </p>
          <div style={{ marginTop: 36, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, maxWidth: 460 }}>
            {[["Quiz library", "questions bank"], ["Elo scoring", "adaptive engine"], ["OpenAPI 3.1", "agent surface"], ["MCP tools", "26 descriptors"]].map(([v, l]) => (
              <div key={l} style={{ padding: "12px 14px", border: "1px solid var(--border)", borderRadius: 6 }}>
                <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, color: "var(--text)" }}>{v}</div>
                <div style={{ fontSize: 11, color: "var(--muted)", fontFamily: "var(--mono)", letterSpacing: 1, textTransform: "uppercase", marginTop: 2 }}>{l}</div>
              </div>
            ))}
          </div>
        </div>
        <div style={{ display: "flex", gap: 18, color: "var(--muted)", fontSize: 12, fontFamily: "var(--mono)", letterSpacing: 0.6 }}>
          <span>OpenAPI 3.1</span><span>·</span><span>MCP</span><span>·</span><span>Elo engine</span>
        </div>
      </div>

      {/* RIGHT — form */}
      <div style={{ padding: "56px 64px", display: "flex", alignItems: "center" }}>
        <div style={{ width: "100%", maxWidth: 420 }}>
          {/* Tab switcher */}
          <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border)", marginBottom: 28 }}>
            {(["signup", "login"] as Tab[]).map(t => (
              <button key={t} onClick={() => setTab(t)} style={{ background: "transparent", border: "none", padding: "10px 0", marginRight: 24, color: tab === t ? "var(--text)" : "var(--muted)", borderBottom: `2px solid ${tab === t ? "var(--accent)" : "transparent"}`, fontWeight: tab === t ? 600 : 500, fontSize: 13, cursor: "pointer" }}>
                {t === "signup" ? "Create account" : "Sign in"}
              </button>
            ))}
          </div>

          <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {tab === "signup" && (
              <Field label="Full name">
                <input value={name} onChange={e => setName(e.target.value)} placeholder="Full name" required style={inputStyle} />
              </Field>
            )}
            <Field label="Institutional email">
              <input type="email" value={email} onChange={e => setEmail(e.target.value)} placeholder="you@institution.edu" required style={inputStyle} />
            </Field>
            {tab === "signup" && (
              <Field label="Role">
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
                  {(["learner","instructor","agent"] as Role[]).map(r => (
                    <button key={r} type="button" onClick={() => setRole(r)} style={{ padding: "10px 8px", background: role === r ? "var(--accent-dim)" : "var(--surface)", border: `1px solid ${role === r ? "var(--accent-line)" : "var(--border)"}`, color: role === r ? "var(--accent)" : "var(--text-2)", borderRadius: 6, fontSize: 12, fontWeight: 500, textTransform: "capitalize", cursor: "pointer" }}>
                      {r}
                    </button>
                  ))}
                </div>
                <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 8, lineHeight: 1.5 }}>{roleDescriptions[role]}</div>
              </Field>
            )}
            {error && <div style={{ padding: "8px 12px", background: "var(--red-dim)", border: "1px solid var(--red)", borderRadius: 4, color: "var(--red)", fontSize: 12 }}>{error}</div>}
            <div style={{ marginTop: 4 }}>
              <Button type="submit" disabled={loading || !email.trim()} style={{ width: "100%", justifyContent: "center" }}>
                {loading ? "Please wait…" : tab === "signup" ? "Create account" : "Sign in"}
              </Button>
            </div>
          </form>

          {/* Agent shortcut callout */}
          <div style={{ marginTop: 32, padding: 14, border: "1px dashed var(--border-strong)", borderRadius: 6, background: "var(--surface)" }}>
            <div style={{ fontSize: 12, fontWeight: 600, color: "var(--accent)", marginBottom: 4 }}>Agent shortcut</div>
            <div style={{ fontSize: 12, color: "var(--text-2)", lineHeight: 1.5 }}>
              Programmatic access? <code style={{ fontFamily: "var(--mono)", color: "var(--text)" }}>POST /v1/agents/register</code> returns a key and MCP manifest in one call.
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ display: "block" }}>
      <div style={{ fontSize: 11, fontFamily: "var(--mono)", letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{label}</div>
      {children}
    </label>
  );
}

const inputStyle: React.CSSProperties = {
  width: "100%", background: "var(--surface)", border: "1px solid var(--border)",
  borderRadius: 6, padding: "10px 12px", fontSize: 14, color: "var(--text)",
  fontFamily: "var(--sans)", outline: "none", boxSizing: "border-box",
};
```

- [ ] **Step 3: Run Playwright test**

```bash
cd web && bunx playwright test e2e/auth.spec.ts --reporter=line
```
Expected: 2 passed.

- [ ] **Step 4: type-check + commit**

```bash
bun run type-check
git add web/app/login/page.tsx web/e2e/auth.spec.ts
git commit -m "feat: rebuild signup/login screen to match two-column design"
```

---

## Task 8: Library screen rebuild

**Files:** `web/app/(learner)/library/page.tsx`

**Reference:** `design/source/src/screen-library.jsx`

- [ ] **Step 1: Playwright test**

`web/e2e/library.spec.ts`:
```typescript
import { test, expect } from "@playwright/test";

test.beforeEach(async ({ context }) => {
  await context.addCookies([{ name: "ame_token", value: process.env.TEST_TOKEN ?? "", domain: "localhost", path: "/" }]);
});

test("library has tabs and hero section", async ({ page }) => {
  await page.goto("/library");
  await expect(page.getByText("Library")).toBeVisible();
  // Tabs
  await expect(page.getByRole("button", { name: /All quizzes/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Assigned/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Completed/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Drafts/ })).toBeVisible();
});
```

- [ ] **Step 2: Rebuild page**

Rewrite `web/app/(learner)/library/page.tsx` with these structures matching the design:

1. `<Topbar title="Library" breadcrumb="Spring 2026 · Active term" actions={<><Button variant="ghost" icon={<Icon name="filter" />}>Filter</Button><Button variant="solid" icon={<Icon name="plus" />}>New quiz</Button></>} />`

2. Tabs: `["all","assigned","completed","drafts"]` with counts from API response. Tab "all" hits `GET /v1/quizzes?status=active`, "completed" maps to a future filter.

3. Hero card — if there is a quiz, render the first result as the "Up next" card in `1.4fr 1fr` grid (quiz details left, cohort KV right). Left side shows: Tags row, serif `h3` title, description, `<LearningObjectives>`, stats row (Questions/Duration), CTA buttons. Right side shows placeholder KV rows while cohort API is not yet available.

4. Quiz grid: `repeat(auto-fill, minmax(330px, 1fr))` with `<QuizCard>` components featuring 4px left accent stripe (`var(--accent)` / `var(--blue)` / `var(--amber)` cycling), serif title, tags footer, "Open →" link.

- [ ] **Step 3: type-check + commit**

```bash
cd web && bun run type-check
git add "web/app/(learner)/library/page.tsx" web/e2e/library.spec.ts
git commit -m "feat: rebuild library screen — hero card + accent-stripe quiz grid"
```

---

## Task 9: Quiz setup + active quiz screens

**Files:**
- Modify: `web/app/(learner)/practice/page.tsx` — full setup form
- Modify: `web/app/(learner)/sessions/[id]/page.tsx` — active quiz

**Reference:** `design/source/src/screen-quiz.jsx` (QuizSetup + active quiz)

- [ ] **Step 1: Playwright test**

`web/e2e/quiz.spec.ts`:
```typescript
import { test, expect } from "@playwright/test";

test.beforeEach(async ({ context }) => {
  await context.addCookies([{ name: "ame_token", value: process.env.TEST_TOKEN ?? "", domain: "localhost", path: "/" }]);
});

test("practice setup shows mode cards and Start is disabled with no questions", async ({ page }) => {
  await page.goto("/practice");
  await expect(page.getByText("Practice")).toBeVisible();
  await expect(page.getByText("Untimed")).toBeVisible();
  await expect(page.getByText("Timed")).toBeVisible();
  await expect(page.getByRole("button", { name: "Start session" })).toBeDisabled();
});

test("active quiz shows progress bar and question palette", async ({ page }) => {
  // This test requires a live session ID — skip in unit CI, run manually
  test.skip(!process.env.TEST_SESSION_ID, "needs TEST_SESSION_ID");
  await page.goto(`/sessions/${process.env.TEST_SESSION_ID}`);
  await expect(page.locator("[data-testid='progress-bar']")).toBeVisible();
  await expect(page.locator("[data-testid='question-palette']")).toBeVisible();
});
```

- [ ] **Step 2: Rebuild practice/setup page**

`web/app/(learner)/practice/page.tsx` key structure:

```
<div style={{ display: "grid", gridTemplateColumns: "1fr 340px", gap: 24, padding: "28px 36px" }}>
  <div> {/* Form */}
    <Topbar title="Take quiz" breadcrumb="Practice" />
    {/* Block 01 — Session mode */}
    <ModeSelector value={mode} onChange={setMode} />
    {/* Block 02 — Tags */}
    <TagPicker tags={availableTags} selected={selectedTags} onChange={setSelectedTags} />
    {/* Block 03 — Difficulty */}
    <DifficultySelector value={diff} onChange={setDiff} />
    {/* Block 04 — Question types */}
    <TypeSelector value={types} onChange={setTypes} />
    {/* Block 05 — Length */}
    <LengthInput count={count} duration={duration} mode={mode} onCountChange={setCount} onDurationChange={setDuration} />
  </div>
  <div> {/* Right rail summary */}
    <Card>
      <div>Session summary</div>
      <KV k="Mode" v={mode} />
      <KV k="Questions" v={count} mono />
      {mode !== "practice" && <KV k="Time limit" v={`${duration} min`} mono />}
      <KV k="Question types" v={types.length ? types.join(", ") : "Any"} />
      <Button disabled={count === 0 || types.length === 0} onClick={startSession} style={{ width: "100%", marginTop: 16, justifyContent: "center" }}>
        Start session
      </Button>
    </Card>
  </div>
</div>
```

Session start calls `POST /v1/sessions` with `{ mode, tags: selectedTags, types, diff, count, duration }` and navigates to `/sessions/{sessionId}`.

- [ ] **Step 3: Rebuild active quiz page**

`web/app/(learner)/sessions/[id]/page.tsx` key additions:

1. Sticky header: attempt kicker + serif title + countdown timer (red when `< 300s`) + Save & exit
2. `data-testid="progress-bar"` — 3px `var(--accent)` bar at `(answered/total)*100%`
3. Question: mono kicker + flag button + 26px serif prompt
4. Input renderers: MC (letter badges A/B/C/D), TF (two big serif squares), Short (mono input), Essay (serif textarea + word count)
5. `data-testid="question-palette"` right rail: 5-column grid of numbered buttons, answered=`accent-dim`, current=accent border, flagged=amber dot

- [ ] **Step 4: type-check + commit**

```bash
cd web && bun run type-check
git add "web/app/(learner)/practice/page.tsx" "web/app/(learner)/sessions/[id]/page.tsx" web/e2e/quiz.spec.ts
git commit -m "feat: quiz setup screen + active quiz with progress bar and palette"
```

---

## Task 10: Results screen rebuild

**Files:** `web/app/(learner)/sessions/[id]/results/page.tsx`

**Reference:** `design/source/src/screen-results.jsx`

- [ ] **Step 1: Playwright test**

`web/e2e/results.spec.ts`:
```typescript
import { test, expect } from "@playwright/test";

test("results page shows score, per-item rows with share links", async ({ page }) => {
  test.skip(!process.env.TEST_SESSION_ID, "needs TEST_SESSION_ID");
  await page.context().addCookies([{ name: "ame_token", value: process.env.TEST_TOKEN ?? "", domain: "localhost", path: "/" }]);
  await page.goto(`/sessions/${process.env.TEST_SESSION_ID}/results`);
  await expect(page.getByText("Results")).toBeVisible();
  await expect(page.locator("[data-testid='score-donut']")).toBeVisible();
  await expect(page.locator("[data-testid='per-item-review']")).toBeVisible();
});
```

- [ ] **Step 2: Rebuild results page**

Key layout (matching `ResultsScreen` in design):

```tsx
// Top row
<div style={{ display: "grid", gridTemplateColumns: "1.3fr 1fr", gap: 18, marginBottom: 24 }}>
  <Card padding={28}>
    <div data-testid="score-donut" style={{ display: "flex", gap: 28, alignItems: "center" }}>
      <DonutSVG percent={scorePercent} size={130} />
      <div>
        <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
          <Tag tone={passed ? "accent" : "red"}>{passed ? "Pass" : "Fail"}</Tag>
          <Tag>{scorePercent.toFixed(1)}%</Tag>
        </div>
        <div style={{ fontFamily: "var(--serif)", fontSize: 38, fontWeight: 500, letterSpacing: -0.6 }}>
          {score} <span style={{ color: "var(--muted)", fontSize: 22 }}>/ {maxScore} pts</span>
        </div>
      </div>
    </div>
    {/* 4-stat strip */}
    <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 20, marginTop: 20, paddingTop: 18, borderTop: "1px solid var(--border)" }}>
      <Stat label="Duration" value={duration} />
      <Stat label="Cohort avg" value="—" />
      <Stat label="Percentile" value="—" />
      <Stat label="Topic mastery" value="—" />
    </div>
  </Card>
  <Card padding={24}>{/* Cohort placeholder */}</Card>
</div>

// Per-item review
<Card data-testid="per-item-review" padding={0}>
  {answers.map((a, i) => (
    <div key={a.questionId} style={{ display: "grid", gridTemplateColumns: "48px 1fr 110px", gap: 18, padding: "18px 22px", borderBottom: "1px solid var(--border)", alignItems: "start" }}>
      <div style={{ width: 40, height: 40, background: a.isCorrect ? "var(--accent-dim)" : "var(--red-dim)", border: `1px solid ${a.isCorrect ? "var(--accent-line)" : "var(--red)"}`, borderRadius: 6, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <Icon name={a.isCorrect ? "check" : "x"} size={16} color={a.isCorrect ? "var(--accent)" : "var(--red)"} />
      </div>
      <div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", textTransform: "uppercase", letterSpacing: 1.1, marginBottom: 6 }}>Q{i+1} · {a.kind}</div>
        <div style={{ fontFamily: "var(--serif)", fontSize: 15, lineHeight: 1.5 }}>{a.prompt}</div>
        <div style={{ marginTop: 8, fontSize: 13, color: "var(--text-2)" }}>
          <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", marginRight: 8, textTransform: "uppercase", letterSpacing: 0.5 }}>Your answer</span>
          {a.given}
        </div>
      </div>
      <div style={{ textAlign: "right" }}>
        <div style={{ fontFamily: "var(--serif)", fontSize: 22, fontWeight: 500 }}>{a.points}<span style={{ color: "var(--muted)", fontSize: 14 }}> / {a.maxPoints}</span></div>
        <button style={{ background: "transparent", border: "none", color: "var(--accent)", fontFamily: "var(--mono)", fontSize: 11, cursor: "pointer", marginTop: 6 }}>See solution →</button>
      </div>
    </div>
  ))}
  <div style={{ padding: "18px 22px", background: "var(--surface-2)", borderTop: "1px solid var(--border)", display: "flex", justifyContent: "flex-end", gap: 10 }}>
    <Button variant="ghost" onClick={() => router.push("/library")}>Back to library</Button>
    <Button onClick={() => router.push("/progress")} icon={<Icon name="arrow" size={14} color="#0b1410" />}>View progress</Button>
  </div>
</Card>
```

`DonutSVG` is a simple inline SVG circle: outer track `var(--surface-2)`, progress arc `var(--accent)`, stroke-dasharray computed from percent.

- [ ] **Step 3: type-check + commit**

```bash
cd web && bun run type-check
git add "web/app/(learner)/sessions/[id]/results/page.tsx" web/e2e/results.spec.ts
git commit -m "feat: rebuild results screen — score donut + per-item review"
```

---

## Task 11: Progress dashboard rebuild

**Files:** `web/app/(learner)/progress/page.tsx`

**Reference:** `design/source/src/screen-dashboard.jsx`

- [ ] **Step 1: Playwright test**

`web/e2e/progress.spec.ts`:
```typescript
import { test, expect } from "@playwright/test";

test("progress shows 5-stat strip", async ({ page }) => {
  await page.context().addCookies([{ name: "ame_token", value: process.env.TEST_TOKEN ?? "", domain: "localhost", path: "/" }]);
  await page.goto("/progress");
  await expect(page.getByText("Progress dashboard")).toBeVisible();
  await expect(page.locator("[data-testid='stat-strip']")).toBeVisible();
});
```

- [ ] **Step 2: Rebuild dashboard page**

Fetch `GET /v1/me/stats` and `GET /v1/me/attempts?limit=200` on mount.

Key structure:
```tsx
<Topbar title="Progress dashboard" breadcrumb="Spring 2026 · last 12 weeks" />

{/* 5-stat strip */}
<Card padding={0} style={{ marginBottom: 18 }} data-testid="stat-strip">
  <div style={{ display: "grid", gridTemplateColumns: "repeat(5,1fr)" }}>
    {[
      { l: "Avg score (12w)", v: `${(stats.avgScore * 100).toFixed(1)}%` },
      { l: "Attempts",        v: String(stats.totalAttempts) },
      { l: "Hours spent",     v: stats.hoursSpent.toFixed(1) },
      { l: "Current streak",  v: `${stats.currentStreakDays} d` },
      { l: "Mastered topics", v: String(stats.masteredTagCount) },
    ].map((s, i) => (
      <div key={s.l} style={{ padding: "22px 24px", borderRight: i < 4 ? "1px solid var(--border)" : "none" }}>
        <Stat label={s.l} value={s.v} />
      </div>
    ))}
  </div>
</Card>

{/* Score trend (simple SVG line — no chart library needed for MVP) */}
<div style={{ display: "grid", gridTemplateColumns: "1.5fr 1fr", gap: 18, marginBottom: 18 }}>
  <Card padding={24}>
    <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, letterSpacing: -0.2, marginBottom: 14 }}>Score trend</div>
    <MiniLineChart attempts={recentAttempts} />
  </Card>
  <Card padding={24}>
    <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, letterSpacing: -0.2, marginBottom: 14 }}>By tag</div>
    <TagBarChart tagStats={tagStats} />
  </Card>
</div>

{/* Mastery map */}
<Card padding={24}>
  <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, letterSpacing: -0.2, marginBottom: 14 }}>Mastery map</div>
  <div style={{ display: "grid", gridTemplateColumns: "repeat(6,1fr)", gap: 6 }}>
    {tagStats.slice(0, 30).map((s, i) => {
      const v = s.total > 0 ? s.correct / s.total : 0;
      return <div key={i} style={{ aspectRatio: "1", background: `color-mix(in oklch, var(--accent) ${Math.round(v*100)}%, var(--surface-2))`, border: "1px solid var(--border)", borderRadius: 3 }} />;
    })}
  </div>
</Card>
```

`MiniLineChart` and `TagBarChart` are simple inline SVG components — no chart library — matching the design's toy chart style.

- [ ] **Step 3: type-check + commit**

```bash
cd web && bun run type-check
git add "web/app/(learner)/progress/page.tsx" web/e2e/progress.spec.ts
git commit -m "feat: rebuild progress dashboard — stat strip + trend + mastery map"
```

---

## Task 12: Exams screen rebuild

**Files:** `web/app/(learner)/exams/page.tsx`

**Reference:** `design/source/src/screen-exams.jsx`

- [ ] **Step 1: Playwright test**

```typescript
test("exams has split list/detail", async ({ page }) => {
  await page.context().addCookies([...]);
  await page.goto("/exams");
  await expect(page.getByText("Exams")).toBeVisible();
  await expect(page.locator("[data-testid='exam-list']")).toBeVisible();
  await expect(page.locator("[data-testid='exam-detail']")).toBeVisible();
});
```

- [ ] **Step 2: Rebuild**

Two-pane layout: `340px 1fr`. Left pane (`data-testid="exam-list"`): vertical stack of exam cards with accent left border on selected, course mono kicker, status Tag, serif title, mono metadata row. Right pane (`data-testid="exam-detail"`): stat strip (5 cols), weight bar, sections table, composition trace.

Weight bar: `display: flex` with each section as `flex: weight` band colored by index cycling `var(--accent)` / `var(--blue)` / `var(--amber)`.

- [ ] **Step 3: type-check + commit**

```bash
cd web && bun run type-check
git add "web/app/(learner)/exams/page.tsx"
git commit -m "feat: rebuild exams screen — split list/detail + weight bar + composition trace"
```

---

## Task 13: Author studio rebuild

**Files:** `web/app/(learner)/author/[quizId]/page.tsx`

**Reference:** `design/source/src/screen-author.jsx`

- [ ] **Step 1: Playwright test**

```typescript
test("author studio has 3-pane layout", async ({ page }) => {
  test.skip(!process.env.TEST_QUIZ_ID, "needs TEST_QUIZ_ID");
  await page.context().addCookies([...]);
  await page.goto(`/author/${process.env.TEST_QUIZ_ID}`);
  await expect(page.locator("[data-testid='questions-list']")).toBeVisible();
  await expect(page.locator("[data-testid='question-editor']")).toBeVisible();
  await expect(page.locator("[data-testid='author-rail']")).toBeVisible();
});
```

- [ ] **Step 2: Rebuild**

Three-pane: `320px flex 280px`.

Left (`data-testid="questions-list"`): list of questions (Q# + prompt preview + type kicker + status Tag + points). Selected row: `accent-dim` background + left accent border. Footer: "Generate questions" callout using `POST /v1/quizzes/generate`.

Center (`data-testid="question-editor"`): header (type label + Duplicate/Move/Delete icons). Form: prompt textarea, options list (for MC — fillable correctness circle + text), points/tag/difficulty row, explanation textarea. Wire to `PATCH /v1/quizzes/{id}`.

Right rail (`data-testid="author-rail"`): 2 cards — Distribution KV + Rubric explainer + Recent activity feed (you vs agent rows with colored bars).

New quiz action in Library "New quiz" button: `POST /v1/quizzes` → navigate to `/author/{id}`.

- [ ] **Step 3: type-check + commit**

```bash
cd web && bun run type-check
git add "web/app/(learner)/author/[quizId]/page.tsx"
git commit -m "feat: author studio 3-pane editor — questions list + editor + rail"
```

---

## Task 14: Agent screen rebuild

**Files:** `web/app/(learner)/agent/page.tsx`

**Reference:** `design/source/src/screen-agent.jsx`

- [ ] **Step 1: Playwright test**

```typescript
test("agent screen has 4 tabs", async ({ page }) => {
  await page.context().addCookies([...]);
  await page.goto("/agent");
  await expect(page.getByRole("tab", { name: "API keys" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "MCP tools" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Import demo" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Recent activity" })).toBeVisible();
});
```

- [ ] **Step 2: Rebuild**

Tab switcher (4 tabs). Each tab body:

**API keys:** `1.4fr 1fr` split. Left: key list — label + masked prefix, Copy/Rotate/Revoke buttons, created/last-used caption, scope tags. Right: auth explainer + sample Bearer curl block + scope reference checklist. Wire to `GET/POST/DELETE /v1/me/keys`.

**MCP tools:** `300px flex` split. Left: scrollable tool list from `GET /v1/agents/mcp.json`. Right: tool detail — method tag + path, Inputs + Returns description, example curl + descriptor JSON block.

**Import demo:** `1.1fr 1fr` split. Left: textarea + format toggle (JSON/MD) + `POST /v1/quizzes` or `POST /v1/questions` endpoint + Send button. Right: response card with status Tag + JSON block.

**Recent activity:** full-width table from `GET /v1/agents/activity`. Columns: Time / Tool (accent mono) / Agent / Status / Note.

- [ ] **Step 3: type-check + commit**

```bash
cd web && bun run type-check
git add "web/app/(learner)/agent/page.tsx"
git commit -m "feat: agent screen — API keys + MCP tools + import demo + activity log"
```

---

## Task 15: Final gate

- [ ] **Step 1: Run full make check**

```bash
make check
```
Expected: fmt + lint + all unit tests pass.

- [ ] **Step 2: Run Playwright e2e suite**

```bash
cd web && bunx playwright test --reporter=list
```
Expected: all non-skipped specs pass.

- [ ] **Step 3: Regenerate OpenAPI snapshot**

```bash
make openapi
git add api/openapi.yaml web/src/api/generated/schema.d.ts
```

- [ ] **Step 4: PR**

```bash
git add -u
git commit -m "chore: regenerate openapi snapshot and schema types after design catchup"
```
Then open PR from `feat/design-catchup` → `main`.

---

## Verification criteria per gap

| Gap | Done when |
|-----|-----------|
| B1  | `test_register_and_login` passes; login page successfully registers → redirects to `/library` |
| B2  | `test_create_quiz` passes; "New quiz" button in Library creates a quiz and navigates to Author |
| B3  | `test_me_stats` passes; Progress dashboard renders real numbers |
| F1  | `getComputedStyle(document.documentElement).getPropertyValue('--serif')` returns a non-empty value in browser |
| F2  | Font waterfall: page uses Source Serif 4 for `h1` elements (visible in DevTools → Computed) |
| F3  | `bun run type-check` passes with zero errors importing from `@/components/ui` |
| F4  | Sidebar shows icons next to every nav item; Logo appears top-left |
| F5  | Playwright `auth.spec.ts` — both tests pass |
| F6  | Playwright `library.spec.ts` — hero card + grid visible |
| F7  | Playwright `quiz.spec.ts` — setup screen with Start disabled when no questions |
| F8  | Active quiz: progress bar fills as questions answered; palette shows answered state |
| F9  | Results: donut SVG renders; per-item rows have "See solution →" |
| F10 | Progress: 5-stat strip shows real numbers from `/v1/me/stats` |
| F11 | Exams: selecting exam in list populates detail pane with weight bar |
| F12 | Author: editing question prompt updates the list preview |
| F13 | Agent: "API keys" tab lists keys from `/v1/me/keys` |
