# AME agent-native learning platform roadmap

Status: planning baseline; AME version remains `0.3.0`

AME is not being optimized for an immediate release. The current clean-slate
rework is a valid foundation, but the product target is larger: a stable,
course-quality learning platform whose familiar learner experience is
continuously composed and adapted by agents.

This roadmap defines the product milestones and implementation boundaries. It
does not change the release version, publish a release, or authorize pushing
the branch.

## Product thesis

The learner should experience a coherent course or journey, not a sequence of
agent-generated prompts. The agent should be able to create, explain, assess,
adapt, and extend that journey without making the browser experience unstable
or opaque.

The target loop is:

```text
learner intent
  -> journey plan
  -> chapters and objectives
  -> lessons and examples
  -> interactive practice
  -> assessment or task
  -> feedback and evidence
  -> targeted review or next chapter
  -> durable progress and resume
```

The browser is the learner workspace. External agents are authorized clients
and curriculum operators. Both use the same durable resource model.

## Target learning model

```text
Journey / course
  └─ Chapter / module
       ├─ Lesson
       ├─ Worked example
       ├─ Interactive activity
       ├─ Quiz or assessment
       ├─ Task, lab, or project
       └─ Review / recommendation
```

An activity is not limited to Markdown. The platform should support a typed,
versioned component tree, including rich text, diagrams, code, runnable
examples, quizzes, scenarios, file submissions, terminal labs, and agent
review. Each capability must have a stable schema, a browser renderer, a
learner-state contract, and a safe fallback when execution is unavailable.

## Milestones

### M0 — Product contract and golden user stories

Define the learner, agent, and operator stories before changing the schema.
The canonical examples should include one complete Apache Flink journey and a
second non-Flink topic to prove that the model is not hardcoded.

Acceptance criteria:

- learner stories cover discovery, resume, learning, assessment, task,
  feedback, adaptation, and failure recovery;
- agent stories cover create, inspect, revise, publish, recommend, and review;
- every story has observable browser and API outcomes;
- the Flink journey has chapters, meaningful instruction, quizzes, and tasks;
- the story fixtures identify expected database state and transitions.

Issue-sized tasks:

1. Write the learner experience contract.
2. Write the external-agent operating contract.
3. Specify the Flink golden journey.
4. Specify a topic-neutral acceptance fixture.

### M1 — Versioned curriculum and journey manifest

Build the durable curriculum model behind the course-like UX.

Acceptance criteria:

- journeys contain ordered chapters/modules;
- chapters contain ordered activities and linked objectives;
- content is versioned and publishable;
- learner progress points at immutable content versions;
- the learner API returns one nested, resumable journey manifest;
- migrations and fixtures are idempotent and safe for clean local stacks.

Issue-sized tasks:

1. Add chapter/module persistence and ordering.
2. Define objective and activity relationships.
3. Define content version and publication state.
4. Add the nested journey-manifest API.
5. Add repository, PostgreSQL, and HTTP contract tests.

### M2 — Rich activity and frontend capability runtime

Move from Markdown-oriented activities to typed learning capabilities.

Initial capabilities should include rich text, callouts, diagrams, code
blocks, runnable examples, quiz blocks, scenarios, and task submissions.

Acceptance criteria:

- each capability has a versioned payload schema;
- unknown capabilities render a useful fallback;
- the browser can resume an activity without losing local or server state;
- interactive state is persisted through explicit activity contracts;
- generated content cannot inject arbitrary unsafe frontend behavior;
- accessibility and keyboard behavior are tested for every initial capability.

Issue-sized tasks:

1. Define the activity block schema and registry.
2. Build the learner activity renderer.
3. Add diagram and code-example blocks.
4. Add interactive scenario and task blocks.
5. Add capability validation and safe fallback behavior.

### M3 — Assessment, task, and evidence engine

Make learning outcomes verifiable. A learner should demonstrate knowledge or
skill, and AME should explain what the result means.

Acceptance criteria:

- quizzes support objective grading and answer rationales;
- essays, code, and projects support pending/manual or agent-assisted review;
- tasks have submissions, rubrics, and review state;
- evidence is linked to objectives and the exact content version;
- assessment completion and task completion are distinct states;
- failed or abandoned work cannot create false mastery.

Issue-sized tasks:

1. Define assessment and task lifecycle states.
2. Add rubric and submission contracts.
3. Add result and feedback components to the learner UX.
4. Add objective evidence transitions and regression tests.
5. Verify a complete Flink quiz and task path.

### M4 — Agent curriculum operations and adaptation

Give agents powerful operations over the learning model while preserving
validation, provenance, review, and learner ownership.

Acceptance criteria:

- agents can create and revise draft journeys;
- agents can compose chapters, activities, assessments, and tasks;
- content carries source, provider, creator, and review metadata;
- publishing validates structure, references, and capability payloads;
- agents can inspect evidence and recommend a next activity;
- agent failures are retryable and never create premature learner progress.

Issue-sized tasks:

1. Define agent commands and resource permissions.
2. Add draft/publish validation.
3. Add provenance and generation state to rich content.
4. Add adaptation and recommendation decisions.
5. Add an end-to-end external-agent fixture.

### M5 — Course-quality learner experience

Turn the model into a complete learner product with the familiar affordances
of a serious online course platform.

Acceptance criteria:

- Learning Desk shows journey, chapter, activity, and objective progress;
- the learner always has a clear next action;
- activity, quiz, task, result, review, and resume states are coherent;
- the learner can inspect why a recommendation was made;
- the learner can ask for another explanation, example, or difficulty level;
- empty, loading, failure, review-pending, and completed states are designed;
- the complete Flink golden journey is usable without API knowledge.

Issue-sized tasks:

1. Redesign the Learning Desk around chapters and next actions.
2. Build lesson and interactive activity layouts.
3. Build quiz, task, result, and review flows.
4. Add agent-assisted alternative explanations.
5. Run browser acceptance tests against the local stack.

### M6 — Reliability, security, and content quality

Make the platform dependable as content and agent capabilities grow.

Acceptance criteria:

- all state transitions are idempotent and ownership-checked;
- published content is immutable or explicitly versioned;
- capability payloads are schema-validated and sanitized;
- provider failure, timeout, retry, and partial-generation behavior are tested;
- migrations, fixtures, and restart behavior are repeatable;
- the Flink and topic-neutral stories pass on a clean local stack.

Issue-sized tasks:

1. Add contract coverage for content and progress transitions.
2. Add authorization and provenance audit coverage.
3. Add provider retry and failure recovery tests.
4. Add fixture reset and clean-stack verification.
5. Document operational and security boundaries.

### M7 — Learner analytics and retention (0.4 roadmap)

Add analytics after the underlying event and evidence model is trustworthy.

Planned metrics include current/best streak, assessment completion rate,
average score, attempts, time spent, objective mastery trends, and review
history. Each metric needs an explicit denominator, timezone rule, and privacy
boundary.

This milestone does not block the richer 0.3 learning experience.

### M8 — Extensible interactive ecosystem

Add simulations, sandboxes, reusable content libraries, collaborative
activities, author preview, and self-host instance sharing after M1–M6 are
stable. These are capability families, not a reason to weaken the core
curriculum or learner-state contracts.

## Database and implementation boundary

The database should distinguish:

```text
curriculum content
  journeys, chapters, objectives, activities, blocks, assessments, tasks

learner runtime
  enrollments, activity state, sessions, attempts, answers, submissions,
  evidence, recommendations

agent operations
  intents, generation runs, provenance, review decisions, audit events
```

Relational columns should own identity, ordering, ownership, lifecycle, and
version references. Typed JSON payloads should own capability-specific content.
The API should expose a nested manifest and explicit commands; agents should
not write arbitrary rows directly.

The first PostgreSQL migration should mint canonical content and stable
versioned fixtures. A local bootstrap step may enroll `haru@example.com`, but
production migrations should not depend on one learner email.

## GitHub tracking convention

Create one parent issue for each milestone and child issues for the numbered
tasks. Use labels such as `roadmap`, `milestone:m0`, `milestone:m1`,
`frontend`, `api`, `db`, `agent`, `content`, and `verification`.

Each implementation issue should define:

- user-visible outcome;
- domain/API/database scope;
- tests and local-stack evidence;
- dependencies;
- one conventional commit boundary;
- explicit non-goals.

The branch remains unpushed until the user explicitly authorizes publishing.
