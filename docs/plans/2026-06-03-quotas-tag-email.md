# Plan: per-plan resource quotas + tag normalization + email-alias dedup

> Status: PLANNED. Branch `feat/rate-limit-quota` already carries the
> rate-limit/config/valkey core (commits `77ea43a`, `16a6e95`). Build the items
> below on top of that branch. Backend-only except the migration + seed.

## Decisions (locked with Haru 2026-06-03)

- Quota the **owner-created DB resources**: assessments, questions, agents.
  **No tag quota** (see tag note below).
- Limits **generous**: assessments & questions `free=50 / premium=5000`; agents
  stay `free=1 / premium=100`.
- Quota limits live in **config** (`ame.toml`), mirrored huge in `ame.dev.toml`.
- Tags: **no `owner_id` schema change.** Tags are a shared name-dedup table but
  visibility is already owner-scoped via the RLS-protected question join
  (`list_tags`), so Ada/Mira never see each other's tags. Just normalize names.
- Tag canonical separator = **hyphen**.
- Email dedup = **plus-strip + lowercase** (provider-agnostic), via a canonical
  column, not by mutating the stored address.

## Key invariants (do not get these wrong)

- **Count with explicit owner filters on `state.pool`, NOT via RLS.**
  `tb_assessments` has **no** RLS policy (only `tb_questions` does), so a bare
  `COUNT(*)` would count every owner's rows. Use:
  - assessments: `created_by IN (SELECT id FROM tb_users WHERE id=$owner OR owner_user_id=$owner)`
  - questions: `WHERE owner_id = $owner`
  - agents (existing): `owner_user_id = $owner AND role = 'agent'`
- Charge to `user.owner_id` (= `owner_user_id.unwrap_or(id)`; human→self, agent→owner).
- **Charge the whole batch**: reject when `usage + add_count > limit`, not `+1`.
- `ame.dev.toml` quotas MUST be huge (e.g. 1_000_000) or `db-bulk` (premium,
  thousands of rows) / `db-seed` / e2e trip the 5000 cap and fail.

## Phase 1 — quota limits → config

- `api/src/config.rs`: add `[quota]` with `agents`, `assessments`, `questions`,
  each `{ free: i64, premium: i64 }`. Add `QuotaConfig` to `Config`.
- `ame.toml`: assessments/questions `free=50, premium=5000`; agents `free=1,
  premium=100`.
- `ame.dev.toml`: all quota values `1_000_000`.

## Phase 2 — `api/src/http/quota.rs`

- `QuotaKind::{AgentCreation, Assessment, Question}` (no `Tag`).
- Pull limits from config instead of the hardcoded `match`.
- Signature: `check_quota(pool, owner_id, kind, add_count: i64)` — count current
  usage (queries above), reject if `usage + add_count > limit` with
  `ApiError::QuotaExceeded { kind, limit, usage }`.

## Phase 3 — wire call sites (before the insert, charged to `user.owner_id`)

- `assessments.rs::create_assessment` — Assessment +1, Question +N (inline `questions.len()`)
- `assessments.rs::create_assessment_section` / add-items — Question +N
- `agents.rs::run_assessment_batch_create` — Assessment +1, Question +N
  (replaces the raw item-count cap concern; the rate-limit burst cap stays)
- standalone question create (`bank`/`http::questions`) — Question +1/+N

## Phase 4 — tag normalization (no migration; reseed picks it up)

- `bank/tags.rs::normalize_tag(name) -> String`: trim → lowercase → collapse any
  run of whitespace/`-`/`_` into a single `-` → strip leading/trailing `-`.
  So `"A B"`, `"a-b"`, `"a_b"`, `" a  b "` all → `"a-b"`.
- Apply at all four ingestion points: `create_tag`, `get_or_create_tags_tx`, and
  the two `INSERT INTO tb_tags ... ON CONFLICT (name)` upserts in
  `bank/questions.rs`.
- Update demo tag names in `scripts/seed.py` and any literal tag assertions, then
  `make db-reset` to re-slug.

## Phase 5 — email-alias dedup (additive migration)

- New migration: `ALTER TABLE tb_users ADD COLUMN email_canonical text`; partial
  `UNIQUE` index `WHERE email_canonical IS NOT NULL` (agents have NULL email);
  backfill existing rows
  (`lower(split_part(local,'+',1)) || '@' || lower(domain)`).
- `canonical_email(email) -> String` in Rust: lowercase, strip `+suffix` from the
  local part; keep raw `email` for login/delivery.
- `http::auth::register`: set `email_canonical`; map its unique violation to the
  existing "email already registered" 422.

## Verification

- `make check` green after each phase.
- Behavior checked on the running stack (Haru): free user blocked at 50
  assessments/questions; premium at 5000; batch over remaining quota → 429;
  `"A B"` and `"a-b"` collapse to one tag; `abc+1@x.com` after `abc@x.com` →
  "email already registered".
