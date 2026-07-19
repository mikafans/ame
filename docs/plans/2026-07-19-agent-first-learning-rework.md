# AME 0.3 — Agent-first learning platform

Status: design certification pending implementation

This document is the canonical plan for the clean-slate AME 0.3 redesign. It
defines the product promise, learner stories, domain model, API shape, TDD
gates, implementation order, and release certification. It supersedes earlier
plans and story sets for this rework.

The complete learning loop below is a future implementation target. Until the
certification gates pass, no existing route, table, fixture, or screen is
treated as part of the 0.3 contract.

## Product promise

AME is a self-hostable learning system that an external agent can operate for a
learner.

The learner says, using any topic they choose:

> There is an AME platform. Use it to help me learn the topic I name.

The agent then uses AME to create and operate a structured learning journey.
AME owns the durable learning state: goals, objectives, topics, explanations,
questions, assessments, attempts, evidence, deep dives, recommendations,
progress, and streaks. The browser is the learner workspace; an external agent
is an authorized client of the same API.

Agent-first does not mean agent-only. A learner must be able to inspect,
understand, continue, and challenge the work in the browser.

## Non-negotiable design decisions

### One resource model

There is no product-level agent account, agent-owned learner copy, agent scope,
agent token, run door, or second resource API. The web app and external agents
use the same learner-owned resources and service contracts.

Request authentication, rate limiting, audit metadata, and administrative roles
remain platform safeguards. They are not a separate learning identity model.

An external client may create or resume a learner through the configured
self-host registration mode. After authentication, it operates only on the
learner subject attached to that session. A second learner cannot be reached by
changing an identifier in a request.

### Full learning loop

The redesign restores and formalizes these capabilities rather than reducing
AME to onboarding:

```text
intent
  -> goal and journey
  -> objectives and topics
  -> explanation and worked example
  -> practice assessment
  -> attempt and evidence
  -> results and weak-topic interpretation
  -> deep dive or application activity
  -> next recommendation
  -> progress, mastery, and streak history
```

Questions are the first dependable activity primitive. The activity model must
remain extensible for explanations, projects, reflection, code, and future
media without making every activity a disguised question.

### Clean-slate schema and API

AME 0.3 may replace the schema, API contracts, and frontend architecture. No
migration, compatibility endpoint, dual read, backfill, or legacy adapter is
part of this rework. Local validation uses a clean PostgreSQL volume.

This is a reset boundary for the product release, not permission to destroy a
user's environment implicitly. Reset commands must be explicit in the
self-hosting documentation.

### Trait-contract-first and TDD

Every capability follows this order:

1. define domain types, repository/service traits, state transitions, and error
   semantics;
2. write behavior tests against an in-memory fake;
3. implement the PostgreSQL adapter and run the same contract suite against it;
4. expose the behavior through the API;
5. verify the browser journey and the real local stack.

No task is complete because a handler returns a plausible JSON response. A
behavior is complete only when its happy, evil, and edge cases pass at the
trait, PostgreSQL, HTTP, and browser boundaries that apply to it.

## Learner stories and certification gates

The exact semantic fixtures live in
[`2026-07-19-agent-first-learning-story-fixtures.json`](./2026-07-19-agent-first-learning-story-fixtures.json).
The stories below are the release contract, not claims about the current
worktree. Each story needs API trace, database assertions, browser evidence
where applicable, and happy/evil/edge test results.

## Release roadmap

### AME 0.3 — The certifiable agent-first learning loop

0.3 ships one complete vertical product, not a collection of disconnected
endpoints. A fresh self-host deployment must let an external agent help a
learner start, learn, practice, understand results, continue, and return later.

#### Included in 0.3

- one unified learner-owned API for the browser and external agents;
- self-host registration with explicit `open`, `password`, or `invite` mode;
- email-as-identifier behavior for the no-SMTP local mode, with safe collision
  handling and documented recovery limits;
- versioned template registry with the first three templates:
  `understand-a-subject`, `prepare-for-an-exam`, and `build-a-project`;
- natural-language intent capture and idempotent journey bootstrap;
- learner-visible goal, objectives, effort, success criteria, explanation, and
  worked example before the first assessment;
- learner-scoped topics/skills and objective links;
- question bank with multiple choice, true/false, short answer, essay, and code
  question types;
- immutable question and assessment versions;
- practice and graded assessment modes;
- resumable activity sessions and attempts;
- server-side objective grading, pending-review states for essay/code work, and
  correct multiple-choice option remapping;
- objective-linked evidence, results, weak-topic interpretation, and one
  primary next recommendation;
- grounded deep dives with source/provenance, review state, example, and a
  small application task;
- learner timeline, objective mastery snapshots, and explicitly defined
  qualifying streak events;
- provider generation states, retries, failure recovery, and no false progress
  on provider failure;
- request authentication, learner ownership enforcement, rate limiting,
  idempotency, audit records, and safe error semantics;
- browser surfaces for intent, preview, activity, assessment, results,
  deep-dive, recommendation, progress, resume, empty, and failure states;
- generated OpenAPI, `skill.json`, `llms.txt`, frontend client/schema, and human
  self-host documentation from the actual 0.3 contract;
- clean local-stack certification with Postgres, Valkey, API, web, and
  containerized Caddy.

#### 0.3 release gate

All C1-C6 stories must pass on a clean local stack. Each must have trait/fake,
PostgreSQL, HTTP, browser, and integration evidence where applicable. A
hardcoded topic fixture, mock agent, or isolated happy-path test cannot satisfy
the gate.

#### Explicitly not in 0.3

- email delivery, magic links, OAuth, or hosted identity;
- agent accounts, agent scopes, agent tokens, or a second agent API;
- marketplace discovery, payments, instructor economies, or course commerce;
- certificates, badges, rankings, or competitive leaderboards;
- public social feeds, generic forums, or unmoderated learner-to-learner chat;
- broad media authoring, video hosting, or an H5P-compatible runtime;
- production-grade code execution sandboxes;
- automatic mastery claims from generated content alone;
- cross-tenant sharing or public learner data;
- PG-as-Valkey replacement or a new cache abstraction without an evidence-based
  performance requirement.

### 0.4 — Retention and source-grounded learning

After 0.3 proves that learners complete and return to the first loop, add:

- spaced retrieval and review scheduling;
- source import from URLs, documents, and local files;
- citation extraction, source snapshots, licensing, and grounding checks;
- learner notes and annotations;
- “explain differently,” “make harder,” and “show another example” actions;
- richer project submissions and rubric-based review;
- export/import of journeys and learner history;
- improved recovery options for deployments that configure identity services.

### 0.5 — Reusable interactive learning

Once the content and evidence contracts are stable, add reusable interactive
activities inspired by community LMS patterns:

- diagrams, interactive presentations, simulations, and embedded exercises;
- reusable content libraries and template sharing within a self-host instance;
- author preview, validation checklists, draft/publish workflow, and content
  review queues;
- objective-specific discussions with moderation and learner controls;
- accessibility audits for every activity type.

### 0.6 — Groups and institutional workflows

Only after individual learning is reliable:

- cohorts, teams, and shared journeys;
- instructor or mentor review workflows;
- cohort progress analytics with strict privacy boundaries;
- scheduled programs, deadlines, and notifications;
- optional certificates or badges backed by explicit assessment policy.

These releases are conditional roadmap candidates, not commitments. Each new
capability requires a real user story, domain contract, fixture, happy/evil/edge
tests, and a local-stack or browser certification path before entering the
implementation queue.

### C1 — An agent bootstraps a learner journey

Given a learner's email identifier and a learner-selected topic, an external
agent discovers AME, creates or resumes the learner, selects a versioned
template, and creates a first journey.

The result contains a goal, observable objectives, an explanation, a worked
example, diagnostic/practice work, and one next action. Repeating the request
with the same idempotency identity returns the existing package. It does not
create an agent account, duplicate learner, duplicate journey, or premature
learning progress.

### C2 — An agent composes a grounded assessment

Given a journey objective, an external agent creates or selects versioned
questions and composes a practice or graded assessment. Questions support
multiple choice, true/false, short answer, essay, and code responses. Each item
has objective linkage, explanation/rationale, difficulty, source/provenance,
and review state.

The browser renders a readable assessment. Historical attempts remain
reproducible after question or assessment revision. Unreviewed or incomplete
generated content cannot silently appear as approved learning content.

### C3 — A learner completes practice and an exam

The learner can practice, resume an interrupted attempt, and take a distinct
graded exam. AME scores objectively gradable items server-side, represents
essay/code review as pending when needed, and returns item feedback, objective
evidence, a performance summary, and one primary next recommendation.

Multiple-choice option shuffling must not change grading or correct-answer
display. Practice performance must not masquerade as an exam pass.

### C4 — An agent deepens a weak topic

After results identify a weak objective, the learner asks the agent for a deep
explanation. The agent creates or retrieves a deep dive linked to the evidence,
including an example, caveats, sources, review status, and a small application
task.

The learner reads and completes it in the same journey. The deep dive improves
understanding but does not silently change mastery, score, or exam status.

### C5 — Progress survives sessions and clients

After several completed activities, the learner returns through the browser or
another authorized external agent. Both clients see the same activity history,
attempts, objective evidence, mastery state, recommendations, and streak.

Restart does not erase or duplicate progress. Streak semantics are explicit:
the qualifying event, learner timezone, calendar-day boundary, and duplicate
event behavior are stored and tested. Abandoned or failed work cannot falsely
advance mastery.

### C6 — A self-host operator runs one coherent origin

From a clean volume, the documented Postgres + Valkey + API + web + Caddy stack
starts and exposes one public origin. Human self-host guidance, `llms.txt`,
`skill.json`, and `openapi.yaml` agree with the actual API. Registration and C1
work without SMTP. API/web restart preserves learner state.

The recommended stack uses containerized Caddy. A system-level Caddy is neither
required nor part of this story.

## Domain model

```text
Learner
  └─ LearningGoal
       └─ LearningJourney (template version)
            ├─ Objective[] ── Topic/Skill[]
            ├─ Activity[]
            │    ├─ Explanation
            │    ├─ Assessment (practice | graded)
            │    │    └─ QuestionVersion[]
            │    ├─ DeepDive
            │    └─ Application/Reflection
            ├─ ActivitySession[]
            │    └─ Attempt[] ── Evidence[]
            ├─ MasterySnapshot[]
            ├─ StreakEvent[]
            └─ Recommendation[]
```

Every generated resource carries template version, source/provider identity,
content version, review state, and creator/request metadata. Generated content
is not learner evidence until the learner performs the associated activity.

The clean baseline must cover:

- learners and resumable sessions;
- goals, journeys, template versions, objectives, topics, and skills;
- typed activities and lifecycle state;
- assessments, sections, question versions, options, and provenance;
- activity sessions, attempts, answers, grading, and evidence;
- deep dives and application tasks;
- recommendations, mastery snapshots, streak events, and timeline queries;
- idempotency, audit, settings, and dependency/provider status.

The schema specification at
[`2026-07-19-learning-baseline-schema.md`](../specs/2026-07-19-learning-baseline-schema.md)
is the fresh 0.3 baseline. There is no compatibility migration from the
retired pre-0.3 model; a new self-host stack applies this baseline on startup.

## Unified API direction

The API is organized around learner resources, not callers:

```text
POST   /public/v1/auth/register
POST   /public/v1/auth/login
POST   /public/v1/onboarding/preview
POST   /public/v1/onboarding/start
GET    /api/v1/me
GET    /api/v1/learning/journeys
GET    /api/v1/learning/journeys/:id
POST   /api/v1/learning/journeys/:id/activities/:activity_id/start
GET    /api/v1/learning/sessions/:id
POST   /api/v1/learning/sessions/:id/finish
POST   /api/v1/questions
POST   /api/v1/assessments
POST   /api/v1/assessments/:id/attempts
POST   /api/v1/attempts/:id/answers
POST   /api/v1/attempts/:id/finish
GET    /api/v1/progress/:journey_id/objectives/:objective_id
POST   /api/v1/deep-dives

GET    /public/llms.txt
GET    /public/skill.json
GET    /public/openapi.yaml
GET    /healthz
GET    /readyz
GET    /metrics
```

These are the 0.3 route names. TDD contracts define their semantics, and the
generated OpenAPI, skill manifest, and `llms.txt` are checked against the same
source. The same endpoints serve the browser and an external agent.

The API must provide explicit errors for expired sessions, email collisions,
provider unavailability, stale versions, invalid transitions, forbidden
learner ownership, duplicate submissions, and incomplete review. It must never
convert an authorization failure into an empty learner state.

## Behavior-driven TDD matrix

Before each implementation slice, add tests in three groups.

### Happy paths

- new onboarding creates one learner journey and first learning moment;
- retry returns the same bootstrap result;
- question versions compose into practice and graded assessments;
- practice and exam attempts finish with correct scoring;
- weak-objective evidence produces a deep-dive recommendation;
- completed activity updates timeline, mastery, and streak once;
- browser and external client observe the same state;
- clean five-service stack serves the documented public origin.

### Evil paths

- another learner's identifiers cannot be substituted into a request;
- expired or logged-out sessions cannot read or write learning state;
- stale question/assessment versions cannot mutate historical attempts;
- clients cannot forge scores, mastery, streaks, or recommendation provenance;
- unreviewed content cannot be presented as approved;
- an email collision cannot silently claim an existing account;
- repeated answers, onboarding, and finish requests are idempotent;
- provider or Valkey failure cannot create false progress.

### Edge paths

- bootstrap fails after every durable write and safely resumes;
- an attempt is abandoned and resumed;
- an assessment mixes automatically and manually reviewed item types;
- an assessment has no objectively gradable item;
- a question is revised after an old attempt;
- no evidence, tied objectives, and equal recommendations are deterministic;
- streak evaluation crosses the configured learner timezone/day boundary;
- direct and Caddy public-document responses remain semantically identical.

The contract test suite must run against both an in-memory fake and PostgreSQL.
HTTP tests then verify authentication, idempotency, error mapping, and response
shape. Browser tests verify the learner-visible story. The local-stack tests
are the final integration gate, not a replacement for the lower-level suites.

## Implementation order

### T00 — Freeze the design contract

Approve this document and the story fixture. Record any change to identity,
activity types, evidence, streak semantics, registration, or public origin here
before code changes.

### T01 — Build behavior contracts first

Create domain traits and happy/evil/edge tests for learner onboarding, goals,
journeys, activities, assessments, attempts, grading, deep dives, progress,
streaks, recommendations, and idempotency. Use deterministic fixtures and no
model/provider dependency in the core contract suite.

### T02 — Replace the clean PostgreSQL baseline

Implement the schema required by the domain contract, including question
versions, assessment attempts/evidence, deep dives, mastery, streaks,
recommendations, provenance, and audit. Run the same repository contract tests
against PostgreSQL. Do not migrate old data.

### T03 — Implement self-host identity and onboarding

Implement the configured local registration mode, email collision behavior,
resumable session, intent capture, bootstrap idempotency, and provider failure
state. No SMTP dependency is allowed for the first journey.

### T04 — Implement the learning domain services

Implement journey/template selection, explanation/example activities, question
and assessment composition, versioning, activity sessions, answer persistence,
grading, evidence, and result calculation behind the approved traits.

### T05 — Implement the deep-dive and progress loop

Implement weak-topic interpretation, grounded deep dives, application tasks,
recommendation provenance, mastery snapshots, timeline, and explicit streak
semantics. Generated content must never fabricate learner evidence.

### T06 — Expose the unified API and regenerate contracts

Expose the learner resources to the browser and external agents through one API.
Regenerate OpenAPI, `skill.json`, `llms.txt`, and the frontend client from the
actual API. Remove agent-management, run-door, scope, and compatibility
contracts that do not belong to the new design.

### T07 — Rebuild the learner frontend

Implement the intent entry, journey preview, explanation/example, practice,
exam, result, deep-dive, recommendation, progress, streak, resume, failure,
and empty states. The browser must show learner language and useful context,
not raw API payloads.

### T08 — Certify the self-host origin

Run C1-C6 through the documented Postgres + Valkey + API + web + Caddy stack on
clean volumes. Verify public docs, health/readiness, restart/resume, and no
SMTP/system-Caddy dependency.

### T09 — Remove obsolete surface and release

Delete retired schema/API/frontend behavior, update checks and docs, attach the
evidence bundle, and publish only when every certification is passing.

## Certification evidence

For each C1-C6, attach:

- exact stack-start command and clean-volume boundary;
- redacted API requests/responses or browser actions;
- database assertions for durable state and ownership;
- happy/evil/edge test names and results;
- screenshots or rendered-route evidence where learner UX matters;
- expected and observed behavior;
- known limitations and an explicit pass/fail decision.

No mock agent, isolated unit test, or hardcoded topic fixture can certify a
story by itself. Topic fixtures may make the scenario deterministic, but the
implementation must exercise the generic journey, question, assessment,
deep-dive, progress, and public-contract capabilities.

## Definition of done

AME 0.3 is complete when a learner can ask an external agent for help, receive
a durable and understandable journey, learn through explanations and
question-backed practice/exams, receive evidence-based next steps, deepen weak
topics, and see progress over time in the browser or through another authorized
client.

The product is not complete when only onboarding works. C1-C6 must be simulated
against the real recommended local stack, with all required TDD layers passing,
and the evidence must be attached to the implementation handoff. No migration
or compatibility layer is required.
