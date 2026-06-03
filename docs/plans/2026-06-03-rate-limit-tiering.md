# Plan: Tiered, per-owner rate limiting + config + valkey

> Status: PLANNED — depends on the scaling/batch stack landing first. Build as
> its own branch/PR after that merges.

## Goal

Replace the static, env-tuned `tower_governor` limiters with a **per-owner,
plan-tiered** rate limiter backed by **valkey** (multi-instance correct), driven
by a **config file** instead of scattered env vars. Fold the batch-size cap into
the limiter so there is no magic `MAX_BATCH` constant.

## Non-goals

- No new plan tiers — only the existing `free` / `premium` (`tb_users.plan`).
- No change to auth/token issuance or scopes.
- Not a general caching framework — only the two hot caches below.

## Background

- Three limiters exist today, all **static, built once at startup**
  (`api/src/http/mod.rs`): global per-**token** (burst 100, 1/s), export
  (1/60s), public per-IP (burst 10, 1/2s). Tunables are env vars
  (`AME_GLOBAL_RATELIMIT_BURST`, `_PERIOD_SECS`, `AME_RATELIMIT_*`) — the "magic"
  to remove.
- `tower_governor` can't vary rate per request, so tiering and weighted cost
  require replacing the global layer with our own limiter.
- **Multi-instance breaks every in-memory limiter** — with N replicas each node
  allows the full limit, so effective limit = N×. All three must move to a
  shared store (valkey).
- The auth extractor (`api/src/auth/extractor.rs:92`) runs a DB query on **every
  authenticated request** to resolve user + owner + `plan`. That query both
  feeds the limiter (needs `owner_id` + `plan`) and is the prime cache target.
- `owner_plan = COALESCE(owner.plan, user.plan)` is already resolved — so
  sub-accounts and agent tokens inherit the owner's tier.

## Design (locked)

| Aspect        | Decision                                                              |
| ------------- | -------------------------------------------------------------------- |
| Bucket key    | `owner_id` — all sub-accounts/agent tokens share one pool            |
| Tier          | `owner_plan` → `free` \| `premium`                                   |
| Limits        | free: burst 60, +1/s · premium: burst 600, +10/s                    |
| Batch cost    | weighted = item count; batch > tier burst → 429 (tier-aware message) |
| Tunables      | `config.toml`; env reserved for secrets only                         |
| Backend       | valkey — token buckets + auth/plan cache + aggregate cache           |
| Limiters move | **all three** (global per-owner, public per-IP, export) → valkey     |
| Valkey down   | **fail-open** with a conservative in-process fallback bucket         |

Effective max batch = tier burst (free 60 / premium 600). No `MAX_BATCH` const.

## Phases (each a dispatchable unit)

1. **Config system** — `config.toml` loaded once into `AppState`
   (`[ratelimit.free]`, `[ratelimit.premium]`, `[ratelimit.public]`,
   `[ratelimit.export]`, `[batch]`). Delete the `AME_*RATELIMIT*` env reads.
   `toml` + `serde`; env only for `DATABASE_URL`/`VALKEY_URL`/secrets.

2. **Valkey infra** — add the service to `db/docker-compose.yml` (+ healthcheck),
   add `deadpool-redis` (or `fred`) dep, build a pool into `AppState`,
   `make`-wire it into `dev`/`db-up`.

3. **Distributed tiered limiter** — atomic token-bucket Lua script keyed by
   `owner_id`; rate/burst from config by `owner_plan`. A `RateLimiter` service in
   `AppState` exposing `try_consume(owner_id, plan, cost)`. Auth-mounted
   middleware charges 1; `assessment.batchCreate` charges `items.len()` and
   rejects > burst. Move public (per-IP) + export onto valkey too. Fail-open with
   in-process fallback when valkey is unreachable.

4. **Caches** — (a) auth/plan: cache token → `{owner_id, plan, scopes}` with
   ~30s TTL **and** explicit invalidation on plan upgrade (free→premium must not
   lag). (b) aggregates: cache the expensive `explore/facets` counts + the
   activity-log stats `COUNT(*)` with short TTL.

## Files (anticipated)

```
config.toml                       (new — tier tunables)
api/src/config.rs                 (new — loader)
api/src/ratelimit.rs              (new — limiter service + Lua)
api/src/cache.rs                  (new — valkey-backed caches)
api/src/http/mod.rs               (drop env governors; wire limiter)
api/src/http/agents.rs            (batchCreate weighted charge)
api/src/auth/extractor.rs         (read-through auth/plan cache)
api/Cargo.toml                    (deadpool-redis / fred)
db/docker-compose.yml             (valkey service)
Makefile                          (valkey in dev/db-up)
```

## Verification

- Single instance: `make check` green; limits enforced; batch > burst → 429 with
  upgrade message; premium gets 10× the free bucket.
- Multi-instance: two API instances against one valkey share a bucket (combined
  traffic hits the limit, not 2×).
- Plan upgrade takes effect within the cache TTL (or immediately on invalidation).
- Valkey down → API stays up (fail-open), limiter falls back in-process.
