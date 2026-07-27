# AME M0 — Golden learner and agent stories

Status: product contract; implementation follows in M1–M6

This document turns the agent-native platform thesis into observable stories.
It defines what a learner should experience, what an external agent may do,
and what must be true in the database and API. It is deliberately richer than
the current starter-check journey: AME must teach, ask for evidence, respond,
and provide a meaningful next action.

The semantic fixtures for these stories are in
[`2026-07-22-m0-story-fixtures.json`](./2026-07-22-m0-story-fixtures.json).

## Story A — Learner discovers and starts a journey

Persona: Haru, a software engineer who wants to learn Apache Flink.

Intent:

> I want to learn Apache Flink fundamentals, including streams, state,
> windows, and how Flink jobs run on Kubernetes.

The learner can use an external agent to submit this intent or start through
the browser. AME presents a journey preview before the first activity:

- what the learner will be able to do;
- the chapters and their order;
- expected effort and prerequisites;
- what the first lesson will teach;
- how knowledge will be checked;
- what kinds of practical tasks will follow.

The learner is not required to invent the curriculum or understand AME's API.

Acceptance criteria:

1. The intent produces one learner-owned, resumable journey.
2. Repeating the same request with the same idempotency key resumes the same
   journey and does not duplicate content.
3. The preview names concrete Flink outcomes rather than repeating generic
   onboarding copy.
4. The first available action is an actual lesson with instructional content.

## Story B — Learner studies a chapter

The Flink journey contains four chapters:

1. Streams, events, and time
2. State, consistency, and recovery
3. Windows, watermarks, and late data
4. Job execution and the Flink Kubernetes Operator

Each chapter contains a sequence of learning activities. A typical chapter
flow is:

```text
lesson → worked example → knowledge check → practical task → review
```

The learner can open an activity and understand its purpose before starting.
Activities may include rich text, diagrams, code blocks, runnable examples,
scenarios, and structured prompts. Markdown is one content block, not the
activity model.

Acceptance criteria:

1. The Learning Desk shows chapter order, objective progress, current activity,
   and a clear next action.
2. A lesson contains enough explanation to answer the following check without
   relying on unexplained self-reflection.
3. A worked example connects the concept to a concrete Flink input, operation,
   state transition, or deployment decision.
4. Interactive content has a stable fallback when a runtime capability is not
   available.
5. Leaving and returning to an activity preserves the learner's position.

## Story C — Learner demonstrates knowledge

At the end of each chapter, the learner completes a short assessment. The
assessment is composed from versioned questions linked to chapter objectives.

The first Flink assessment should include:

- multiple-choice: identify keyed versus non-keyed processing;
- true/false: explain why state requires fault-tolerant recovery;
- short answer: describe the role of watermarks;
- scenario/task: choose a windowing strategy for late events.

The system grades objective items on the server, explains the answer, and
records objective evidence. Short-answer, code, or scenario work may be
pending review; pending review must be visible rather than represented as a
false score.

Acceptance criteria:

1. The learner sees question purpose, progress, and answer state.
2. The learner can resume an interrupted attempt.
3. Results show score where grading is objective and review status otherwise.
4. Each result explains why an answer was accepted or rejected.
5. Weak objectives lead to a targeted review or practice recommendation.
6. Practice completion and graded-exam completion remain distinct states.

## Story D — Learner completes an application task

The learner must use the concept, not only recognize an answer. A Flink task
can begin as a structured design or debugging exercise and later grow into a
code or terminal lab without changing the learning contract.

Example task:

> A stream contains out-of-order click events. Choose a window, watermark, and
> allowed-lateness policy. Explain what happens to an event that arrives after
> the window is emitted.

The task records the learner's submission, the objective being tested, the
evaluation method, feedback, and review state. Completing the task contributes
evidence only through the declared evaluation contract.

Acceptance criteria:

1. The task states the expected artifact and evaluation criteria.
2. A learner can submit, revise when allowed, and see submission status.
3. Agent or human feedback is attached to the submission and objective.
4. A task cannot silently mutate an assessment score or mastery snapshot.
5. The UI can evolve from structured responses to code or sandbox artifacts.

## Story E — Agent adapts the journey

After the learner completes a check or task, an external agent can inspect the
same evidence and propose the next action. The agent may:

- recommend the next chapter;
- insert a targeted explanation or alternate example;
- create a deeper practice set;
- request a task or review;
- explain why the learner should revisit an objective.

The agent does not directly assert mastery, pass an exam, or erase history.
Every generated resource has provenance, lifecycle, and review state.

Acceptance criteria:

1. The recommendation references the evidence and objective that caused it.
2. Generated activities appear as draft, review, or published according to
   their lifecycle.
3. Provider failure leaves the learner's existing state intact.
4. The learner can accept, defer, or ask for another explanation.
5. The same recommendation can be rendered by the browser and read by an
   authorized external agent.

## Story F — Learner returns and resumes

Haru leaves after Chapter 1, then returns later through the browser. The
journey opens at the next meaningful activity, preserving completed work,
unfinished attempts, results, recommendations, and evidence.

Acceptance criteria:

1. Refresh and API/web restart preserve the journey state.
2. The current activity is deterministic and explainable.
3. Completed work is not recreated or counted twice.
4. A qualifying learning event is recorded once with timezone semantics.
5. A second authorized client sees the same state without a second learner
   identity.

## Story G — Topic-neutral proof

The same journey model is exercised with a non-Flink subject, such as an
introductory database indexing journey. It must contain the same structural
stages but different objectives, content, assessment items, and task rubric.

This prevents the Flink fixture from becoming a hardcoded product path.

Acceptance criteria:

1. The topic-neutral fixture can be created through the same agent operations.
2. No Flink-specific field or frontend branch is required.
3. Chapter, lesson, assessment, task, feedback, and recommendation behavior
   remains equivalent.

## Required evidence for M0 completion

M0 is complete when the following artifacts are committed:

- this learner-story contract;
- the semantic Flink and topic-neutral fixtures;
- a mapping from each story to API outcomes and database assertions;
- a list of browser states needed for M1–M5;
- GitHub issue drafts with one implementation boundary per issue.

M0 does not claim that these stories already pass. It makes the target
observable so implementation work cannot be considered complete merely because
rows or endpoints exist.

## Story-to-boundary traceability

| Story | API outcome | Database assertion | Browser proof |
| --- | --- | --- | --- |
| A — start | Preview/start returns one journey and first action; retry replays the original result | One learner, goal, journey, ordered chapters, objectives, and first activity; no duplicate idempotency record | Intent, preview, journey outline, and first lesson are visible |
| B — study | Journey manifest returns chapter/activity order and typed content blocks | Published activity references a validated content version and objective links | Lesson, example, interactive block, and resume state render |
| C — demonstrate | Assessment/attempt/answer/finish returns item feedback, score or pending review, and recommendation | Attempt references exact assessment/question versions; evidence only follows declared grading | Quiz progress, answer feedback, result summary, and next action render |
| D — apply | Task submission returns lifecycle and review state | Submission is linked to task, objective, content version, and reviewer/generation provenance | Task instructions, submission, pending/reviewed result, and feedback render |
| E — adapt | Agent can inspect evidence and create/recommend a versioned next activity | Recommendation records cause, target, generation run, and status; generated content is not evidence | Recommendation explains why it exists and can be accepted or deferred |
| F — return | Journey/progress reads return the same state after restart and from another client | Completed activity, attempt, evidence, and streak event remain unique and owner-scoped | Resume opens the next meaningful activity without duplication |
| G — generalize | The same manifest and activity contracts accept a non-Flink subject | No schema row or API branch requires Flink-specific identifiers | Database-indexing journey has equivalent course flow and different content |

The current 0.3 routes cover parts of this boundary. M1 should add or reshape
the manifest and chapter contracts before M2 builds more renderers. Existing
assessment, attempt, evidence, recommendation, and streak routes should be
extended only where the richer content model requires it.

## Issue-ready implementation boundaries

These are the first child issues to create under the roadmap milestones:

1. `M1: add versioned chapters and nested journey manifest`
   - Scope: chapter persistence, activity ordering, objective links, manifest
     read contract, repository and HTTP tests.
   - Non-goal: rich block rendering or analytics.

2. `M1: define typed versioned activity content contract`
   - Scope: capability registry, payload validation, lifecycle/provenance
     fields, unknown-capability fallback.
   - Non-goal: arbitrary generated frontend code or code execution.

3. `M1: mint Flink and topic-neutral golden journey fixtures`
   - Scope: canonical content versions, chapters, objectives, activities, and
     deterministic local enrollment/bootstrap.
   - Non-goal: learner-specific production migration assumptions.

4. `M2: render the first rich learner capabilities`
   - Scope: rich text, diagrams, code examples, scenarios, and structured task
     submission in the browser.
   - Non-goal: full sandbox or terminal runtime.

5. `M3: connect tasks to submissions, review, and evidence`
   - Scope: task lifecycle, rubric/review state, objective evidence, and result
     feedback.
   - Non-goal: learner analytics dashboard.

6. `M5: rebuild the Learning Desk around chapters and next actions`
   - Scope: course-like navigation, lesson/quiz/task/resume states, and clear
     recommendation UX.
   - Non-goal: current/best streak and assessment completion metrics.
