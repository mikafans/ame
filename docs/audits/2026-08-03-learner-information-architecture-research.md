# Learner information architecture research report

Date: 2026-08-03  
Status: approved direction — implementation follows the course-scoped model below

> Existing plans and screens are implementation history, not product authority.
> This report starts from AME's agent-first product premise, audits the current
> code only as evidence of what exists, and records the approved course-scoped
> direction that implementation must satisfy.

## Executive conclusion

AME should have one global learner destination, **Learning**, with a
course-scoped tab bar inside it. It should not have one overloaded dashboard,
and it should not add another global sidebar item for every learner concern.

The proposed tabs are:

| Tab | Primary learner question | Scope |
| --- | --- | --- |
| Learn | What should I do now? | selected course |
| Course | What did my agent create, and how is it structured? | selected course |
| Progress | What have I actually demonstrated and what needs review? | selected course |
| Sources | Why should I trust this course and activity? | selected course |
| Courses | Which course do I want to resume or revisit? | learner-owned library |

`/agent` remains outside Learning. It is the setup and authorization surface
for an external agent, not a learner-workspace tab.

This is not a recommendation to copy Coursera or edX. It is a candidate
definition that uses their useful separation of: current work, course
structure, progress, and discovery. AME adds two product-specific
requirements: agent-authored course provenance and evidence-based progress.

## Research question

How should AME let a learner with one or more agent-authored courses:

1. resume meaningful work without distraction;
2. understand the course that an agent authored;
3. inspect evidence, feedback, and sources without mistaking them for work;
4. return to another active or completed course; and
5. keep agent setup outside the learning flow?

## Method and evidence boundary

This report used:

- the current AME learner shell, Learning Desk, course-detail route, and
  generated API surface;
- public Coursera course and product pages plus Coursera's published progress
  and preview explanations;
- public edX course/product pages and its learner syllabus guidance.

The research did **not** use an authenticated Coursera or edX learner account.
Claims below therefore concern public course-information and stated product
patterns, not private dashboards, A/B variants, or implementation details.

## External findings

### Coursera

Public Coursera course pages make course structure visible before lesson work:
they present outcomes, level, schedule/effort, module count, assessments, and
module detail. The `Learning How to Learn` course, for example, exposes four
modules, workload, skills, and assignment count before the learner enters the
course.

Source: <https://www.coursera.org/learn/learning-how-to-learn>

Coursera's published progress-design explanation says that its course home
surfaces a progress bar, an ordered week-by-week summary, and a recommended
next video, reading, or assignment. The important pattern is not the visual
style: the learner receives one next action while retaining an orienting course
outline.

Source: <https://blog.coursera.org/new-progress-tracking-features-on-coursera/>

Coursera's 2025 preview update gives prospective learners the first module and
course features rather than only a sales description. This supports AME's
requirement that the first learner view contain real course work, not platform
marketing or agent implementation detail.

Source: <https://blog.coursera.org/introducing-courseras-new-course-preview-experience/>

### edX

edX describes the enrolled learner flow as: access course materials, complete
assignments, and participate in the learning community. It distinguishes this
course flow from selecting a programme or credential.

Source: <https://www.edx.org/about-us/how-it-works>

Its learner guidance tells an auditor to begin by reading the syllabus, which
contains the schedule, readings, assignments, and due dates, then work through
the materials. This supports a separately discoverable course-outline view.

Source: <https://www.edx.org/resources/how-to-audit-an-edx-class>

edX public course pages also expose effort, prerequisites, outcomes, and the
learning track before the learner starts. This is a useful course-overview
pattern, but AME must replace certificate/price/track information with agent
brief, source set, and assessment contract.

Source: <https://www.edx.org/learn/how-to-learn/edx-how-to-learn-online?index=product>

## What the research means for AME

### Do adopt

1. **One dominant current action.** Learn must make the current activity and
   its purpose obvious. It is not a dashboard of every available capability.

2. **A visible, ordered course model.** Course must show the module sequence,
   outcomes, activity types, expected effort, current module, and completion
   state. The learner needs this to judge whether the agent-authored course is
   coherent.

3. **Separate progress from task execution.** Progress should contain attempts,
   answer-specific feedback, evidence, review queue, and objective trends.
   Showing empty `0% signal` cards on Learn gives no useful decision support.

4. **Separate course selection from course work.** Courses must list every
   learner-owned active and completed course. Course switching should be
   explicit and reversible.

5. **Show trust information in course context.** Sources should state what was
   used, what was reviewed, where it applies, and how to inspect citations.
   It should not expose raw source snapshots, byte ranges, or internal hashes as
   the default learner experience.

### Do not copy

1. Marketplace, certificate, rating, subscription, and institution surfaces.
   They are outside AME's agent-authored-course purpose.

2. Coursera's schedule/deadline model as a default. AME's learner may set a
   time budget, but its progress must remain evidence-derived and not pretend a
   missed calendar target is failed learning.

3. A generic catalog-first landing screen for a learner who already has work.
   The agent-first handoff must lead to the prepared course.

4. A broad analytics dashboard as the first returning-learner screen.

## AME current-state audit

### Existing strengths

- The global learner shell deliberately has one `Learning desk` navigation
  item. This is a sound global-navigation constraint.
- `GET /api/v1/learning/journeys` already returns owner-scoped journey
  summaries, and existing detail routes use `/learning/journeys/{id}`.
- A detail route already loads activities, chapters, assessments, attempts,
  deep dives, and completion state.

### Existing gaps

1. The Learning Desk selects the first active journey and does not provide a
   learner-facing way to switch to another owned course.
2. The old desk mixed five distinct concerns: current work, all courses,
   sources/citations, export, and analytics. The latest revision removed the
   extra content but did not establish its destinations.
3. The course detail is operationally rich but does not yet present a compact,
   learner-readable course brief: promise, expected effort, outcomes, module
   map, source set, and agent rationale.
4. The source-library component is an internal-data view (raw content,
   content hashes, byte offsets). It is not an appropriate learner Sources tab.
5. The export and analytics components remain implemented but are now
   unreachable from the learner UI.
6. Simulated learners currently live in separate accounts. This proves owner
   isolation, but it does not exercise the essential multi-course story for one
   learner.

## Current API contract matrix

The candidate UX does not require a new learner identity or a parallel course
model. The owner-scoped journey API is the correct base. The gaps are in
learner-oriented projections and associations, not in creating a new global
dashboard API.

| UX need | Current evidence | Contract verdict | Required change before UI |
| --- | --- | --- | --- |
| Courses list | `GET /api/v1/learning/journeys` returns owner-scoped title/goal, status, next activity, and creation date | Partial | Add deterministic recency or last-learning timestamp, completion summary, current module, and review-due summary. Never choose `journeys[0]` as product policy. |
| Learn | Journey detail returns chapters, published activities, recommendation, and learner-owned status | Mostly present | Define a stable selected-course rule and a learner-ready next-activity projection with expected effort and purpose. |
| Course | Detail returns goal, objectives, chapters, and activities | Partial | Publish and return a learner-facing course brief: prerequisites, time budget, module rationale, expected effort, and reviewed/publication summary. |
| Progress | Journey analytics and attempts exist; due reviews are owner-scoped | Partial | Define objective/evidence projection that suppresses empty metrics, carries feedback/rationale, and scopes due review to the selected course. |
| Sources | Source snapshots and citations are owner-scoped but globally listed | Missing for learner UX | Add course/activity association and a learner-readable source projection. Do not use the raw snapshot response as tab data. |
| Agent authorship | Course revisions, review/publish commands, and scoped delegations exist | Partial | Expose a published-course provenance summary to the learner without exposing delegation secrets, author drafts, or raw internal logs. |
| Course isolation | Journey, analytics, review, citation, and source endpoints all perform owner checks | Present at endpoint level | Add multi-course browser tests proving that switching and deep links cannot cross owner/course state. |

### Consequences

1. **Courses can be implemented only after list metadata is enriched.** A list
   that has no last-learning time, completion summary, or due-review state
   cannot help the learner choose.

2. **Sources requires new association data, not a new view over existing raw
   storage.** Today a source snapshot belongs to an owner but is not returned
   as a reviewed, course/module/activity-specific learner object.

3. **Course needs a published brief as a server contract.** Constructing it
   from optional goal/activity fields in the browser would let a partial or
   draft course look complete.

4. **Progress can reuse analytics only as supporting evidence.** The primary
   learner progress projection must be driven by activities, attempts,
   feedback, and completed/reviewed evidence, not a generic percentage.

## Proposed learner-view contract

This is the smallest contract that supports the selected Coursera-style model
without reconstructing course state in the browser. It is a design proposal,
not an implemented API.

### 1. Course library

`GET /api/v1/learning/courses`

Returns one owner-scoped entry per learner-visible published journey, ordered
by `lastLearningAt DESC`, then creation time. It is the data source for **My
Learning** and course selection.

```json
[
  {
    "journeyId": "uuid",
    "title": "Operate Apache Flink state safely",
    "goal": "Debug and recover a stateful Flink job",
    "status": "active",
    "lastLearningAt": "2026-08-03T12:00:00Z",
    "progress": { "completed": 3, "total": 16 },
    "currentModule": { "id": "uuid", "title": "Checkpoint recovery", "position": 2 },
    "next": { "activityId": "uuid", "title": "Trace a failed checkpoint", "estimatedMinutes": 15 },
    "review": { "dueCount": 1 }
  }
]
```

Rules:

- drafts, failed authoring runs, and another learner's courses are absent;
- `completed` courses remain listed but never become the automatic selected
  course if an active course exists;
- a course has `next: null` only when it is completed or waiting for explicit
  agent/learner review;
- `lastLearningAt` changes only from learner activity, not agent authoring.

### 2. Selected-course shell

`GET /api/v1/learning/journeys/{journey_id}/learner-view`

Returns the selected course's published learner projection. The browser may
load individual tab data lazily, but this shell must contain the stable title,
selected revision, publication state, and tab availability.

```json
{
  "journeyId": "uuid",
  "title": "Operate Apache Flink state safely",
  "status": "active",
  "publishedRevisionId": "uuid",
  "tabs": ["learn", "modules", "progress", "resources"],
  "next": { "activityId": "uuid", "title": "Trace a failed checkpoint", "estimatedMinutes": 15 },
  "courseUrl": "/learning/journeys/uuid"
}
```

The existing `/learning/journeys/{id}` route remains the canonical URL. Tab
selection is a query parameter (`?tab=learn`, `?tab=modules`, and so on), not a
new top-level route or a client-only hidden state.

### 3. Course / Modules

`GET /api/v1/learning/journeys/{journey_id}/course`

This is the **Course** and **Modules** view. It must return only the published
revision and use learner-readable language.

Required fields:

- title, learner goal, promise, prerequisites, time budget, constraints;
- authoring summary: source-reviewed course created for this learner's brief;
- outcomes with success criteria;
- modules in order, each with rationale, estimated minutes, activity types,
  completion state, and current/locked/ready state;
- assessment/application expectation per module;
- publication/review date and source-review status.

The browser must not infer course completeness from a raw activity list.

### 4. Learn

`GET /api/v1/learning/journeys/{journey_id}/learn`

This is the default course home. It contains one next action, reason, expected
effort, current-module context, compact progress, and only reviews that belong
to this journey.

It excludes all-course lists, source inventory, empty objective cards, and
long-term analytics. If no activity is ready, it explicitly says whether the
course is complete, waiting for review, or waiting for the learner to accept an
agent-proposed adaptation.

### 5. Progress

`GET /api/v1/learning/journeys/{journey_id}/progress-view`

This is not the existing analytics response rendered directly. It is an
evidence-first projection with:

- module and outcome completion derived from completed/reviewed work;
- attempts, score/result, answer-specific feedback, and rubric feedback;
- due and completed reviews for this course;
- evidence only where it exists; absent evidence is described as "not assessed
  yet", never `0% mastery`;
- agent adaptation proposals, their reason, and learner accept/defer state.

Existing analytics may be exposed as an optional detail section after the above
learner questions are answered.

### 6. Resources

`GET /api/v1/learning/journeys/{journey_id}/resources`

This is a published course/resource projection:

- source title, publisher/author, source version/date, license, and review
  state;
- short relevance statement and modules/activities that use it;
- learner-readable citation/excerpt where allowed;
- source limitations or unresolved gaps.

It must not return raw document bytes, checksums, source-storage IDs, or
citations unrelated to the selected course. Existing source/citation endpoints
remain authoring/audit primitives and cannot be reused unchanged for this tab.

## Contract-level acceptance tests

Before any visual implementation is accepted, black-box API tests must prove:

1. Haru owns active Flink, active Netty, and completed learning-science courses;
   the library lists exactly those three in deterministic order.
2. Selecting Netty returns only Netty's modules, progress, resources, and due
   reviews; a Flink activity/citation cannot appear in any selected Netty view.
3. A second learner cannot list or retrieve Haru's library or any Haru
   learner-view endpoint, including a guessed UUID.
4. A draft or failed revision cannot enter the library, selected shell, Course,
   Modules, Learn, Progress, or Resources endpoints.
5. Learn has one actionable next item, or an explicit terminal/waiting state;
   it never silently falls back to another course.
6. Progress reports no mastery score before a graded/reviewed signal and shows
   feedback after a relevant attempt.
7. Resources contain only published, selected-course source associations and
   do not return raw imported material.

## Proposed interaction model

### Entry and URL model

Keep the one global `Learning desk` navigation item. Its URL remains
`/learning`.

The selected course is represented by the existing canonical detail URL:
`/learning/journeys/{journeyId}`. A tab can be deep-linked without adding a
second made-up route, for example:

```text
/learning/journeys/{journeyId}              Learn
/learning/journeys/{journeyId}?tab=course   Course
/learning/journeys/{journeyId}?tab=progress Progress
/learning/journeys/{journeyId}?tab=sources  Sources
```

`/learning` resolves as follows:

- no owned courses: agent handoff / reviewed starting path entry;
- one active course: redirect or render that course's Learn tab;
- several active courses: return to the most recently active course's Learn
  tab, with **Courses** always available;
- no active courses but completed courses: open Courses.

This avoids a new top-level route while keeping the current canonical course
route useful, bookmarkable, and owner-scoped.

### Learn tab

Contains only:

- selected course title and one-sentence promise;
- current activity, expected effort, why it is next, and one primary action;
- concise module progress and current-module context;
- due review only when it is due for this selected course.

It excludes: other courses, all-objective cards, exports, raw source material,
long-lived analytics, generic AME explanation, and agent authorization.

### Course tab

Contains:

- agent-authored brief: learner goal, current level, time budget, constraints;
- outcomes and prerequisites;
- ordered modules, activity types, expected effort, and completion state;
- course publication/review status expressed in learner language;
- a compact explanation of why the course is structured this way.

It must distinguish published learner material from draft author material.

### Progress tab

Contains:

- completed activities and attempts;
- immediate/terminal feedback and rationale links;
- review schedule and due items;
- objective evidence and trends only after evidence exists;
- adaptation proposed by the agent, with the learner able to accept or defer.

It must never let an agent appear to have completed learning on the learner's
behalf.

### Sources tab

Contains:

- reviewed source list with title, author/publisher, version/date, license,
  and short relevance statement;
- source-to-module/activity links;
- learner-readable citations and excerpts where licence permits;
- reported limitations or source gaps.

It excludes database identifiers, raw imported blobs, checksums, byte ranges,
and generic global source inventory.

### Courses tab

Contains every learner-owned course, grouped by Active and Completed, with:

- title / goal;
- current module and next activity;
- progress and last-learning time;
- status such as active, review due, complete, or waiting for agent review;
- resume/open action.

This tab is the answer to “where are my other journeys?” It is not a catalog
and it must never show another learner's courses.

## Roles and boundaries

| Surface | Learner | External agent | AME |
| --- | --- | --- | --- |
| `/agent` | creates/revokes a scoped handoff | receives explicit contract and capability | limits authority and records provenance |
| Learn | completes work | cannot complete work | recommends deterministic next action |
| Course | reads published brief/syllabus | authors draft and responds to validation | publishes only reviewed/valid material |
| Progress | reads feedback/evidence, accepts adaptation | reads allowed durable evidence | computes and preserves evidence |
| Sources | audits course trust | imports/cites allowed sources | records review and provenance |
| Courses | chooses a course | no cross-course access by default | owner-scopes list and detail |

## Acceptance stories required before UI implementation

1. **One WIP course.** Haru signs in and lands on Learn for the published
   Flink course. Only its next activity and its due review are prominent.

2. **Multiple courses.** Haru owns an active Flink course, an active Netty
   course, and a completed course. Courses lists all three; selecting Netty
   changes Learn, Course, Progress, and Sources to Netty without leaking data.

3. **Agent-authored brief.** Haru can see what the agent was asked to prepare,
   module outcomes, effort, sources, and publication state without seeing a
   draft as a learner-ready course.

4. **Truthful progress.** Before a graded/reviewed result, Progress has no
   invented percentage. After an assessment, it presents score, feedback,
   evidence, and next review truthfully.

5. **Source trust.** Haru can follow a lesson source to its reviewed metadata
   and course relevance without seeing raw database storage.

6. **Isolation.** A second learner cannot list, select, or deep-link to
   Haru's Flink or Netty journeys.

7. **Responsive and accessible tabs.** The tab bar is keyboard-operable,
   focus-visible, content-fitting at 390px, and uses text/icon status rather
   than colour alone.

## Risks and open decisions

1. **Tab count:** Five is justified only if all five have real content. Until
   agent-authored brief and learner-readable sources exist, Course and Sources
   should remain planned rather than render empty panes.
2. **Selected-course persistence:** Decide whether selection follows the most
   recently active course server-side or is a browser preference. The default
   must be deterministic and never hide a due review in another active course.
3. **Completed-course visibility:** Define whether completed courses become
   read-only, can be revisited, or can be forked by an authorized agent.
4. **Global vs selected-course reviews:** The Courses tab may show a small
   cross-course review count, but Learn must show only selected-course reviews.
5. **Reference-course readiness:** The current generic simulations are not a
   valid substitute for the required Flink and Netty authored-course stories.

## Product-definition decisions required

Before any learner UI implementation, explicitly decide:

1. Is the five-tab model the AME learner UX, or should one of Course, Progress,
   and Sources be combined?
2. What is the course-selection default when a learner has multiple active
   courses?
3. What agent-authored facts must a learner always be able to inspect before
   beginning work?
4. Which source/provenance facts are learner-facing versus operational-only?
5. What does completion mean for a course: archive, revisit, revise, or fork?

Only after those decisions are approved should AME create multi-course Flink
and Netty fixtures, implement the tabs one at a time, and certify each story.
