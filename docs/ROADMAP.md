# AME Roadmap — toward v1.1 "Admin Console GA"

**Current version:** `v0.1.0` (`api/Cargo.toml`)
**Target:** `v1.1` — a complete, operable admin console. The four operator
features below are the v1.1 epic; they ship incrementally across v0.2 → v1.0 and
are integrated + hardened at v1.1.

The features are ordered by **dependency and operational risk**, not by the order
listed — token revocation and feature flags are foundational control-plane tools
an operator needs *before* the platform has real traffic; the analytics
dashboard depends on the metrics pipeline (Phase D) and so comes last.

## At a glance

| Version | Theme | Operator features delivered | Hard dependency |
|---------|-------|------------------------------|-----------------|
| v0.1 (now) | Platform + Admin foundation | Users, audit, health, assessment moderation (soft-delete) | — |
| v0.2 | Operational control plane | **#1 Token Audit**, **#2 Settings & Feature Flags** | admin shell (done) |
| v0.3 | Content integrity + data hygiene | **#3 Feedback / Flags / Support Queue**, **data retention & pruning** | learner session UI |
| v1.0 | Insight + hardening | **#4 Analytics Dashboard** | Phase D metrics |
| v1.1 | GA polish | (all four integrated) | v1.0 |

---

## v0.1 — Platform + Admin foundation *(current)*

Already shipped or in flight on `feat/admin-api`:

- Core platform: auth (email+password), assessments (draft→active), agents
  (role=agent + profiles), per-plan quotas, tiered per-owner rate limiting.
- Admin shell: paginated/filtered **users** (promote/demote with self-escalation,
  self-demote, self-disable and last-admin guards), **audit log**, **system
  health**, **assessment moderation** (recoverable soft-delete + restore).
- Ops safety: `/readyz` + `/healthz` probes, daily `pg_dump` backup cronjob,
  RUNBOOK.
- **In flight (Gemini):** Phase D observability — business/metrics endpoints.
  This is the data source v1.0 analytics builds on.

**Exit criteria:** admin can run day-to-day moderation and the service has
restart/backup safety.

---

## v0.2 — Operational control plane

The two tools an operator needs to *react* to incidents without a redeploy.

### #1 — Global API Key & Agent Token Audit
Plan: [`docs/plans/2026-06-03-admin-token-audit.md`](plans/2026-06-03-admin-token-audit.md)
- `GET /v1/admin/tokens` (paginated; filter by `q`/`status`/`role`/`owner_id`;
  `token_hash` never exposed) and `DELETE /v1/admin/tokens/{id}` (idempotent
  global revoke, audited).
- `/admin/tokens` page (human + agent tokens in one view).
- No migration — reuses `tb_api_tokens`.

### #2 — Global Platform Settings & Feature Flags
- New `tb_settings` (key/value/jsonb + `updated_by` + `updated_at`) read through a
  small cached settings service.
- Operator-tunable without redeploy: **maintenance mode** (middleware returns 503
  for non-admins), default **rate-limit burst/rate**, and **quota** values now
  baked into config.
- `GET/PUT /v1/admin/settings` + `/admin/settings` UI; every change audited.
- **Risk to manage:** these flags gate live request handling — needs a safe
  default + fail-open-to-config-file path if the store is unreachable.

**Exit criteria:** an operator can revoke a leaked key and flip maintenance mode
in under a minute, no deploy.

---

## v0.3 — Content integrity

### #3 — Feedback, Flags & Support Queue
- Learner-facing "flag this question" during practice/exam → new
  `tb_content_flags` (target type/id, reporter, reason, status:
  open/resolved/dismissed).
- `/admin/flags` moderation queue: review flagged questions/assessments, jump to
  the item, edit an incorrect answer key, or remove content (reuse the
  soft-delete path from v0.1). Resolution audited.
- Ties into existing assessment/question moderation rather than a parallel system.

**Exit criteria:** a wrong answer key reported by a learner is visible to admins
and fixable in-console.

### Data retention & pruning *(must precede v1.0)*
Plan: [`docs/plans/2026-06-03-data-retention.md`](plans/2026-06-03-data-retention.md)
- Scheduled, batched pruning of transient/terminal rows
  (`tb_idempotency_keys` — currently leaking with no cleanup, `tb_activity_log`,
  `tb_webhook_deliveries`, read `tb_messages`, expired/revoked tokens,
  long-soft-deleted assessments, aged audit rows). Learning/analytics data is
  never auto-pruned.
- Daily k8s CronJob (mirrors the backup cronjob), env-configurable windows,
  starts in dry-run, one audit row per run.
- **Blocks v1.0** so analytics builds on a bounded, healthy DB — and resolves the
  `tb_activity_log` grade-count dependency *with* the Phase D metrics owner.

**Exit criteria:** every fast-growing table has an enforced retention policy;
PG size is bounded by usage, not by uptime.

---

## v1.0 — Insight + hardening

### #4 — Platform Analytics Dashboard
- Built on the **Phase D metrics** from v0.1. Charts: DAU, attempts/day,
  registration growth, most-failed tags.
- `GET /v1/admin/analytics?range=...` (server-side aggregation, cached) +
  `/admin/analytics` page (MUI charts).
- Read-only; no new write paths.

### Hardening pass (gates the 1.x line)
- Security review of all `/v1/admin/*` endpoints (authz, audit coverage,
  no secret leakage).
- Load/perf check on the heaviest admin list queries at realistic row counts.
- E2E coverage for each admin feature.

**Exit criteria:** all four features functionally complete; security + perf
reviewed.

---

## v1.1 — GA polish

Integration and finish work, no new feature surface:

- Consistent admin nav/IA across Users · Assessments · Tokens · Settings ·
  Flags · Analytics · Audit · Health.
- Accessibility + empty/error/loading states across admin pages.
- Operator docs: extend RUNBOOK with the new control-plane actions.
- Final audit-log completeness review (every state-changing admin action emits an
  audit row).

**Exit criteria:** the admin console is something a non-author operator can run
the platform with, confidently. Tag `v1.1.0`.

---

## v2.0 — High Scale & Performance (The "1M Concurrent Users" Problem)

Targeting extreme load spikes (e.g., 1 million concurrent users opening assessment sessions simultaneously) by redesigning the hot paths to eliminate database bottlenecks and network saturation.

### #5 — In-Memory Assessment & Question Plan Caching
- **Goal**: Eliminate SQL read queries when creating sessions.
- **Plan**: Cache published assessment objects (along with their linked sections and questions) in local memory using `moka` or `dashmap` in Rust, or as pre-compiled JSON blobs in Valkey/Redis. Bypasses multi-table SQL joins on `tb_assessments`, `tb_assessment_sections`, `tb_assessment_items`, and `tb_questions`.

### #6 — Asynchronous Session Write Buffering
- **Goal**: Decouple the HTTP response latency from Postgres write transactions.
- **Plan**: 
  - When a user calls `POST /v1/sessions`, generate the session UUID, compile the session question list, push the creation event to a Valkey Stream or message queue, and immediately return a `201 Created` response.
  - Run background worker groups to pull session creation events and perform bulk/batched inserts (`COPY` or batched inserts) into `tb_sessions` and `tb_session_questions`, shifting high-write database contention out of the request-response cycle.

### #7 — Horizontal Scaling & Connection Multiplexing
- **Goal**: Bounded OS resources and stable connection pool limits.
- **Plan**:
  - Horizontal pod auto-scaling (HPA) for the Axum API server.
  - Deploy Envoy or ingress controller configured for HTTP/2 multiplexing and active load shedding.
  - Implement active queueing/backpressure on the database connection pool using tools like `pgbouncer` or internal pool queuing.

---

## Cross-cutting principles

- Every state-changing admin action writes to `tb_audit_log`.
- Destructive-but-recoverable (assessments) uses soft-delete + restore;
  terminal-by-design actions (token revoke) say so in the UI.
- No secrets (token hashes, password hashes) ever cross the API boundary.
- Admin reads run on the unscoped pool (RLS is `tb_questions`-only) — keep it
  that way deliberately, not by accident.

## Out of scope for the 1.x line

- SSO/SAML, scheduled exams, public sharing (all explicitly not supported).
- Multi-region / HA Postgres, per-token traffic forensics (last-IP, full request
  logs) beyond the v1.0 analytics aggregates.
