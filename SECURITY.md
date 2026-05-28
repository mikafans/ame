# Security Policy

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security problems.

Email **azusa146@gmail.com** with:

- A description of the issue and its impact.
- Reproduction steps or a proof-of-concept.
- The commit SHA / version you tested against.

We aim to acknowledge reports within 7 days and to ship a fix or mitigation within 30 days for high-severity issues. There is no bug bounty.

## Supported versions

ame is pre-1.0. Only `main` and the most recent tagged release receive security fixes.

## Threat model and known limitations

ame is designed for trusted instructor/learner cohorts behind authenticated sign-up. The following are **known limitations** that operators should account for before any public deployment:

### Authentication

- Email + password only. No MFA, no SSO/SAML, no password reset flow yet.
- The `ame_token` session cookie is **not** `HttpOnly` — the frontend reads it client-side. `Secure` is set in production and a strict CSP is shipped in `web/next.config.mjs`; even so, an XSS payload could exfiltrate the token. Auth will move to a server-only cookie in a later phase.
- API tokens are server-generated 192-bit random secrets, stored as `sha256(secret)` and compared in constant time. Password hashing (login flow) still uses Argon2.

### Authorization

- `POST /v1/agents/register` is gated by a shared `AME_AGENT_ACCESS_CODE` secret. When the env var is unset, registration is disabled entirely. Operators must opt in by setting it (e.g. `openssl rand -hex 32`).
- `/v1/auth/login`, `/v1/auth/register`, and `/v1/agents/register` are rate-limited per IP (`tower_governor`). Defaults: burst 10, refill 1/2s. Tune via `AME_RATELIMIT_BURST` / `AME_RATELIMIT_PERIOD_SECS`.
- Role gating happens in route handlers, not in middleware. Audit new routes carefully.

### Operational foot-guns

- The previous `DEMO_MODE=1` runtime auth bypass has been removed entirely.
- Default CORS policy now allows only `http://localhost:3000`. Override via `AME_CORS_ORIGINS` (comma-separated). Setting it to `*` re-enables permissive CORS.
- Rate limiting is built in for the public auth/registration endpoints only. Front the API with an additional limiter (nginx `limit_req`, cloudflared, Envoy) if you expose any authenticated endpoint to untrusted traffic.

### Data

- All passwords are Argon2-hashed (default params).
- API token *ids* are stored in plaintext (UUID v7); the *secret* portion is stored as `sha256(secret)` and compared in constant time.
- The database does not encrypt data at rest beyond what Postgres provides.

If you find a behavior that contradicts this document, please report it as a security issue.
