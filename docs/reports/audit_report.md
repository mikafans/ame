# AME v0.2.0 Pre-Release Code Audit Report

This report presents the findings of a comprehensive code review of the `ame` platform backend, conducted prior to the `v0.2` release. The audit covers logical correctness, database constraint mismatches, performance/concurrency race conditions, and API design.

---

## Executive Summary of Findings

| ID | Issue Title | Severity | Affected Component(s) | Impact |
| :--- | :--- | :--- | :--- | :--- |
| **BUG-01** | Code Question Grading Ignores Exemplar | **Critical** | `api/src/engine/graders.rs` | Automated grading for code questions fails, forcing manual review even when an exact match exemplar is provided. |
| **BUG-02** | Idempotency Key Insertion Causes 500 Error for Humans | **Critical** | `db/migrations/...`, `api/src/http/idempotency.rs` | Any write request (`POST`/`PATCH`) containing an `Idempotency-Key` header sent by a human user crashes with a database foreign key constraint violation. |
| **BUG-03** | Idempotency Middleware is Never Mounted in Production | **Major** | `api/src/http/mod.rs` | The idempotency engine is dead code in the actual web server, leaving the production routes vulnerable to duplicate form submissions and double-writes. |
| **BUG-04** | Sub-Second Truncation Skips Paged Questions | **Major** | `api/src/bank/questions.rs` | Question list pagination cursor truncates timestamps to seconds, causing questions created within the same second to be permanently skipped on page transitions. |
| **BUG-05** | Accent Stripping Normalization is Unimplemented | **Medium** | `api/src/engine/graders.rs` | `Normalize::CaseInsensitiveStripAccents` only lowercases strings. Answers differing only by accents (e.g., `"café"` vs `"cafe"`) are graded incorrectly. |
| **BUG-06** | Concurrent Replays Cause Double-Execution Race Condition | **Medium** | `api/src/http/idempotency.rs` | Concurrent identical requests bypass the idempotency check and execute side effects twice because locking is performed at response-write rather than request-start. |
| **BUG-07** | Quota System Counts Retired/Archived Questions | **Low** | `api/src/http/quota.rs` | Archived questions continue to count towards user question quotas, eventually locking active authors out of question creation. |

---

## Detailed Findings & How-to-Fix

### BUG-01: Code Question Grading Ignores Exemplar
- **Severity**: **Critical**
- **Affected File**: [`api/src/engine/graders.rs`](file:///home/haru/Projects/project-github/ame/api/src/engine/graders.rs)
- **Description**: The grading logic for code questions (`QuestionKind::Code`) always returns `GradeStatus::PendingManual` and `correct: false` (lines 189–205). It binds `source` to `_` and never compares the submitted code to the question's `payload.exemplar` field, even if it is set.
- **Implication**: Students who submit code matching the instructor's exemplar exactly are marked incorrect, and the system fails to automate grading.
- **How to Fix**: Update `grade_response` in `api/src/engine/graders.rs` to extract `source` and compare its whitespace-trimmed version with the trimmed `exemplar`.

```diff
-        (QuestionKind::Code, AttemptResponse::Code { source: _, .. }) => {
+        (QuestionKind::Code, AttemptResponse::Code { source, .. }) => {
             let payload: CodePayload = serde_json::from_value(payload.clone()).map_err(|e| {
                 GradeError::InvalidPayload {
                     kind,
                     reason: e.to_string(),
                 }
             })?;
 
+            if let Some(ref exemplar) = payload.exemplar {
+                let trimmed_source = source.trim();
+                let trimmed_exemplar = exemplar.trim();
+                if trimmed_source == trimmed_exemplar {
+                    return Ok(GradeOutcome {
+                        status: GradeStatus::Graded,
+                        correct: true,
+                        points_awarded: max_points,
+                        max: max_points,
+                        correct_answer: serde_json::json!({ "language": payload.language, "exemplar": payload.exemplar }),
+                        note: Some("Matches exemplar exactly".to_string()),
+                    });
+                }
+            }
+
             Ok(GradeOutcome {
                 status: GradeStatus::PendingManual,
                 correct: false,
                 points_awarded: 0,
                 max: max_points,
                 correct_answer: serde_json::json!({ "language": payload.language, "exemplar": payload.exemplar }),
                 note: None,
             })
         }
```

---

### BUG-02: Idempotency Key Insertion Causes 500 Error for Humans
- **Severity**: **Critical**
- **Affected Files**: 
  - [`db/migrations/20260602000001_baseline_sessions.sql`](file:///home/haru/Projects/project-github/ame/db/migrations/20260602000001_baseline_sessions.sql)
  - [`api/src/http/idempotency.rs`](file:///home/haru/Projects/project-github/ame/api/src/http/idempotency.rs)
- **Description**: The `tb_idempotency_keys` table has a foreign key constraint `token_id uuid NOT NULL REFERENCES tb_api_tokens(id)`. Human users authenticate using login sessions (`tb_login_sessions`), which are separate from API tokens (`tb_api_tokens`). If a human user sends a request with an `Idempotency-Key` header, the middleware tries to insert the login session's UUID into `tb_idempotency_keys.token_id`, triggering a foreign key constraint violation.
- **Implication**: Human users experience `500 Internal Server Error` crashes whenever their client includes an `Idempotency-Key` header on a request.
- **How to Fix**: Create a database migration to drop the foreign key constraint on `token_id` in `tb_idempotency_keys` so that the column can hold either login session IDs or API token IDs (which only live for a short 48-hour retry window anyway).

#### Step 1: SQL Migration
Create a new migration file `db/migrations/20260614000000_drop_idempotency_keys_fk.sql`:
```sql
-- Drop the foreign key constraint referencing tb_api_tokens
ALTER TABLE tb_idempotency_keys DROP CONSTRAINT IF EXISTS tb_idempotency_keys_token_id_fkey;
```

---

### BUG-03: Idempotency Middleware is Never Mounted in Production
- **Severity**: **Major**
- **Affected File**: [`api/src/http/mod.rs`](file:///home/haru/Projects/project-github/ame/api/src/http/mod.rs)
- **Description**: The `idempotency_middleware` is declared and exported, but it is **never** applied in the actual `router(pool)` setup in `api/src/http/mod.rs`. It is only mounted in the test router inside `api/tests/auth.rs`.
- **Implication**: The production backend has zero request idempotency protection. Retried client requests (due to timeout or network blips) will execute side-effects (such as starting new attempts, subtracting quotas, and updating ELO) multiple times.
- **How to Fix**: Apply the `idempotency_middleware` as a layer on the appropriate POST/PATCH routes in `api/src/http/mod.rs`.

```diff
     // Endpoints that should be logged (activity_log)
     let logged_router = Router::new()
         .merge(assessments::router(state.clone()))
         .merge(sessions::router(state.clone()))
         .merge(me::router(state.clone()))
         .merge(plans::router(state.clone()))
         .merge(agents::logged_router(state.clone()))
         .merge(messages::router(state.clone()))
         .route("/v1/me/export", axum::routing::get(export::export_data))
+        .route_layer(middleware::from_fn_with_state(
+            state.clone(),
+            idempotency::idempotency_middleware,
+        ))
         .layer(middleware::from_fn_with_state(
             state.clone(),
             activity::activity_log_middleware,
         ));
```

---

### BUG-04: Sub-Second Truncation Skips Paged Questions
- **Severity**: **Major**
- **Affected Files**: 
  - [`api/src/bank/questions.rs`](file:///home/haru/Projects/project-github/ame/api/src/bank/questions.rs)
  - [`api/src/http/questions.rs`](file:///home/haru/Projects/project-github/ame/api/src/http/questions.rs)
- **Description**: The next page cursor is generated by formatting the Unix timestamp in seconds (`q.created_at.unix_timestamp()`). When parsed back in `decode_cursor`, the sub-second component is truncated to `.000000`. The SQL query checks `(q.created_at, q.id) < ($9, $10)`. If a row was created at `12:00:00.123456`, it evaluates to `q.created_at < 12:00:00.000000`.
- **Implication**: Any question rows created in the same second as the cursor row but with non-zero sub-seconds (e.g. `12:00:00.123456`) are skipped entirely when requesting the next page. This is highly common with batch-seeding or rapid automated insertions.
- **How to Fix**: Use nanosecond or microsecond precision inside the cursor string representation.

#### In `api/src/bank/questions.rs`:
```diff
 pub fn decode_cursor(cursor: &str) -> Option<(OffsetDateTime, Uuid)> {
     let parts: Vec<&str> = cursor.split('_').collect();
     if parts.len() != 2 {
         return None;
     }
 -    let ts = OffsetDateTime::from_unix_timestamp(parts[0].parse().ok()?).ok()?;
 +    let ns = parts[0].parse::<i128>().ok()?;
 +    let ts = OffsetDateTime::from_unix_timestamp_nanos(ns).ok()?;
     let id = Uuid::parse_str(parts[1]).ok()?;
     Some((ts, id))
 }
```

```diff
     let next_cursor = if has_more {
         rows_vec
             .last()
 -            .map(|q| format!("{}_{}", q.created_at.unix_timestamp(), q.id))
 +            .map(|q| format!("{}_{}", q.created_at.unix_timestamp_nanos(), q.id))
     } else {
         None
     };
```

---

### BUG-05: Accent Stripping Normalization is Unimplemented
- **Severity**: **Medium**
- **Affected File**: [`api/src/engine/graders.rs`](file:///home/haru/Projects/project-github/ame/api/src/engine/graders.rs)
- **Description**: The normalization method `Normalize::CaseInsensitiveStripAccents` only maps to `.trim().to_lowercase()` (lines 134 and 140). It does not perform any diacritic/accent removal.
- **Implication**: Users who submit correctly spelled answers but omit accents (or vice-versa) on questions configured with `CaseInsensitiveStripAccents` (e.g. `"café"` vs `"cafe"`) are graded as incorrect.
- **How to Fix**: Add the `unicode-normalization` crate to `api/Cargo.toml` and implement helper logic to decompose characters and filter out combining diacritical marks.

#### In `api/Cargo.toml`:
```toml
unicode-normalization = "0.1.24"
```

#### In `api/src/engine/graders.rs`:
```rust
use unicode_normalization::UnicodeNormalization;

fn strip_accents(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}
```
Then use it inside the normalization matcher:
```rust
            let given = match payload.normalize {
                Normalize::Exact => answer.trim().to_string(),
                Normalize::CaseInsensitiveStripAccents => strip_accents(&answer.trim().to_lowercase()),
            };
```

---

### BUG-06: Concurrent Replays Cause Double-Execution Race Condition
- **Severity**: **Medium**
- **Affected File**: [`api/src/http/idempotency.rs`](file:///home/haru/Projects/project-github/ame/api/src/http/idempotency.rs)
- **Description**: The middleware queries the DB for an existing key. If it doesn't exist, it proceeds to execute the request handler (`next.run(req)`). The key is only inserted at the end when writing the response. If two identical requests hit the middleware at the same time, both will see `None` and run the request handler concurrently.
- **Implication**: Double execution of write actions (double creation of sessions, double quota usage, etc.) still occurs if the client retries rapidly before the first request finishes writing.
- **How to Fix**: Implement an "in-progress" lock state. 
  1. Insert a placeholder row at the beginning of the middleware with a temporary status (e.g. `-1` representing "in progress").
  2. If the insertion fails due to a unique key conflict, check the status:
     - If the status is `-1`, a concurrent request is currently in flight. Return `409 Conflict` (or `503 Service Unavailable` with a `Retry-After` header).
     - If the status is a real response code, replay the cached response.
  3. Once the handler finishes executing, update the placeholder row with the actual response status and body.

---

### BUG-07: Quota System Counts Retired/Archived Questions
- **Severity**: **Low / Improvement**
- **Affected File**: [`api/src/http/quota.rs`](file:///home/haru/Projects/project-github/ame/api/src/http/quota.rs)
- **Description**: The query `SELECT COUNT(*) FROM tb_questions WHERE owner_id = $1` in `check_quota` counts all questions owned by the user, including those that have been retired and marked as `status = 'archived'`.
- **Implication**: Over time, active authors who retire and archive old questions will run out of quota permanently and be unable to create new questions.
- **How to Fix**: Update the quota counting query to exclude archived questions:
```sql
SELECT COUNT(*) FROM tb_questions WHERE owner_id = $1 AND status != 'archived'
```

---

## Conclusion

By fixing the logical gaps in **BUG-01** (Code Grader), **BUG-02** (Idempotency Key DB Constraint), **BUG-03** (Middleware mounting), and **BUG-04** (Cursor pagination), AME `v0.2` will be structurally stable and ready for production release.

Detailed instructions on how to run tests to verify these changes are detailed in the project's [CLAUDE.md](file:///home/haru/Projects/project-github/ame/CLAUDE.md).
