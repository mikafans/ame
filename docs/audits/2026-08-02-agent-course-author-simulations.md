# API-only agent course-author simulations

Date: 2026-08-02

Purpose: establish the 0.4.0 baseline by asking independent agents to act as
external course authors. This is a black-box product test, not a source review.

## Method

Each simulation used only `http://localhost:28800` public discovery and API
endpoints. Each agent used a newly registered learner, never read repository
files, never accessed an existing learner, never used a browser, and never
submitted or invented learner work. The agents read `/public/llms.txt`,
`/public/learning-contract.json`, `/public/skill.json`, and
`/public/openapi.yaml` before acting.

For reproducibility, repeat each prompt with a fresh local account, preserve
the returned bearer token, and record the first failed stage in this sequence:

```text
discover -> register -> preview -> start/resume -> course brief -> source set
-> draft modules -> lesson -> formative check+rationale -> assessment/task
-> feedback -> publish -> learner evidence -> adaptation
```

The test passes only when an independent learner could traverse the whole
sequence. Creating a journey or an activity row is not a pass.

## Real-user prompts

### Story A — Flink backend engineer

> I’m a backend engineer. In 4 weeks, I need to design a reliable clickstream
> aggregation service in Apache Flink, including event time, watermarks,
> checkpoints, and deploying with the Kubernetes Operator. I can study 4
> hours/week.

Expected course: four bounded modules, each with topic-specific instruction,
worked example, formative check and rationale; a mastery assessment; a
late-event/windowing design task with rubric; then adaptation after weak
evidence.

Observed stages:

| Stage            | Result                                                                                                                                                                                                                   |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Public discovery | Pass: all four public documents returned `200`.                                                                                                                                                                          |
| Register         | Pass: `POST /public/v1/auth/register` returned `201`.                                                                                                                                                                    |
| Preview          | Pass: `POST /public/v1/onboarding/preview` returned `200` with sensible Flink objectives.                                                                                                                                |
| Start/resume     | Blocked: `POST /public/v1/onboarding/start` returned `422` (`email already belongs to a learner`) both without and with the documented bearer token. The documented login retry returned `401 missing or invalid token`. |

Result: the agent could not create a journey, so no course-authoring stage was
reachable. This is a public contract/runtime mismatch, not a content failure.

### Story B — Netty service maintainer

> I maintain a Java service. Over 3 weeks I need to design and debug an
> asynchronous TCP protocol service with Netty: event loops, channel pipelines,
> ByteBuf ownership, backpressure, and graceful shutdown. I can study 30
> minutes/day.

Expected course: four topic-specific modules, code/worked examples, formative
checks with rationale, mastery assessment, pipeline/service design task with
rubric, and adaptation after weak evidence.

Observed stages:

| Stage                | Result                                                                                                                                                                                                                                                             |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Register and preview | Pass: `201` and `200`. The preview supplied generic identify/explain/apply objectives rather than a time-bounded Netty curriculum.                                                                                                                                 |
| Start/resume         | Literal guide failed with `422`; retrying with bearer auth returned `201`.                                                                                                                                                                                         |
| Draft structure      | Partial: one chapter and one proposed explanation were created with `200`. The activity was already reported as `publicationStatus: published`.                                                                                                                    |
| Reviewed lesson      | Blocked: a published content-generation run completed, but `PATCH /api/v1/learning/activities/{id}/content` returned `422` because `sourceReferences` was empty. The user brief supplied no source material and the agent was intentionally not allowed to browse. |

Created: one learner-origin journey, one chapter, one activity, one generation
run. No session, answer, attempt, submission, evidence, recommendation, or
streak was created.

Result: AME correctly refuses unsourced reviewed content, but its public agent
workflow offers no source-acquisition step or usable source package before
authoring. The direct activity lifecycle also misrepresents an empty shell as
published.

### Story C — returning physics student

> I’m returning to university physics. In two weeks I want to confidently
> solve one-dimensional kinematics and Newton’s-law problems, with explanations
> when I make a mistake. I have 45 minutes per day and basic algebra.

Expected course: a constrained kinematics/forces plan, numeric formative
checks with explanations, a mastery assessment, and targeted remediation.

Observed stages:

| Stage                      | Result                                                                                                                                                                                                 |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Register and preview       | Pass: `201` and `200`.                                                                                                                                                                                 |
| Authenticated start/resume | Blocked: fresh registration followed by start returned `422`; authenticated journey list returned `200 []`; the public contract has no successful documented creation path for that state.             |
| Alternate capability probe | An onboarding-created account produced a generic four-chapter/eight-activity physics course that added energy and momentum beyond the two-week brief.                                                  |
| Authored check             | Blocked: a numeric question with rationale and no citations returned `422 source_references must not be empty`. A newly appended practice activity was already `published` without content provenance. |

No learner work, attempt, task, evidence, or recommendation was fabricated.

## Cross-field gap matrix

| Required capability                 | Flink                | Netty                                                | Physics                              | 0.4.0 action                                                                             |
| ----------------------------------- | -------------------- | ---------------------------------------------------- | ------------------------------------ | ---------------------------------------------------------------------------------------- |
| Public identity handoff             | Blocked              | Literal guide blocked; bearer retry happened to work | Blocked                              | Make the register-to-start/resume flow canonical, tested, and recoverable.               |
| Brief honors scope and constraints  | Preview was relevant | Generic objectives                                   | Over-expanded beyond requested scope | Introduce a course brief and agent-authored/revisable objectives before modules.         |
| Source acquisition                  | Not reached          | No documented source path                            | No documented source path            | Put learner materials or agent-supplied allowed URLs/documents before content authoring. |
| Draft versus publish                | Not reached          | Empty activity reported published                    | Empty activity reported published    | Add course/activity draft-review-publish lifecycle and server validation.                |
| Formative checks and rationale      | Not reached          | Not reached                                          | Not reached                          | Require source-backed versioned checks and answer-specific feedback for every lesson.    |
| Assessment/task/feedback/adaptation | Not reached          | Not present in bootstrap                             | Not present in bootstrap             | Validate the entire learner loop before publish.                                         |

## Conclusion

The existing API exposes useful primitives, but an external agent cannot yet
reliably convert a real learner brief into a publishable course. The first
release gate is not more catalog content. It is a documented, executable
authoring workflow that reaches a truthful published course from a fresh
account and source set, then proves a learner can complete it.

This audit is the acceptance baseline for 0.4.0-A through 0.4.0-E in
[`../ROADMAP.md`](../ROADMAP.md).

## Real-user agent handoff smoke — 2026-08-03

The API-only simulations above prove authoring primitives, but not the product
handoff between a human and their agent. A separate browser-only learner smoke
started at `/agent`, while an API-only agent followed the exact displayed Flink
setup brief.

Result: **blocked**. The public page could explain the intent, but it could not
bind the agent to the signed-in learner. The agent can register or log in as a
different learner and publish there; the human browser has no deliberate,
scoped delegation, journey context, or returned course URL. Linking raw
`llms.txt` is not an authorization handoff, and copying a full bearer token is
not an acceptable replacement.

The next implementation gate is therefore a learner-initiated, revocable
delegation or one-time capability. Its handoff payload must contain the AME
origin, learner/journey context, approved scope and expiry, the learner goal,
and a return learning-desk URL. The agent must use that capability to author
only in the delegated learner scope. The learner smoke passes only after it
uses the resulting browser course, completes a check, and sees feedback and
progress.

### Agent-side control result

An independent API-only agent did complete the displayed Flink prompt through
the public surface: register, authenticated onboarding, official-source import,
exact citation, private revision, objectives/chapters/activities, reviewed
source-backed lessons, question and rubric generation chains, validation,
review, and publish. The published revision was
`019fc54a-00fb-7b34-8c6c-7209c09aa192`; validation returned `[]`.

This is proof that the public authoring workflow can publish a structural
course. It is not a real-user handoff pass because that course belongs to the
agent-created test learner. The run also found two follow-up contract gaps:

- the validator requires formative assessments to attach to an explanation
  activity, but the compact contract did not say that; and
- draft recovery reads do not return activity IDs, so a restarted agent must
  retain every creation response ID.

Post-publish summary is also not a valid learner handoff: the published
journey's list-summary reported `status: onboarding`, `goal: null`, and
`nextActivity: null`, although the detailed journey read contained the reviewed
chapters, objectives, activities, and raw intent. The list/dashboard contract
must derive a published course's goal and next learner activity before the
agent can truthfully return the learner to the desk.

The feature gate remains blocked until delegation binds this successful
authoring flow to the human learner's scope and browser return path.
