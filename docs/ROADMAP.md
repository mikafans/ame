# AME roadmap

## 0.5.0 — agent-authored courses

Status: planned. This is the next product release; it does not authorize a
release, tag, push, or hosted publication.

AME is the durable learning system an authorized agent uses to create and
operate a real course for one learner. It is not a catalog of generic
templates, a chat transcript, or a browser shell around activity rows.

The 0.3.1 foundation already provides owner-scoped journeys, chapters,
activities, versioned questions and assessments, tasks, evidence, sources,
provenance, and browser resume. 0.5.0 connects those capabilities into one
reliable authoring and learner loop.

### Release outcome

Given a learner's domain, goal, current level, time budget, and constraints,
an authorized agent can assemble, validate, publish, and adapt a
source-grounded course. The learner can complete it in the browser without
needing API knowledge or accepting invented progress.

```text
learner brief
  -> agent course brief and source set
  -> measurable outcomes and module map
  -> instruction and worked examples
  -> formative checks with answer rationale
  -> assessment or applied task with feedback/rubric
  -> publish validation
  -> learner completion, evidence, and adaptation
```

The required authoring relationship is:

```text
outcome
  -> instructional item(s)
  -> formative check + explanation of each answer
  -> mastery assessment or applied task + rubric/feedback
  -> evidence-backed next module or targeted review
```

An agent must not publish a course with substituted generic content, an
outcome that has no instructional path, a lesson with no check, a check with no
answer rationale, or an application task with no evaluation path.

### What 0.5.0 delivers

#### 1. Canonical learner and agent stories

Replace the current "journey created" completion claim with the following
observable stories:

- A learner gives an agent a domain, goal, level, time budget, and constraints.
- The agent returns a course brief: outcomes, prerequisites, source set,
  module sequence, estimated effort, and the reason for each module.
- The agent creates the course as a draft; the learner cannot mistake draft
  material for an approved course.
- The learner receives each module as instruction, example, formative check,
  and either a mastery assessment or applied task.
- The learner receives answer-specific feedback and can resume an interrupted
  attempt.
- The agent reads only durable learner evidence, then adds targeted review,
  an alternate explanation, or the next module. It never manufactures work or
  mastery.

The contract, public principles, agent guide, OpenAPI descriptions, and browser
acceptance fixtures must tell the same story.

#### 2. Course draft and publish lifecycle

Add an explicit course-level draft/review/published lifecycle above individual
activity content. Publishing is a server-side validation command, not an
agent's assertion.

The publish validator must reject a course unless it has:

- a learner-facing course brief, ordered modules, measurable outcomes, and
  source/review metadata;
- at least one instructional activity and one worked example for every module;
- at least one objective-linked formative check for every instructional item;
- versioned questions, correct-answer rationales, and appropriate feedback for
  every check;
- an objective-linked mastery assessment or reviewed application task for each
  declared outcome;
- a rubric and terminal feedback/review path for open-ended work;
- valid activity order, capability schemas, immutable content references, and
  owner/provenance checks.

The course manifest must expose draft versus published state and explain any
validation failures to the authoring agent without exposing another learner's
state.

The implementation work includes the currently missing pieces: agent-created
and revisable objectives; draft rather than immediately-published activities;
per-activity source certification; required rationale/feedback for objective
questions; learner-visible result feedback; an authorized, provenance-recorded
agent-review path for rubric tasks; and an adaptation proposal the learner can
accept or defer. Existing chapter, activity, question, assessment, rubric,
source, evidence, and recommendation primitives are inputs, not substitutes
for this lifecycle.

#### 3. Agent authoring instructions are a product asset

Ship a versioned, discoverable agent course-authoring playbook alongside the
API, rather than expecting agents to infer pedagogy from endpoint names.

It must provide:

- the exact authoring sequence: inspect brief, import sources, map outcomes,
  author instruction, compose checks, compose assessment/task, validate,
  publish, observe evidence, adapt;
- machine-readable course and publish-validation schemas, plus complete HTTP
  examples for every operation;
- pedagogical constraints: formative checks follow instruction, options and
  answer rationales explain the reasoning, mastery checks are distinct from
  practice, and open work has an explicit rubric;
- source, licensing, review, provenance, ownership, retry, and fixture rules;
- failure handling: validation errors, incomplete generation, pending review,
  and retryable agent runs;
- two complete reference transcripts: Flink and Netty, from learner brief to
  published course and learner adaptation.

`/public/skill.json`, `/public/llms.md`, `/public/openapi.yaml`,
`/public/learning-contract.json`, `/public/learning-principles.md`, and
`/agent` are release-critical projections of this playbook. A drift test must
fail if they advertise an operation or requirement that the live API does not
support.

#### 4. First-class formative checks, assessments, and feedback

Make the distinction visible in the data model and learner UI:

- formative checks are attached to lessons, use versioned questions, and show
  immediate answer-specific rationale;
- mastery assessments use an ordered set of versioned questions, resumable
  attempts, grading/pending-review state, and result feedback;
- application work records a submission, rubric, review status, criterion-level
  feedback, and an evidence transition only after terminal evaluation;
- scenarios are not silently treated as quizzes. A scenario must declare a
  correct/evaluable answer and feedback contract, or render as an ungraded
  reflection.

#### 5. Two reference courses, not two mock titles

Publish and certify two agent-authored reference courses:

| Course                                                                                          | Required proof                                                                                                                                                                                                                                                        |
| ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Apache Flink: event time, state, checkpoints, watermarks, Kubernetes operation                  | Four modules; instruction and worked example per module; formative checks covering event time, checkpoint recovery, watermarks, and deployment; a graded mastery assessment; an applied late-event/windowing design task with rubric; adaptation after weak evidence. |
| Netty: event loops, pipelines, ByteBuf ownership, backpressure, asynchronous TCP service design | Four modules; domain-specific instruction and code/worked examples; formative checks with rationales; a graded mastery assessment; a pipeline/service design task with rubric; adaptation after weak evidence.                                                        |

Neither course may use the generic `subject-starter` copy, generic onboarding
questions as its only checks, or an ungraded scenario as a substitute for a
quiz or task.

#### 6. Learner experience and truthful progress

The Learning Desk must show the published course brief, module sequence,
current module, progress, due review, and a single clear next action. Each
lesson, check, task, result, pending-review, completed, empty, and recovery
state must fit the same course flow. Drafts are author-visible but cannot look
like learner-ready content.

Progress remains derived from the learner's graded or reviewed work; agents
cannot submit answers, complete sessions, or create evidence on a learner's
behalf.

### Delivery order and release gates

| Phase   | Deliverable                                                                                    | Gate                                                                                                                                        |
| ------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.5.0-A | Rewrite canonical user stories, course schema, publication states, and agent playbook contract | Contract tests reject a generic shell as publishable.                                                                                       |
| 0.5.0-B | Implement course draft/publish validation and agent-visible diagnostics                        | API tests cover incomplete outcomes, missing rationales, missing rubrics, provenance, ownership, retries, and immutable published versions. |
| 0.5.0-C | Wire lesson checks, assessment feedback, and application rubrics into the learner flow         | Browser proves lesson → check → result/task → resume without false evidence.                                                                |
| 0.5.0-D | Author Flink and Netty reference courses through the public agent workflow                     | API-only agent simulations create and inspect both courses without source-code access.                                                      |
| 0.5.0-E | Certify the local stack and public agent assets                                                | Fresh local stack, browser learner simulation, agent contract simulation, and documentation/OpenAPI drift checks all pass.                  |

### 0.5.0 definition of done

0.5.0 is not complete because a journey, activities, or a catalog card exists.
It is complete only when:

- a clean local stack can reproduce both published reference courses;
- an API-only external agent follows the public instructions and builds or
  revises a course without reading repository source;
- a browser learner completes representative Flink and Netty module loops and
  receives correct feedback and truthful progress;
- every published module passes the course validator;
- all public agent assets, API schemas, generated client types, and browser
  behavior agree;
- normal and fixture learner data remain strictly owner-scoped and never mix.

### Research basis

The instructional constraints above follow established online-course practice:
design backward from observable outcomes; make module outcomes visible; pair
instruction with formative practice; provide answer-specific feedback; separate
formative checks from mastery assessment; use authentic application for
performance outcomes; and complete a real learner beta pass before release.

See [Coursera on learning objectives and meaningful assessments](https://blog.coursera.org/courseras-commitment-to-learning-how-we-support-skill-development/),
[Coursera on mastery learning](https://blog.coursera.org/how-to-integrate-mastery-learning-into-course-design/),
[Open edX feedback and hints](https://edx.readthedocs.io/projects/edx-partner-course-staff/en/latest/exercises_tools/dropdown.html),
and [Open edX open-response rubrics](https://edx.readthedocs.io/projects/open-edx-building-and-running-a-course/en/open-release-sumac.master/exercises_tools/open_response_assessments/OpenResponseAssessments.html).

## Prior foundations

0.3.1 supplied the core durable learning, assessment, source/provenance,
retention, and local-stack capabilities. They are inputs to 0.5.0, not proof
that an agent-authored course exists. Broader interactive ecosystems,
marketplaces, payments, certificates, public social features, and hosted
identity remain out of scope until this course-authoring release is proven.
