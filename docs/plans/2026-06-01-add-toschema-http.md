# Add ToSchema to HTTP Response/Request Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `#[derive(utoipa::ToSchema)]` to several structs and enums across the `api/src` directory to fix OpenAPI generation and ensure all types used in API responses/requests are correctly represented in the schema.

**Architecture:** We will surgically update structs and enums in the HTTP layer to include `utoipa::ToSchema`. We will also fix outdated references in `api/src/http/openapi.rs` that point to non-existent types or old names.

**Tech Stack:** Rust, utoipa, axum.

---

### Task 1: Update admin.rs

**Files:**
- Modify: `api/src/http/admin.rs`

- [ ] **Step 1: Add ToSchema to ListUsersResponse, PatchUserAdminBody, AuditLogEntry, ListAuditLogsResponse, and ModerateBody.**

```rust
// api/src/http/admin.rs
use utoipa::ToSchema;
```

- [ ] **Step 2: Verify and add ToSchema derivation.**

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct ListUsersResponse { ... }

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchUserAdminBody { ... }

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogEntry { ... }

#[derive(Debug, Serialize, ToSchema)]
pub struct ListAuditLogsResponse { ... }

#[derive(Debug, Deserialize, ToSchema)]
pub struct ModerateBody { ... }
```

### Task 2: Update auth.rs

**Files:**
- Modify: `api/src/http/auth.rs`

- [ ] **Step 1: Add ToSchema to AuthResponse and UserInfo.**

```rust
// api/src/http/auth.rs
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse { ... }

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo { ... }
```

### Task 3: Update agents.rs and me.rs

**Files:**
- Modify: `api/src/http/agents.rs`
- Modify: `api/src/http/me.rs`

- [ ] **Step 1: Add ToSchema to ActivityResponse in agents.rs.**

```rust
// api/src/http/agents.rs
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResponse { ... }
```

- [ ] **Step 2: Add ToSchema to AgentSummary in me.rs.**

```rust
// api/src/http/me.rs
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary { ... }
```

### Task 4: Update quota.rs

**Files:**
- Modify: `api/src/http/quota.rs`

- [ ] **Step 1: Add ToSchema to Plan.**

```rust
// api/src/http/quota.rs
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Free,
    Premium,
}
```

### Task 5: Update stats.rs

**Files:**
- Modify: `api/src/http/stats.rs`

- [ ] **Step 1: Ensure ToSchema is on QuizStatsParams (if used as body/params), QuizStatsResponse, DistributionBucket, ItemStats, ExamStatsResponse, SectionAvg, MeStatsQuery, MeStatsResponse.**

```rust
// api/src/http/stats.rs
use utoipa::ToSchema;
```

### Task 6: Update openapi.rs to fix outdated references

**Files:**
- Modify: `api/src/http/openapi.rs`

- [ ] **Step 1: Update imports and components in openapi.rs.**
Remove non-existent types like `AdminAuditResponse`, `AdminStatsResponse`, `AdminUserResponse`, `AuditEntry`, `TagPerformance`, `ListTagsResponse`.
Use correct names like `ListUsersResponse`, `ListAuditLogsResponse`, `AuditLogEntry`, `Plan`.

---
