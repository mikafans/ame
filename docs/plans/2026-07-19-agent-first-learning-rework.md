# AME — Agent-first learning platform redesign

Status: design for review

This is the consolidated product, domain, API, schema, and validation plan for
the next AME model. It replaces the current capability-first landing flow with
an intent-first learning journey.

The plan is intentionally extensive. No API, database, or frontend
implementation should start until the domain vocabulary, user stories,
authorization model, and simulation gates in this document are accepted.

## Clean-slate redesign decision

This redesign may replace the current schema, API contracts, and frontend
architecture completely. There is no migration requirement and no
backward-compatibility requirement for the pre-redesign model.

The implementation target is a clean database created from the new baseline.
Existing development data may be discarded. Do not spend design effort on
compatibility columns, transitional endpoints, dual-read logic, backfills, or
legacy UI adapters unless a later release explicitly asks for them.

This does not authorize destructive commands against a user environment during
implementation. It means the new baseline and validation must use a clean
volume.

## 1. Product decision

AME should make one promise:

> Tell AME what you want to learn. It prepares a useful first learning session
> and keeps improving the next one with you.

AME is not a generic course marketplace and not a chat wrapper. It is a
self-hostable, structured learning system where agents can create and operate
learning journeys, while the learner retains visibility and control.

The first visible flow should be:

```text
I would like to learn music theory
+ email identifier
        ↓
local learner account and session
        ↓
agent interprets the goal
        ↓
goal + objectives + starter activities
        ↓
short first assessment
        ↓
feedback + next action
```

The user should not manually create a password, agent, question bank,
assessment, or study plan before receiving value.

AME currently has no email delivery service. The self-host MVP therefore treats
email as an account identifier/contact label, not proof of ownership. SMTP,
magic links, OAuth, and hosted identity are optional future modes, not
dependencies of the first learning journey.

## 2. What AME already did well

The redesign can preserve proven concepts and behavior, but it is not required
to preserve current table layouts or wire contracts.

### 2.1 A real assessment engine exists

AME already has:

- one unified assessment entity with `practice` and `graded` modes;
- ordered sections and assessment items;
- deadlines, points, passing scores, result visibility, and rating effects;
- server-side session and attempt persistence;
- answer grading and result aggregation;
- question versioning and MC option-order preservation.

This is the core learning primitive many agent products lack.

### 2.2 The question model is already useful

Questions support MC, true/false, short answer, essay, and code. Each question
has a prompt, typed kind, JSON payload, explanation, points, status, source,
tags, rating, attempt count, and version history.

That is enough to generate a small, varied starter assessment, but an
assessment alone is not enough to attract or retain a learner. It proves the
engine works; it does not yet deliver the feeling of being taught.

### 2.2.1 The first value must be a learning moment

The first generated package must not be “here is a quiz.” It should be a small
guided learning moment:

```text
intent and level
  → promise and objectives
  → short explanation or worked example
  → diagnostic/practice questions
  → immediate feedback
  → small application task
  → progress interpretation
  → one next step
```

The assessment is the diagnostic and evidence component inside that moment.
For subjects such as music theory, the experience may also need notation,
audio, diagrams, examples, or a tiny composition task. A generic set of MC
questions will not demonstrate the product promise by itself.

The first vertical slice must therefore answer three learner questions:

- `What am I going to be able to do?`
- `Can I feel myself learning it right now?`
- `What should I do next, and why?`

The agent should generate a compact journey, not a large curriculum. However,
the compact journey must contain at least one explanation/example and one
application or reflection step in addition to assessment items.

### 2.3 Owner isolation and actor attribution are real foundations

The identity migration already distinguishes human and agent identities.
Learning resources carry owner and creator/actor information. Sessions and
attempts distinguish the learner owner from the acting identity. Activity logs
record agent-related actions.

This gives AME a strong basis for the model:

```text
actor   = human | agent | system/bootstrap
subject = learner owner
```

The redesign should clarify this model. It may replace the current identity
tables and resource ownership columns if a cleaner actor/subject design is
needed.

### 2.4 The agent surface is already discoverable and unified

AME already has:

- REST resources for assessments, questions, sessions, attempts, plans, and
  stats;
- the `/v1/agents/run` dispatcher for agent actions;
- scoped agent capabilities;
- generated `skill.json`;
- `llms.txt` contract documentation;
- OpenAPI generation and frontend client generation.

The correct direction is to make the agent more capable within one coherent new
API. The new API does not need to preserve current endpoint shapes if a better
unified contract is designed.

### 2.5 Self-host operations are concrete

The recommended stack is already operationally clear:

```text
Postgres + Valkey + API + web + Caddy
```

The local stack has health checks, embedded migrations, public document
delivery, and a working Caddy entry point. This is a better foundation for
testing the real product story than a hosted-only design.

## 3. Product lessons to adopt

AME should borrow learning structure from established platforms without
becoming a clone.

Udemy makes course decisions legible before enrollment: learning outcomes,
prerequisites, curriculum preview, instructor context, reviews, and duration.
Its practice tests distinguish flexible practice mode from timed exam mode and
let learners review answers by knowledge area.

- <https://support.udemy.com/hc/en-us/articles/229231027-How-to-preview-and-compare-courses>
- <https://support.udemy.com/hc/en-us/articles/1500006547442-How-to-create-practice-tests-or-practice-test-courses>

Coursera emphasizes explicit skills, alignment between objectives, instruction,
and assessment, visible level and duration, and hands-on activities.

- <https://blog.coursera.org/courseras-commitment-to-skill-development/>
- <https://www.coursera.org/browse/>

AME should adopt these principles:

1. Every generated journey explains its goal, objectives, effort, and success
   criteria.
2. Every assessment is connected to an objective.
3. Practice and exam behavior stay distinct.
4. The learner can preview what is about to happen.
5. Feedback identifies the next action, not only the score.
6. Activities can grow beyond questions later, but questions remain the first
   reliable activity type.

AME should not initially build marketplace discovery, instructor economies,
   payments, certificates, peer discussion, or a large video curriculum.

## 4. Exact user stories

These stories are intentionally concrete. Each becomes a required local-stack
simulation after implementation.

### Story A — New learner intent to first session

As a new learner, I enter:

```text
I would like to learn music theory
```

and an email identifier, so AME creates a local learner session and prepares a
small first assessment without asking me to design the curriculum.

Expected result:

- one learner account is created;
- one authenticated browser session is established;
- the intent is persisted;
- the agent/system creates a goal, objectives, starter questions, and one
  practice assessment;
- the learner sees a plan preview and can start the first session;
- a repeated request does not duplicate the account or starter package.

Required simulation:

1. Start the clean recommended Compose stack.
2. Submit the landing form through the browser.
3. Verify the registration/API response and cookie/session.
4. Inspect the database for owner, intent, questions, assessment, and session.
5. Start and complete the assessment through the browser.
6. Repeat the original request and prove idempotency.

### Story B — Learner understands the generated journey

As a learner, before starting, I can see what AME prepared and why.

Expected result:

- the goal is stated in learner language;
- objectives use observable verbs such as explain, identify, apply, or compare;
- the first assessment has an estimated time and purpose;
- each question is connected to at least one objective or skill;
- the learner can accept or adjust the direction before starting.

Required simulation:

1. Submit a second intent with a different topic and level.
2. Capture the plan/assessment response and rendered page.
3. Verify objective, topic, difficulty, duration, and question rationale fields.
4. Adjust the goal and prove the generated package changes without corrupting
   the original attempt history.

### Story C — Results lead to the next action

As a learner, after the first session I can understand what I know, what needs
practice, and what to do next.

Expected result:

- results contain score and item-level feedback;
- weak objectives/topics are identified;
- the explanation is understandable without reading raw JSON;
- exactly one primary next action is recommended;
- the next action can be started without rebuilding the account or plan.

Required simulation:

1. Complete a deliberately mixed-result assessment.
2. Verify result correctness, including shuffled MC options.
3. Verify weak-skill calculation and recommendation provenance.
4. Start the recommended next activity.
5. Confirm the learner timeline contains both sessions and attempts.

### Story D — Agent as learner co-pilot

As a learner using an external agent, I can ask it to create, inspect, revise,
explain, and recommend learning material through the unified AME API.

Expected result:

- the agent acts on behalf of exactly one learner owner;
- the same assessment/question/session/plan resources are used;
- the learner can distinguish agent-created content from human-created content;
- the activity log records actor, subject, operation, and affected resource;
- revoking the agent prevents further writes while preserving learner history.

Required simulation:

1. Create an agent through the existing owner flow.
2. Use the public agent contract to create questions and an assessment.
3. Read the generated resources through direct reads and the run door.
4. Attempt cross-owner reads and writes and verify rejection.
5. Revoke the agent and verify that old content remains readable to the owner
   while new agent writes fail.

### Story E — Self-host operator and public contracts

As a self-host operator, I can start the recommended stack and expose one
coherent learner/API/public-document origin without SMTP.

Expected result:

- Postgres, Valkey, API, web, and Caddy become healthy/started;
- `/`, `/self-hosting`, `/llms.txt`, `/skill.json`, and `/openapi.yaml` work
  through Caddy;
- direct API compatibility routes return semantically identical public docs;
- registration and first-session bootstrap work with no mail configuration;
- `/healthz` and `/readyz` accurately describe dependencies.

Required simulation:

1. Start from clean named volumes.
2. Run the documented Compose commands only.
3. Verify every public route and response content type.
4. Perform Story A through the Caddy origin.
5. Restart API/web and confirm the learner can resume.
6. Run backup/upgrade smoke checks without destroying normal data.

## 5. Domain model

The stable learning model should be:

```text
LearningGoal
  └─ LearningJourney
       ├─ Objective[]
       │    └─ Skill/topic references
       ├─ Activity[]
       │    ├─ Assessment
       │    ├─ Question practice
       │    ├─ Explanation/deep dive
       │    └─ Future project/resource types
       ├─ Attempt[]
       ├─ MasteryEvidence[]
       └─ NextRecommendation
```

### 5.1 Concepts

- **Learning goal:** the learner’s natural-language intent plus normalized
  statement.
- **Learning journey:** durable owner-scoped container for the goal, objectives,
  activities, progress, and recommendations.
- **Objective:** an observable outcome, not just a tag. It should include a
  stable identifier, verb, subject, success criterion, and order.
- **Skill/topic:** reusable taxonomy item used for discovery and mastery.
- **Activity:** a learner action. Assessment is the first activity type.
- **Mastery evidence:** derived from attempts and feedback, never a raw agent
  opinion without supporting attempts.
- **Recommendation:** a proposed next action with reason, source evidence, and
  expiry/version.

### 5.2 Current model assessment

Existing metadata is useful reference material for the first vertical slice:

- assessment title, description, objectives, mode, duration, points, sections;
- question kind, payload, explanation, source, points, tags, versions;
- session, attempt, result, rating, and deep-dive records;
- actor/owner identity and activity attribution.

It is not sufficient for the stable long-term model because AME lacks:

- a first-class goal/journey record;
- stable objectives distinct from free-text assessment arrays;
- prerequisites, level, estimated effort, and success criteria;
- typed activity/resource metadata beyond question payload JSON;
- durable mastery evidence and recommendation provenance;
- journey progress and completion state;
- content quality/provenance/review status.

### 5.3 Schema strategy

Do not attempt to model every future Udemy/Coursera feature now. Design a clean
new baseline with the smallest durable primitives in this order:

1. `learning_goal` / journey state associated with an owner;
2. normalized objective records or a versioned structured objective document;
3. journey-to-activity ordering and progress;
4. recommendation records with evidence and actor attribution;
5. mastery evidence derived from attempts.

The new schema may replace the current assessment/question/session/attempt
tables if their shapes do not fit the journey model. Do not create separate
agent-owned content tables: actor and subject belong in the new authorization
model. Keep Valkey in the recommended stack; deciding its internal role is
separate from the clean-slate schema decision.

## 6. API and agent redesign

### 6.1 Unified journey API

The new API may redesign existing routes and introduce a coherent journey API
where the domain requires it. The constraint is not “never add an endpoint”;
the constraint is “do not accumulate parallel APIs that expose the same
concept twice.” Route names and request/response shapes may change freely
before the new baseline ships.

#### `POST /v1/auth/register`

Current registration requires email, display name, password, and role. The new
self-host-first contract may replace that request entirely: it should accept an
initial learning intent and create or resume the local learner session in one
operation.

Conceptual request:

```json
{
  "email": "name@example.com",
  "name": "Ada",
  "intent": "I would like to learn music theory"
}
```

The exact credential policy must be explicit:

- `open`: local visitor can create a new learner session;
- `invite`: operator-issued invite is required;
- `password`: current credential flow is required.

Email alone must never silently claim an existing account. A collision must
follow the configured login/claim path.

The response should include user/session plus onboarding state and the first
destination, not only a generic auth object.

#### `GET /v1/me`

Return current goal/journey state, bootstrap status, first destination, and next
action so the browser can recover after refresh or restart.

#### Agent and learning resources

Design one unified surface. The final agent contract may replace
`/v1/agents/run` if a better capability model exists. Bootstrap may be an
internal application service bound to the newly created owner, or a first-class
scoped agent action; it must not be an unrestricted public agent faucet.

### 6.2 Authorization

Every action carries:

```text
actor   = human | agent | system/bootstrap
subject = learner owner
```

Rate limiting protects capacity and abuse surfaces. It does not replace
authentication, authorization, ownership, consent, or auditability.

Existing `owner_id`, `created_by`, `actor_id`, `agent_id`, and activity-log
semantics are reference material only. The new schema should choose one clear
actor/subject representation rather than carrying every historical name.

## 7. Agent capability model

The agent should gain capabilities in stages:

### Stage 1 — Bootstrap co-pilot

- interpret a learner intent;
- propose goal and objectives;
- create a small question set;
- compose a practice assessment;
- explain why each item exists;
- recommend the first next action.

### Stage 2 — Learning loop

- inspect attempts and objective evidence;
- identify weak skills;
- revise the journey;
- create targeted practice;
- request/publish deep dives;
- recommend review based on prior results.

### Stage 3 — Authoring and power workflows

- batch authoring;
- assessment maintenance;
- plan revision;
- export and reporting;
- operator-approved automation.

Every capability must have a scope, owner boundary, activity record, retry
semantics, and public contract entry. No capability is accepted only because an
LLM can call it successfully.

## 8. First-time agent templates

Templates are product presets, not prompt snippets. They give an agent a safe
starting structure while allowing the learner to speak naturally. The agent
recommends a template from the learner's intent and shows the resulting promise
before creating the journey. The learner may accept, adjust, or continue with a
custom goal.

The first release defines exactly three built-in templates:

1. `learn-a-subject` — understand a topic from an unknown or beginner level;
2. `exam-prep` — prepare for a named exam or target capability with practice
   and review;
3. `build-a-project` — learn by producing a concrete artifact, with milestones
   and an application task.

Each version defines a stable ID, display metadata, input fields, stages,
allowed activity types, output contract, quality rules, allowed capabilities,
retry/idempotency behavior, and learner-visible preview. The required inputs
are goal, current level, target/outcome, available time, and preferred style;
the implementation may make some optional only after T00 decides so.

The storage mechanism is an open decision until T02. The implementation must
choose one versioned source of truth—static structured definitions or a
database-backed registry—and must not duplicate templates across prompts,
frontend code, and API handlers.

## 9. Confirmed gaps and unresolved decisions

The following gaps are confirmed by repository inspection:

| Gap | Repository fact | Required resolution |
| --- | --- | --- |
| No first-class learning goal/journey | Current plans are planner output; no goal/journey entity was found in the current API/schema inventory | Add durable goal, journey, progress, and lifecycle state |
| Objectives are not durable learning objects | Assessment objectives are currently free-text arrays | Add stable objective records or one versioned structured objective document |
| No first-time agent template registry | No learning-template inventory or template API was found | Add the three versioned built-ins and one source of truth |
| Landing has no first-session loop | Current landing routes to auth, then the generic explore surface | Add intent capture, preview, first activity, result, and next action |
| Registration is not intent-first | Current registration requires email, name, password, and role | Define the self-host registration mode and bootstrap contract |
| No email delivery | No mail-sending service is part of the self-host design | Treat email as identifier/contact in MVP; do not promise verification or magic links |
| Activity model is assessment-centric | Durable learning primitives are questions, assessments, sessions, attempts, and deep dives | Add explanation/example/application semantics for the first learning moment |
| Mastery and recommendation evidence are incomplete | Attempts and ratings do not form an objective-evidence/recommendation model | Add evidence, recommendation reason, and next-action records |
| Agent API is not a complete learning loop | Current agent actions cover resource operations, not intent bootstrap through feedback | Redesign one unified capability/resource contract |
| Question content is partly untyped | Question payload is JSONB and objective linkage is not a stable contract | Define typed activity payloads and validation |
| Public OpenAPI is machine-readable only | `/openapi.yaml` is raw YAML with no human preview route | Decide whether human API reference is in scope; do not imply it exists |

Unresolved decisions are not permission to invent behavior:

- semantics of `open`, `invite`, and `password` registration without SMTP;
- static structured files versus database-backed template registry;
- internal system actor versus external agent for bootstrap;
- supported self-host model/provider configuration and no-provider behavior;
- mandatory activity types for each first-release template;
- controlled skill taxonomy versus learner-scoped topics;
- provenance and review state for generated content;
- whether the email identifier is required in every deployment mode.

### T00 working decisions for this rework

These are explicit implementation defaults, recorded here so they can be
reviewed and changed deliberately. They are not hidden assumptions:

| Decision | Rework default | Consequence |
| --- | --- | --- |
| Local registration | The recommended local profile is `open`; it accepts one email identifier and creates a learner session. `invite` and `password` remain explicit operator modes. | The landing flow has no password field in `open` mode. |
| Account collision | Email alone never claims an existing account. A collision returns an explicit existing-account state and requires the configured login/claim path. | No unsafe account takeover and no fake recovery path. |
| Recovery without mail | The MVP promises resume through the active browser session only. It does not promise email verification, magic links, or recovery mail. | A deployment needing durable recovery must configure another identity mode before release. |
| Built-in templates | Store the three built-in template definitions as versioned structured files in the repository, loaded by the API as one source of truth. | No prompt/frontend/API copies and no template migration is needed for the first baseline. |
| Bootstrap actor | Onboarding bootstrap runs as a scoped `system/bootstrap` actor for the newly created learner subject. External agents use the same journey service through their own actor identity. | Initial onboarding does not require minting an agent token first. |
| Model/provider | AME does not pretend to host a model. A configured provider/agent is required for generated content; absence or failure produces a resumable unavailable state. | The stack must expose provider configuration and test the no-provider path. |
| First activity set | All templates support explanation, example, diagnostic/practice, feedback, and next action. `exam-prep` additionally supports timed practice; `build-a-project` additionally supports an application milestone. | The activity union is small and typed before adding projects, courses, or media. |
| Skill taxonomy | The first journey may create learner-scoped skill/topic records from the normalized intent. A shared taxonomy is deferred until evidence shows reuse. | No invented global taxonomy blocks the first learning loop. |
| Generated content provenance | Every generated goal, objective, activity, and recommendation records source actor, template version, provider/run identity, and review/status state. | Unsupported or failed generation cannot appear as trusted progress. |
| Public documentation | Human self-host guidance and machine contracts remain separate canonical files, delivered directly by the frontend/public origin; raw OpenAPI remains machine-readable. | A human API explorer is not implied by `/openapi.yaml`. |

These defaults close T00 for implementation planning. Any change must update
this table, the story fixtures, and the affected acceptance checks together.

## 10. Real implementation task list

These IDs are the execution backlog. Dependencies are hard gates. A task is
complete only when its deliverable and acceptance checks are present in the
repository.

Current execution status: T00 and T01 are complete; T02 is in progress with
the built-in catalog and loader landed; T03 has not started.

Checkpoint policy: commit after each bounded task or reviewable vertical slice.
Do not combine template design, PostgreSQL schema replacement, API contracts,
and frontend work in one commit.

### T00 — Approve product invariants

Depends on: none.

Deliverable: reviewed decisions for the promise, first learning moment, three
templates, registration modes, actor/subject model, clean-slate policy, and
first-release activity types.

Acceptance: every unresolved decision above has an owner and decision; all five
stories remain testable; no endpoint, table, or UI implementation starts before
this gate.

### T01 — Create story fixtures and simulation contracts

Depends on: T00.

Deliverable: deterministic fixtures for Stories A–E, including music theory,
exam preparation, and project learning; API, browser, database, retry, and
cross-owner assertions.

Initial fixture source: [`2026-07-19-agent-first-learning-story-fixtures.json`](./2026-07-19-agent-first-learning-story-fixtures.json).
It is semantic by design; T00 must approve wire-level route and field names.

Acceptance: each story has exact input, state transitions, observable output,
and clean-stack commands; fixtures do not require email delivery; partial
bootstrap, duplicate submission, expired session, and unavailable provider are
covered.

### T02 — Specify and version the three templates

Depends on: T00, T01.

Deliverable: the chosen single source of truth for the three templates, schema
validation, and learner-visible preview examples.

Initial source of truth: [`api/templates/index.json`](../../api/templates/index.json),
loaded and validated by [`api/src/templates.rs`](../../api/src/templates.rs).

Acceptance: each template has the complete Section 8 contract; mapping an
intent produces a tested preview; templates cannot create undeclared activity
types/capabilities; a change creates a new version without rewriting journeys.

The three templates are sufficient for the first release because they cover the
three distinct initial intents already required by the stories: understand a
subject, prepare for an exam, and produce an artifact. The catalog must remain
extensible. After the first journey loop is working, evaluate these additional
templates as separate versioned work rather than adding arbitrary prompt
variants:

- `refresh-knowledge` — recover a previously learned topic with a short
  diagnostic and spaced review;
- `learn-from-source` — turn a supplied book, article, or documentation source
  into grounded objectives and activities;
- `teach-back` — learn by explaining a concept and receiving feedback on the
  explanation.

Each candidate requires a real story and acceptance fixture before entering the
built-in catalog.

### T03 — Implement the clean persistence baseline

Depends on: T00, T02.

Deliverable: a new clean baseline for principals/sessions, goals/journeys,
template versions, objectives/skills, activities, assessments/questions,
attempts/evidence, recommendations, and agent runs/audit.

Acceptance: a clean volume creates it without compatibility tables, columns,
backfills, or dual reads; ownership and actor/subject rules are explicit;
version, idempotency, lifecycle, and objective-to-evidence linkage are
queryable; bootstrap retries cannot duplicate starter content.

Required next-task scope: replace the current PostgreSQL migration baseline with
the new clean schema for the journey model. This is a required future task, not
optional cleanup. It may be deferred to a separate session, but no API or
frontend implementation should claim the redesign is complete until a clean
PostgreSQL volume can create and exercise the new baseline. There is no data
migration from the old model; local validation uses a clean volume and the
deployment instructions must document that reset boundary.

### T04 — Implement self-host-first identity and registration

Depends on: T03.

Deliverable: one registration/session contract for the configured self-host
mode, including intent capture and safe existing-account behavior.

Acceptance: mode is enforced server-side; no unsupported SMTP/verification
promise appears; a new learner gets durable session and bootstrap state; an
existing email cannot be silently claimed; expiry, restart, logout, and retry
are tested.

### T05 — Implement the unified journey and activity API

Depends on: T02, T03, T04.

Deliverable: redesigned API for onboarding, learner state, journey preview,
activities, sessions, attempts, results, evidence, recommendations, and
agent-scoped operations.

Acceptance: Story A passes without a frontend; responses contain all UI/agent
fields; no duplicate API surface exists; ownership, actor, rate-limit,
idempotency, and audit checks pass; partial failures are resumable.

### T06 — Implement bootstrap and learning-loop agent capabilities

Depends on: T02, T05.

Deliverable: template selection, goal/objective proposal, starter creation,
targeted practice, feedback, revision, and next-action capabilities.

Acceptance: Stories A–D pass through the published agent contract; generated
items have objective/skill linkage and explanation metadata; writes are scoped
and attributable; revocation and retries work; provider failure cannot produce
false learning progress.

### T07 — Rebuild the learner frontend around the stories

Depends on: T05, T06.

Deliverable: landing prompt, template preview, first journey/activity, result
explanation, next action, resume, adjust, empty, and failure states.

Acceptance: Stories A–C pass through the Caddy origin; first screen asks for an
intent and configured identity fields; preview shows promise/objectives/effort;
results show feedback and one next step without raw JSON; self-hosting has one
primary entry point.

### T08 — Regenerate and align public contracts

Depends on: T05, T06, T07.

Deliverable: generated OpenAPI, frontend client/schema, `skill.json`,
`llms.txt`, and canonical `docs/public` pages for the actual stack/API.

Acceptance: generation is reproducible; frontend/API documents agree
semantically; docs do not claim unsupported email, routes, or components; raw
machine documents and human self-host guidance are tested.

### T09 — Validate the recommended local stack

Depends on: T03, T04, T05, T07, T08.

Deliverable: clean-volume Compose runbook and smoke checks for Postgres, Valkey,
API, web, and Caddy.

Acceptance: the five-service stack reaches declared health/started state; public
and direct API routes work; restart/resume and backup/upgrade smoke checks pass;
web resource limits are sufficient or the documented requirement is corrected.

### T10 — Simulate every story and release evidence

Depends on: T01, T09.

Deliverable: evidence bundle per Story A–E with commands, redacted HTTP traces,
database assertions, browser evidence, and pass/fail results.

Acceptance: all stories pass on a clean stack; Story C proves shuffled-MC
correctness and recommendation provenance; Story D proves cross-owner denial
and revocation; Story E proves public docs, no SMTP dependency, and restart
resume; any failed check blocks release.

### T11 — Remove obsolete surface and close the release gate

Depends on: T10.

Deliverable: removal of disposable legacy schema/API/frontend paths, updated
checks, and final implementation handoff.

Acceptance: no unused compatibility route/table/UI adapter remains; documented
checks pass; reset/deployment instructions reflect clean-slate behavior; the
Definition of done is evidenced rather than asserted.

## 11. Implementation phases

The task IDs are the work breakdown. The phases below group delivery; they do
not permit skipping task gates.

### Phase 0 — Freeze the new model

- Approve the vocabulary and state machines.
- Write target contract tests for registration, journey, agent, session, and
  result behavior.
- Define registration modes and existing-account collision behavior.
- Define the first journey response and failure states.
- Define the exact fixtures for all five user stories.
- Mark the old schema, API, and frontend contracts as disposable reference.

Gate: a reviewer can execute each target story as an API-level scenario before
UI work.

### Phase 1 — New domain and persistence baseline

- Replace the baseline schema with goal/journey/objective/activity/progress
  primitives after the model review.
- Define ownership, actor attribution, versioning, and idempotency.
- Define clean-volume bootstrap and seed fixtures.
- Remove obsolete tables and columns from the new baseline instead of writing
  compatibility migrations.

Gate: clean database tests prove isolation, retry safety, and durable journey
state.

### Phase 2 — New unified API and bootstrap contract

- Implement the new registration, session, journey, activity, and agent
  contracts from the target OpenAPI design.
- Add self-host registration modes.
- Build internal bootstrap orchestration using existing resource services.
- Add resume and failure recovery through the new journey/session contract.

Gate: Story A works through the new HTTP API without a frontend.

### Phase 3 — New agent capabilities

- Implement bootstrap, learning-loop, revision, evidence, and recommendation
  capabilities in the new agent contract.
- Update actor/subject checks, activity records, and ownership tests.
- Regenerate OpenAPI, skill manifest, and client artifacts.

Gate: Story D passes with a real generated agent token and cross-owner attacks
are rejected.

### Phase 4 — New learner frontend

- Replace the landing page and authenticated learner shell with the new
  intent-first journey model.
- Add journey preview, first-session state, result explanation, next action,
  resume, adjust-goal, empty, and failure states.
- Keep agent/API/self-hosting links secondary.

Gate: Stories A–C pass in a browser against the local stack.

### Phase 5 — Public and self-host contracts

- Align `docs/public/`, README, deployment docs, Caddy examples, and the new
  API behavior.
- Verify frontend and API versions of public discovery documents.
- Run clean-volume Compose validation without SMTP.

Gate: Story E passes exactly from the documented commands.

## 12. Simulation protocol

After implementation, every user story must be simulated against AME itself,
not merely represented by unit tests.

For each story, the implementation handoff must include:

- command used to start the stack;
- browser/API actions performed;
- request and response evidence with secrets redacted;
- database assertions;
- screenshots or rendered-route evidence where UI behavior matters;
- expected and observed result;
- any failure/retry/cross-owner checks;
- final pass/fail decision.

The simulation must use the real recommended stack:

```text
Postgres + Valkey + API + web + Caddy
```

No story is complete because a mock agent or isolated component test passed.
The final gate is a clean local-stack run with the public origin, direct API
compatibility routes, real auth/session cookies, real generated resources, and
real result progression.

## 13. Pre-implementation checklist

Before changing code or migrations, review and settle:

- [ ] Product promise and first-user story.
- [ ] Registration modes and existing-account collision policy.
- [ ] Whether the local session is sufficient for the first journey and how a
      returning learner establishes durable credentials.
- [ ] Goal/journey/objective/activity vocabulary.
- [ ] Objective-to-question and objective-to-result relationship.
- [ ] Actor/subject authorization and audit semantics.
- [ ] Bootstrap retry and partial-failure behavior.
- [ ] Minimum generated-content quality rules.
- [ ] Exact metadata required for first-session preview.
- [ ] New public OpenAPI/skill/llms contract.
- [ ] Story simulation fixtures and evidence format.
- [ ] Self-host operator controls for open/invite/password registration.
- [ ] Clean baseline schema and explicit decision that no legacy migration or
      compatibility layer will be implemented.

## Definition of done

A new self-hosted AME visitor can submit a natural-language learning intent and
email identifier, receive a local learner session, understand the generated
goal and first assessment, complete it, and receive a defensible next action.

An agent can extend that journey through the new unified API without escaping
owner boundaries. The new data model preserves learning history, objective
evidence, agent attribution, and journey progress. Every user story has been
simulated by Codex against the running AME stack and the evidence is attached
to the implementation handoff. No legacy migration or compatibility layer is
required for this redesign.
