# Security Audit — AME API (2026-06-03)

**Scope:** "If I publish this online, does it stand — no hacking, no data
intrusion/leak, and above all no admin-hijack?"
**Method:** read of the auth, authz, admin, rate-limit, CORS and error paths
(not theory — file:line references below).
**Version audited:** `v0.1.0`, branch `feat/admin-api`.

## Verdict

**Solid foundation; safe to publish after two small HIGH fixes.** No path found
to data theft, SQL injection, secret leakage, or self-escalation to admin. The
two HIGH items below bear directly on rogue-admin recovery and brute-force/DoS,
and both are a few lines.

---

## Verified strong

| Area | Evidence |
|------|----------|
| Token auth | 192-bit random secret, SHA-256 at rest, constant-time compare, legacy Argon2 rejected → re-login (`auth/token.rs`) |
| Passwords | Argon2id; login timing equalized with dummy hash; deactivated rejected (`http/auth.rs:193`) |
| No self-escalation | `register` accepts only `role: "user"` (`auth.rs:99`); admin-scoped tokens mintable only by admin role (`me.rs:211`) |
| Admin authz | type-level `RequireScope<AdminScope>` on every admin route (`auth/scope.rs:47`, `admin.rs:887`) |
| Admin self-protection | self-demote / self-disable / last-active-admin lockout blocked + audited (`admin.rs:223-287`) |
| SQL injection | all queries use `bind`/`push_bind`, incl. ILIKE patterns |
| Secret leakage | no `token_hash`/`password_hash` in any response; internal errors → generic message + `request_id`, detail logged server-side (`error.rs:135`) |
| Cookies / CORS | HttpOnly, SameSite=Lax, Secure in prod; credentials enabled only with explicit origins (`http/mod.rs:79`) |
| Rate limiting | per-owner tiered + per-IP for unauthenticated — login/register covered (`ratelimit.rs:197`) |
| Audit trail | append-only `tb_audit_log` row on every state-changing admin action |

---

## Findings

### 🔴 HIGH 1 — Demotion does not revoke the demoted admin's live tokens
**Where:** `api/src/http/admin.rs:366-396` (role path).
**Issue:** admin power rides on the **token's** scope (fixed at login).
The demote path only runs `UPDATE tb_users SET role`; `invalidate_user_tokens`
clears just the 30s cache, and on re-read scopes still come from the unchanged
token row. A demoted (or compromised-then-demoted) admin **keeps `Scope::Admin`
on every existing token until expiry — up to 30 days.** The *disable* path, by
contrast, revokes tokens (`admin.rs:327`).
**Impact:** you cannot fully de-power a rogue admin by demoting them. Direct hit
on the admin-hijack threat.
**Fix:** on role change away from `admin`, also
`UPDATE tb_api_tokens SET revoked_at=now() WHERE user_id=$1 AND revoked_at IS NULL`,
then `invalidate_user_tokens`. Re-login re-issues with `scopes_for_role("user")`.
**Test:** demote an admin with an active admin token → that token now 401s.

### 🔴 HIGH 2 — X-Forwarded-For spoofing bypasses per-IP rate limiting
**Where:** `api/src/ratelimit.rs:217-221` (`get_client_ip`).
**Issue:** uses the **leftmost** XFF value, which is client-controlled. An
attacker rotates `X-Forwarded-For` per request → fresh limiter bucket each time →
**uncapped login brute-force and public DoS.** Safe only if a trusted proxy
overwrites XFF; k8s ingress typically *appends*, leaving the leftmost spoofable.
**Fix:** resolve client IP from a trusted hop — rightmost-after-N-trusted-proxies
or the ingress real-IP header — with the trusted-proxy count configurable
(`AME_TRUSTED_PROXIES`). Fall back to `ConnectInfo` when not proxied.
**Test:** two requests with different spoofed XFF but same socket share a bucket.

### 🟠 MEDIUM 3 — Global token-revoke must invalidate the 30s cache
**Where:** Feature #1 plan (`docs/plans/2026-06-03-admin-token-audit.md`).
Existing revoke/rotate call `invalidate_token`; the new admin global revoke must
too, or a "revoked" key keeps working ~30s — wrong for a panic button. Folded
into the token-audit plan's checklist.

### 🟠 MEDIUM 4 — No defense-in-depth on the `/v1/admin` prefix
Admin protection is per-handler; a future route that forgets `RequireScope` would
be silently unprotected. Add a router-level guard on the admin sub-router **and**
a test asserting every `/v1/admin/*` path 403s without admin scope. → v1.0
hardening pass.

### 🟡 LOW
- **Password policy / lockout:** ≥8 chars, no complexity/breach check; rate limit
  is per-IP, so distributed attempts on one account aren't slowed — consider
  per-account failed-login backoff.
- **Email canonicalization:** `register` binds raw `body.email` (`auth.rs:123`)
  while login is exact-match — confirm the write uses the canonical form from
  `20260603000000_email_canonical.sql`, else case/dot variants split accounts.
- **Security headers:** app sets none (HSTS / X-Frame-Options /
  X-Content-Type-Options / CSP); fine only if the ingress adds them — verify.

### ℹ️ INFO
- `last_used_at` does a `tokio::spawn` + UPDATE on every authenticated request
  (`extractor.rs:162`) — perf, not security; batch/sample later (ties to the
  data-retention plan).

---

## Remediation plan

| # | Severity | Fix | When |
|---|----------|-----|------|
| 1 | HIGH | Revoke tokens on admin demotion | **before public launch** |
| 2 | HIGH | Trusted client-IP resolution for rate limiting | **before public launch** |
| 3 | MEDIUM | Cache-invalidate on global token revoke | with Feature #1 |
| 4 | MEDIUM | Router-level admin guard + 403 sweep test | v1.0 hardening |
| LOW/INFO | — | password policy, email canon check, headers, last_used batching | opportunistic |

Dispatch lines for #1 and #2: `docs/plans/2026-06-03-security-fixes-tasks.jsonl`.
