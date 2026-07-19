# AME roadmap

AME is rebuilding toward a stable 0.3 agent-first learning loop. The release
is deliberately centered on one product story: a learner gives an external
agent a topic and an email identifier, the agent creates a durable journey,
and the learner can study, practice, examine, deepen weak topics, and return to
the same progress from the browser or another authorized client.

## 0.3 — agent-first learning loop

The authoritative design, user stories, contracts, and TDD matrix are in
[`plans/2026-07-19-agent-first-learning-rework.md`](plans/2026-07-19-agent-first-learning-rework.md).

The release gate is C1–C6:

- C1: bootstrap or resume a learner journey from one prompt and an idempotency key.
- C2: compose versioned, reviewed questions into practice or graded assessments.
- C3: complete resumable attempts with server-side grading and pending manual review.
- C4: attach source-backed deep dives to evidence without mutating mastery silently.
- C5: persist activity history, attempts, evidence, mastery, recommendations, and streaks.
- C6: run the complete Postgres + Valkey + API + web + containerized Caddy origin.

The current implementation already includes the unified `/api/v1` learner API,
`/public/v1` onboarding API, static Caddy delivery for public contracts,
prompt-grounded five-step first learning packages, versioned assessments,
reviewed agent-grounded explanation/worked-example content, learner-safe
results, deep dives, progress reads, streak history, and browser coverage.
Remaining release work is certification and review evidence, not a compatibility
migration.

## 0.4 — retention and source-grounded learning

After 0.3 proves the first loop, add spaced retrieval, source import and
licensing checks, learner notes, alternate explanations, richer submissions,
and journey-history export/import. Each addition needs a real user story,
domain contract, happy/evil/edge tests, and local-stack or browser evidence.

## Later, only after the contracts stabilize

Reusable interactive activities, diagrams, simulations, collaborative
self-host features, and optional hosted identity can follow. Email delivery,
OAuth, agent accounts/scopes, payments, rankings, public social feeds, broad
media hosting, and PG-as-Valkey replacement are not 0.3 requirements.
