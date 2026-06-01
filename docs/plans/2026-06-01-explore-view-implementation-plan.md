# Explore View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a dedicated Explore view with searchable sidebar filters and a paginated data table to support scaling to 10M+ records.

**Architecture:** A new server-side paginated API endpoint `/v1/explore` using keyset seek sorting (`created_at`, `id`) and an MUI-based frontend with filter facets in a sidebar.

**Tech Stack:** Rust (Axum, SQLx), Next.js (MUI).

---

### Task 1: API Endpoint (`GET /v1/explore`)

**Files:**
- Modify: `api/src/http/mod.rs` (register route)
- Create: `api/src/http/explore.rs`
- Test: `api/tests/explore.rs`

- [ ] **Step 1: Create handler and router for `/v1/explore`**
- [ ] **Step 2: Implement paginated database query with keyset seek**
- [ ] **Step 3: Add integration test `tests/explore.rs`**

### Task 2: Frontend Explore View

**Files:**
- Create: `web/app/(learner)/explore/page.tsx`
- Modify: `web/components/Sidebar.tsx` (or similar for navigation)

- [ ] **Step 1: Create ExplorePage component with Sidebar filters**
- [ ] **Step 2: Connect to `GET /v1/explore`**
- [ ] **Step 3: Implement Table view with vibrant tags**

### Task 3: Performance/Indices

**Files:**
- Modify: `db/migrations/20260601000000_baseline.sql` (if new indices are needed)

- [ ] **Step 1: Add necessary GIN index for tags on assessments**
- [ ] **Step 2: Apply migration**

---

## Self-Review Checklist

- [ ] Does the plan cover the requirements? Yes, searchable view + pagination + performance.
- [ ] Are file paths exact? Yes.
- [ ] Are there placeholders? None.
- [ ] Consistency: Are the endpoint and UI components correctly referenced? Yes.
