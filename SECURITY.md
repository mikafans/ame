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
- The `ame_token` session cookie is **not** `HttpOnly` — the frontend reads it client-side. Until that is moved server-side, deployments must rely on strict CSP and the absence of third-party scripts to mitigate XSS exfiltration.
- API tokens (`/v1/auth/login` returns one; `/v1/agents/register` mints one) are Argon2-verified on every request. This is intentional but costly; do not deploy ame to untrusted networks without a rate limiter / WAF in front.

### Authorization

- `POST /v1/agents/register` is **unauthenticated by design** for self-onboarding MCP clients. Any public deployment must front it with an invite system or rate limit, or the endpoint becomes an open API-key faucet.
- Role gating happens in route handlers, not in middleware. Audit new routes carefully.

### Operational foot-guns

- Setting `DEMO_MODE=1` in the API process disables authentication entirely and treats every request as an Instructor. This is for local demos only — never set it in production. Future versions will gate this behind a compile-time feature flag.
- The default CORS policy allows any origin. Override before production.
- No rate limiting is built in. Front the API with one (e.g. nginx `limit_req`, cloudflared, Envoy).

### Data

- All passwords are Argon2-hashed (default params).
- API token *ids* are stored in plaintext (UUID v7); the *secret* portion is Argon2-hashed.
- The database does not encrypt data at rest beyond what Postgres provides.

If you find a behavior that contradicts this document, please report it as a security issue.
