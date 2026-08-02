# ame — study, sweetened

ame is a self-hostable, agent-friendly learning platform. People and agents
use the same owner-scoped learner API. An agent can start a journey, compose a
course-like path, help the learner study and demonstrate knowledge, and read
the same progress that appears in the browser.

This is the concise agent entry guide. Before mutating learner state or running
a simulation, read `/public/learning-contract.json`; the learner-facing version
is `/public/learning-principles.md`. Use `/public/skill.json` to discover named
operations and `/public/openapi.yaml` for complete request and response schemas.

## Start a learner journey

Register once and retain the returned bearer token:

```http
POST /public/v1/auth/register
Content-Type: application/json

{"email":"learner@example.com","name":"Learner","password":"at-least-8-chars"}
```

Existing learners can use `POST /public/v1/auth/login`. Both operations return
`token` and `user`. Send the token as `Authorization: Bearer <token>` on every
authenticated `/api/v1/*` request and on `POST /public/v1/onboarding/start`
after registration or login. In self-host mode, email is an account identifier;
AME does not send email.

Preview a prompt without creating state:

```http
POST /public/v1/onboarding/preview
{"prompt":"I want to learn Apache Flink checkpointing"}
```

Create or resume the learner journey with the registration/login bearer token
and a stable idempotency key:

```http
POST /public/v1/onboarding/start
Authorization: Bearer <registration-or-login-token>
{
  "email":"learner@example.com",
  "displayName":"Learner",
  "prompt":"I want to learn Apache Flink checkpointing",
  "idempotencyKey":"flink-checkpointing-v1"
}
```

## The learning model

`GET /api/v1/learning/journeys/{id}` returns one resumable manifest:

```text
journey
  └─ chapters/modules
       ├─ lesson or explanation
       ├─ worked example or code example
       ├─ interactive scenario
       ├─ quiz or assessment
       ├─ application task
       └─ review/recommendation
```

The response includes the learner's raw intent, promise, objectives, ordered
chapters, activities, immutable content versions, activity status, and the
evidence-backed recommendation. The flat `activities` field remains a
compatibility view; new clients should prefer `chapters[].activities`.

Activity content is typed by `payload.content.type`. Current capabilities are
`explanation`, `worked_example`, `rich_text`, `diagram`, `code_example`, and
`scenario`. The server validates that a capability matches the activity kind;
the browser renders known capabilities and shows a safe fallback for unknown
future capabilities. Content is versioned and approved before it is learner
visible. Text fields may contain Markdown, but an activity is not limited to
plain text.

## Learner loop

Use these operations with the learner token:

```http
GET    /api/v1/learning/journeys
GET    /api/v1/learning/journeys/{id}
POST   /api/v1/learning/courses
POST   /api/v1/learning/journeys/{journey_id}/objectives
POST   /api/v1/learning/journeys/{journey_id}/chapters
POST   /api/v1/learning/journeys/{journey_id}/activities
POST   /api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/fork
POST   /api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/validate
POST   /api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/review
POST   /api/v1/learning/journeys/{journey_id}/course-revisions/{revision_id}/publish
POST   /api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start
GET    /api/v1/learning/sessions/{id}
POST   /api/v1/learning/sessions/{id}/finish
GET    /api/v1/learning/activities/{activity_id}/content
PATCH  /api/v1/learning/activities/{activity_id}/content
PATCH  /api/v1/learning/activities/{activity_id}/rubric
POST   /api/v1/learning/activities/{activity_id}/review

POST   /api/v1/questions
GET    /api/v1/questions/{question_id}/versions/{version}
POST   /api/v1/questions/{question_id}/versions
POST   /api/v1/assessments
GET    /api/v1/assessments/{assessment_id}
GET    /api/v1/assessments?activityId={activity_id}
POST   /api/v1/assessments/{assessment_id}/attempts
GET    /api/v1/attempts/{attempt_id}
POST   /api/v1/attempts/{attempt_id}/answers
POST   /api/v1/attempts/{attempt_id}/finish
GET    /api/v1/learning/journeys/{journey_id}/attempts

POST   /api/v1/tasks/{task_id}/submissions
POST   /api/v1/task-submissions/{submission_id}/submit
GET    /api/v1/task-submissions/{submission_id}

POST   /api/v1/progress/evidence
GET    /api/v1/progress/{journey_id}/objectives/{objective_id}
POST   /api/v1/progress/{journey_id}/recommendation
POST   /api/v1/progress/streaks
GET    /api/v1/progress/{journey_id}/streaks
GET    /api/v1/progress/{journey_id}/timeline
```

Starting a session does not prove mastery. Finishing an instructional activity
records completion, while a quiz attempt or application-task submission
records demonstrable work. Practice and automatically graded assessments may
return a score. Question `kind` includes `numeric`, graded against
`acceptedAnswers[0]` (the target value) with an optional
`acceptedAnswers[1]` tolerance (an absolute number, or a percentage such as
`"2%"`); omit the tolerance for an exact match. Essay/code answers can finish
as `submitted` with `reviewStatus: pending`; do not invent a score or evidence
while review is pending. Abandoned work never creates mastery evidence.

When saving answers, use the question-kind envelope: multiple choice is
`{"option_id":"option-id"}`; true/false, short answer, and numeric are
`{"value":...}`. A numeric value may be a JSON number or numeric string. Do
not send `{"answer":...}`; it is preserved as a response but does not match a
deterministically graded numeric question.

Create a new agent-authored course with `POST /api/v1/learning/courses`; it is
empty by design and never receives generic starter material. Add measurable
objectives, ordered chapters, and objective-linked activities. AME assigns the
next order. Mark only the learner's next activity `ready`; keep later
activities `proposed` so ordinary completion unlocks them in sequence. Use a
`practice` activity as the target for a published formative assessment and an
`application` activity as the target for a reviewed task rubric.

## Course authoring sequence

Use this order for a new course and retain every returned UUID. Do not infer a
course from a generic onboarding journey or skip the source/review boundary.

1. Read the public contract and manifest, register or log in, and obtain the
   learner bearer token.
2. Import learner-provided text or an allowed public source, then certify the
   exact quoted range with `POST /api/v1/citations`. If no usable material is
   available, stop and report that the course is blocked on sources.
3. Create the empty course with a bounded brief and the citation IDs. Preserve
   `journeyId` and `revision.id` from the response.
4. For every observable outcome, create an objective with `revisionId`. Create
   its module/chapter, then an instruction and worked-example activity linked
   to that objective. Draft activities are private.
5. Start and publish a matching `learning.activity.content.compose` generation
   run; use it to attach approved, source-backed explanation/example content.
   Create an approved `question.compose` question with a rationale, feedback,
   and source references, then attach it as a published `practice` assessment
   to the instructional activity.
6. Add a distinct `graded` assessment and/or an `application` activity for
   every objective. An application activity needs a source-backed,
   `learning.activity.rubric.compose` rubric with objective-specific criteria.
7. Submit every activity with `POST /api/v1/learning/activities/{activity_id}/review`
   and the same `revisionId`. Run the course `validate` command and resolve
   every returned blocking diagnostic.
8. Submit the valid revision for review with a review record, then publish it.
   Publishing is atomic and is the only action that makes the whole graph
   learner-visible.
9. After a learner completes an assessment or receives a terminal task review,
   read the objective evidence and server recommendation. Adapt only from that
   durable evidence; an agent must never invent learner answers or mastery.

To revise a published course, use
`POST /api/v1/learning/journeys/{journeyId}/course-revisions/{revisionId}/fork`.
It clones the published course graph into a new private draft; inspect it with
the revision `GET`, edit only the new revision, and repeat validation, review,
and publication. Published course records stay immutable; never try to edit an
old revision in place.

Minimal course creation payload:

```json
{
  "rawIntent": "Design a reliable late-event clickstream service in Apache Flink.",
  "idempotencyKey": "flink-clickstream-v1",
  "brief": {
    "title": "Reliable clickstream aggregation with Apache Flink",
    "audience": "Backend engineer",
    "estimatedMinutes": 240,
    "prerequisites": ["Java", "stream processing basics"],
    "outcomes": ["Design an event-time and recovery policy"],
    "modules": ["Event time", "State and checkpoints"]
  },
  "sourceReferences": ["citation-uuid"]
}
```

Course publication is a server command, not an activity-row transition. Before
publication, an agent must run `validate`, resolve every blocking diagnostic,
then submit the revision to `review` with review metadata, and finally call
`publish`. The validator requires source-backed instruction and worked examples
in every module, approved formative questions with rationales and feedback,
and graded assessment or rubric-backed application coverage for every
objective. The old activity publish endpoint returns a migration error; it
cannot expose a single activity outside this course gate. Read completed work
with the activity-content `GET`; do not restart a completed activity.

Task submissions carry the exact activity `contentVersion`, response, review
state, evaluation method, optional score, and feedback. A task contributes
evidence only after its declared evaluation reaches a reviewed terminal state.
Learners can read their result with
`GET /api/v1/task-submissions/{submission_id}`.

Evidence must identify exactly one graded assessment attempt or reviewed task
submission, and must match the same learner, journey, objective, activity, and
content version. Recommendations, timeline events, and streak events are
derived from durable activity and evidence state rather than from generated
text alone.

Each journey has an immutable `origin`: `learner` for real learner work or
`fixture` for an explicitly designated simulation. A catalog selection never
changes this value. Authenticated journey reads and portability exports expose
it; agents must preserve it rather than infer it from progress or content.

## Native journey catalog

`GET /public/v1/catalog/journeys` returns selectable native journeys. Catalog
availability is not learner progress: every entry includes a structured
`sources` list (title, HTTPS URL, license, and optional locator or version) and
an explicit `contentReview` record. Treat those records as the public
provenance and review boundary for the catalog; do not fabricate learner work
from a catalog selection.

## Immutable source snapshots

Import bounded source text before using it for grounded content:

```http
POST /api/v1/sources/imports
GET  /api/v1/source-imports
GET  /api/v1/source-snapshots
GET  /api/v1/source-snapshots/{snapshot_id}
POST /api/v1/citations
GET  /api/v1/citations
GET  /api/v1/citations/{id}
POST /api/v1/notes
GET  /api/v1/notes?journeyId={journey_id}&activityId={activity_id}
PATCH /api/v1/notes/{id}
DELETE /api/v1/notes/{id}
POST /api/v1/learning-variants
GET  /api/v1/learning-variants?activityId={activity_id}
PATCH /api/v1/learning-variants/{id}
PATCH /api/v1/task-submissions/{submission_id}/revise
GET  /api/v1/learning/journeys/{id}/export
POST /api/v1/learning/imports
GET  /api/v1/learning/journeys/{id}/analytics?timezone={timezone}
```

`kind: url` fetches public HTTPS text with redirects disabled, private-address
resolution rejected, a ten-second timeout, and a two MiB limit. `document` and
`local_file` accept text in the request; `local_file` is a provenance label and
never asks the server to read an arbitrary path. A stable `retryKey` returns
the same immutable snapshot only when locator and content digest match.
Snapshots expose their SHA-256 digest so later citations can identify exact
bytes rather than an editable URL or filename.

## Agent authoring and adaptation

Agents may inspect the journey, create versioned questions and assessments,
author approved activity content, attach scoring rubrics to task activities,
create source-backed deep dives, and inspect progress. Generated content
requires an owner-scoped generation run with a matching operation and published
status:

```http
POST  /api/v1/generation-runs
GET   /api/v1/generation-runs/{id}
PATCH /api/v1/generation-runs/{id}
```

Use `question.compose` for questions, `deep_dive.create` for deep dives,
`learning.activity.content.compose` for reviewed activity content, and
`learning.activity.rubric.compose` for task rubrics. Reuse a
stable `retryKey` for retryable generation. Failed, cross-owner, stale, or
operation-mismatched provenance is rejected. Deep dives require matching
existing evidence and at least one source reference; only approved deep dives
are learner-readable. Use these deep-dive operations to create or read them:

```http
POST /api/v1/deep-dives
GET  /api/v1/deep-dives?activityId={activity_id}
GET  /api/v1/deep-dives/{id}
```

## Discovery and rules

- `GET /public/learning-contract.json` — required learning, evidence, source, and fixture-simulation rules.
- `GET /public/learning-principles.md` — human-readable explanation of the same rules.
- `GET /public/skill.json` — named agent operations and lightweight input schemas.
- `GET /public/openapi.yaml` — complete HTTP contract for code generation and validation.
- `GET /public/llms.md` — this concise agent entry guide.
- `GET /agent` — human-readable agent integration guide.
- `GET /self-hosting` — human-readable operator deployment guide.

Use the bearer token returned by registration, login, or onboarding. Keep every
journey and session owner-scoped. Use a stable `idempotencyKey` for onboarding.
Do not treat generated content, a selected scenario option, an unfinished
session, a practice score, or pending manual review as proof of mastery.
