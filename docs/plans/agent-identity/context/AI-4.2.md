# AI-4.2 — Plan + quota model, governor re-key, enforcement

**Phase:** 4 · **Lane:** api · **Agent:** opus · **Depends on:** AI-4.1
**Spec:** §"Rate limits & quotas" + §"Authorization → Scope ceiling"
**Paths (edit only here):** `api/src/domain/`, `api/src/quota.rs` (new), `api/src/http/mod.rs`

## Objective
Two limiters: (1) per-token **throughput** via `tower_governor`, re-keyed from IP
to token id; (2) per-owner **plan quotas** enforced server-side before a write.

## Do
- `Plan` enum in `api/src/domain/` + a quota table (consts) per the spec matrix
  (agents, req/min/token, question-creates/day, public quizzes, export).
- New `api/src/quota.rs`: a `check_quota(owner_id, QuotaKind)` that counts
  current usage and returns `Err(429)` with a clear body when over. Resolve plan
  via the owner (`owner_user_id`), so agents inherit.
- In `api/src/http/mod.rs`, re-key the `GovernorLayer` from the default
  IP extractor to the token id (custom key extractor).

## Read first
- `api/src/http/mod.rs:78–95` — current `GovernorLayer` config (IP-based).
- `AuthenticatedUser.owner_id` (AI-1.2).

## Gotchas
- Quota checks run **before** the write, in the handler path. AI-3.2 (public
  publish), AI-1.3 (agent count), AI-5.1 (export) call into `quota.rs` — keep
  its API small and stable so they wire in without touching this file again.
- Numbers are a starting point; centralize them so they're tunable.

## Done when
- Governor keyed by token; `check_quota` returns 429 over-limit with a clear
  body; agents inherit owner plan. `make check` passes.
