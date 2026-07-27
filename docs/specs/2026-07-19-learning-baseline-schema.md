# AME 0.3 learning baseline PostgreSQL schema

Status: design checkpoint; must be revised with the behavior contracts before
implementation

This document defines the clean PostgreSQL target for AME 0.3. It is not a
migration. The implementation starts with this baseline on an explicitly reset
development volume.

## Boundary

- PostgreSQL is the source of truth for identity, learning state, content,
  attempts, evidence, recommendations, progress, and audit history.
- Valkey is an acceleration layer for cache, rate limiting, and ephemeral
  coordination. It is never the source of truth for learning records.
- There are no agent accounts, agent-owned resources, agent scopes, agent
  tokens, run-door resources, dual reads, compatibility tables, or backfills.
- The API authenticates a client session and resolves one learner subject. All
  learner resources are owned by that subject.
- Provider execution metadata is stored for provenance and retry safety; it is
  not a second product identity model.

## PostgreSQL conventions

Use UUID primary keys, UTC `timestamptz`, explicit `created_at` and
`updated_at` for mutable records, and checked text state values. JSONB is
allowed for versioned payloads only when the API validates a distinct schema for
the corresponding type and version.

## Target tables

### Identity and runtime

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_users` | Learner or operator account | Canonical email uniqueness and explicit account status; no agent rows |
| `tb_login_sessions` | Browser or API learner session | Store only a secret hash; expiry and revocation are explicit; session resolves one subject |
| `tb_idempotency_keys` | Retry protection | `(subject, operation, key)` is unique; successful retry returns the original result |
| `tb_generation_runs` | Provider/generation provenance | Records operation, template/content version, provider identity, status, retry key, and failure; never grants ownership |
| `tb_audit_log` | Security and product audit | Records request actor metadata, subject, operation, target, outcome, and structured details |
| `tb_settings` | Self-host configuration | Registration mode, provider status, timezone defaults, and operator settings are explicit |

Request metadata may distinguish browser, external agent, system bootstrap,
and operator calls for rate limiting and audit. This distinction is not exposed
as separate learning resources or authorization scopes.

### Templates, goals, and journeys

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_template_versions` | Versioned built-in/operator journey definitions | `(template_id, version)` unique; published versions immutable |
| `tb_learning_goals` | Raw intent and normalized promise | Stores subject, raw intent, normalized goal, template version, status, and idempotency reference |
| `tb_learning_journeys` | Durable learning container | Belongs to one subject and goal; lifecycle is resumable; active-journal uniqueness is explicit |

The checked-in template catalog is the source for built-in versions. A journey
stores the exact version used so later template edits cannot rewrite history.

### Objectives and activities

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_journey_chapters` | Ordered course/module grouping | Belongs to one journey and subject; chapter order is unique within a journey |
| `tb_skills` | Subject-scoped normalized topics/skills | Unique per subject and normalized name; provenance recorded |
| `tb_journey_objectives` | Observable outcome | Ordered within a journey; has statement, verb, success criteria, and status |
| `tb_objective_skills` | Objective-to-skill relation | Composite key prevents duplicate links |
| `tb_activities` | Ordered learner action | Typed kind, chapter-local position, lifecycle, content version, publication state, schema version, subject, and provenance |
| `tb_activity_objectives` | Activity-to-objective relation | Published diagnostic/practice/application/deep-dive activities have objective links |
| `tb_deep_dives` | Grounded explanatory content | Linked to objective/evidence; content version, source references, review status, and application task are explicit |
| `tb_recommendations` | Next-action proposal | Stores reason, supporting evidence, target activity, source generation run, status, and version |

First activity kinds:

```text
explanation, example, diagnostic, practice, feedback,
application, reflection, deep_dive, timed_practice, recommendation
```

The activity union is extensible. A deep dive is a first-class learning
activity, not an untracked article attached to a result.

### Questions and assessments

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_questions` | Stable logical question identity | Subject, kind, lifecycle, and provenance are explicit |
| `tb_question_versions` | Immutable question snapshots | `(question_id, version)` unique; historical attempts reference exact version |
| `tb_assessments` | Practice or graded assessment configuration | Mode, time limit, scoring, pass policy, and result visibility are explicit |
| `tb_assessment_sections` | Ordered assessment grouping | Stable section order within assessment |
| `tb_assessment_items` | Ordered question membership | Item order unique; points and question-version reference explicit |

Question kinds are `multiple_choice`, `true_false`, `short_answer`, `essay`,
and `code`. Multiple-choice options have stable identities separate from their
presentation order. An assessment stores the version it used; revision never
rewrites completed attempts.

### Sessions, attempts, and progress

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_learning_sessions` | Learner interaction with a journey/activity | One subject and journey; resumable lifecycle; client actor metadata is non-authoritative |
| `tb_attempts` | Submission and grading state | References exact activity/content version; immutable learning facts after grading except explicit review metadata |
| `tb_attempt_answers` | Individual response/evaluation | One answer per item/version/submission; raw response and evaluated result are separated |
| `tb_mastery_evidence` | Objective/skill evidence | Must reference supporting attempt/activity evidence and derivation version |
| `tb_mastery_snapshots` | Current derived learner progress | Rebuildable from evidence; never accepted from an untrusted client score |
| `tb_streak_events` | Qualifying learning events | Unique qualifying event; learner timezone and calendar-day calculation are explicit |

Scores, mastery, streaks, and recommendations are derived from evidence. A
client may submit an answer or application result, but cannot submit a trusted
mastery or streak update directly.

## Required constraints and indexes

The clean baseline must enforce or support:

- unique canonical email and subject-scoped normalized skills;
- ordered chapters and chapter-local activities, with an explicit compatibility
  path for legacy ungrouped activities;
- separate learner activity status from content publication status;
- immutable template, question, assessment, and content versions;
- foreign keys preventing resources, attempts, evidence, and recommendations
  from crossing subjects;
- ordered objectives, activities, sections, and assessment items;
- exact question-version references for every historical attempt;
- idempotency uniqueness and original-result replay;
- one qualifying streak event per subject/activity/day rule;
- indexes for current journey, ordered activities, active sessions, objective
  evidence, recommendations, timeline, generation runs, and audit time;
- explicit `ON DELETE` behavior for every relationship;
- application-level ownership checks tested at the HTTP boundary. PostgreSQL
  row-level security is optional and must not be half-enabled.

## Clean-baseline implementation sequence

1. Write the domain/repository traits and behavior contract tests.
2. Replace the migration set with this clean baseline plus versioned template
   seeds.
3. Run the same repository contract suite against PostgreSQL.
4. Add isolation, retry, lifecycle, versioning, grading, evidence, progress,
   and streak integration tests.
5. Expose the behavior through the unified API and browser.
6. Reset a clean local volume and certify C1-C6 from the project plan.

The schema is not complete when SQL parses. It is complete when a clean volume
can create a learner, goal, journey, objective, explanation, question,
assessment, session, attempt, evidence, deep dive, recommendation, mastery
snapshot, streak event, generation provenance record, and audit event while
preserving subject boundaries and retry safety.
