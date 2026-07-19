# AME learning baseline PostgreSQL schema

Status: T03 schema design checkpoint

This document defines the clean PostgreSQL target for the agent-first learning
rework. It is not a compatibility migration. The implementation will replace
the current migration history with a new baseline and validate it on a clean
database volume.

## Boundary

- PostgreSQL is the source of truth for identity, learning state, content,
  attempts, evidence, recommendations, and audit history.
- Valkey remains the cache/rate-limit/session acceleration layer; it is not the
  source of truth for any learning record.
- There is no old-data backfill, dual-read path, compatibility column, or legacy
  table retained for this redesign.
- Existing local data may be discarded by the documented clean reset flow.
- The baseline must be usable by the API application role and by SQLx's clean
  volume migration command.

## PostgreSQL primitives

The baseline keeps the existing UUIDv7 helper, `pgcrypto`, and `pg_trgm`
extensions where they remain useful. All durable records use UUID primary keys,
UTC `timestamptz`, explicit `created_at`, and `updated_at` where mutable.

State fields use checked text values in the first baseline. They remain
application-visible strings so the API can evolve them without PostgreSQL enum
type churn, but every allowed value is constrained in the baseline.

## Target tables

### Identity and actors

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_users` | Human learner/operator account | Canonical email is unique when present; account status is explicit; no agent rows are disguised as users |
| `tb_identities` | Actor identity for `human`, `agent`, or `system` | One identity has one type; system identities cannot belong to a learner; agent identities have an owner |
| `tb_agents` | External agent metadata and revocation | Every agent maps to one actor identity and one learner owner; revoked agents cannot write |
| `tb_login_sessions` | Browser session | Stores only a hash of the session secret; expiry and revocation are explicit |
| `tb_api_tokens` | Agent token metadata | Token hashes only; scopes, expiry, and revocation are explicit |

The authorization vocabulary is:

```text
actor   = identity performing the operation
subject = learner owner whose journey or resource is affected
```

Every new learner-owned record stores `subject_user_id`. Every agent/system
write stores `actor_identity_id`. A human write uses the human identity as both
actor and subject unless an explicitly authorized operator action says
otherwise.

### Templates and learning intent

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_template_versions` | Versioned built-in or operator template definition | `(template_id, version)` is unique; definition is validated JSON; published versions are immutable |
| `tb_learning_goals` | Raw learner intent and normalized promise | Stores raw intent, normalized statement, selected template version, source actor, status, and idempotency identity |
| `tb_learning_journeys` | Durable learning container | One goal has one active journey in the first release; lifecycle is explicit and resumable |

The checked-in catalog in `api/templates/index.json` is the source for built-in
template versions. The database stores the exact version used by a journey so
later template edits cannot rewrite history.

### Skills and objectives

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_skills` | Learner-scoped normalized topic/skill | Unique per subject and normalized name in the first release; provenance is recorded |
| `tb_journey_objectives` | Observable outcome for one journey | Ordered; has verb, statement, success criteria, and lifecycle; belongs to one journey |
| `tb_objective_skills` | Objective-to-skill relation | Composite primary key prevents duplicate links |

Objectives are not stored as a free-text array on an assessment. They are
durable records that activities and evidence can reference.

### Activities and content

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_activities` | Ordered learner action in a journey | Has a typed `kind`, lifecycle, position, payload schema version, source actor, and provenance |
| `tb_activity_objectives` | Activity-to-objective relation | Every diagnostic, practice, application, and milestone activity has at least one objective before publication |
| `tb_questions` | Typed question content used by activities | Question kind and payload are validated together; explanation and provenance are explicit |
| `tb_question_versions` | Immutable question snapshots | `(question_id, version)` is unique; attempts reference the exact version |
| `tb_assessments` | Assessment activity configuration | Mode, time limit, scoring, and result behavior are explicit; no free-text objective array |
| `tb_assessment_items` | Ordered question membership | Item order is unique within an assessment; points override is explicit |

The first activity kinds are:

```text
explanation, example, diagnostic, practice, feedback,
application, reflection, milestone, timed_practice, recommendation
```

`tb_activities.payload` remains JSONB, but the API validates a distinct schema
per `kind` and `payload_schema_version`. JSONB is a storage format, not a claim
that all activity types share one untyped contract.

### Progress and evidence

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_learning_sessions` | One learner's active/finished interaction with activities | Belongs to one subject and journey; actor and subject are separate |
| `tb_attempts` | Submitted answer/application evidence | References exact activity/content version; immutable after grading except authorized review metadata |
| `tb_mastery_evidence` | Objective/skill evidence derived from attempts | Must reference supporting attempt/activity evidence and derivation version |
| `tb_recommendations` | Next action proposal | Stores reason, evidence IDs, source actor/run, status, and expiry/version |

Scores and recommendations are not accepted as unexplained agent assertions.
The API may derive a summary, but the durable record points back to the
attempts and objective/skill links that support it.

### Agent execution and audit

| Table | Purpose | Required invariants |
| --- | --- | --- |
| `tb_agent_runs` | One bootstrap/revision/content-generation execution | Records actor, subject, template version, provider/run identity, status, retry key, and error state |
| `tb_idempotency_keys` | Request retry protection | Scope includes actor, subject, operation, and key; successful retries return the original result |
| `tb_audit_events` | Security and product audit trail | Records actor, subject, operation, target, result, and structured metadata |

Provider failure creates a failed or resumable `tb_agent_runs` record. It does
not create completed learning progress, mastery, or a successful recommendation.

## Required constraints and indexes

The baseline must include:

- unique canonical email and unique `(subject_user_id, normalized_skill_name)`;
- unique `(template_id, version)`;
- unique active journey per goal/subject according to the chosen lifecycle;
- unique objective/activity ordering within a journey;
- foreign keys that prevent an activity, attempt, evidence, or recommendation
  from crossing subjects;
- indexes for current journey lookup, ordered activities, active sessions,
  objective evidence, recommendation retrieval, agent runs, and audit time;
- an idempotency uniqueness constraint that prevents duplicate bootstrap;
- explicit `ON DELETE` behavior for every relationship;
- row-level security only if the API consistently sets the required PostgreSQL
  session context; otherwise ownership is enforced in the application and
  tested as a deliberate choice, not left half-enabled.

## Clean-baseline implementation sequence

1. Replace the current migration set with one new clean baseline and any
   required seed migration for built-in template versions.
2. Add schema-level checks for the tables and constraints above.
3. Update Rust domain/storage modules to use the target names and shapes.
4. Reset a clean local PostgreSQL volume and run SQLx migrations.
5. Add isolation, retry, lifecycle, and foreign-key integration tests.
6. Commit the baseline separately before implementing the new API handlers.

The baseline is not complete when the SQL parses. It is complete when the API
can create a learner, goal, journey, template version reference, objective,
activity, session, attempt, evidence, recommendation, and audit event on a
clean volume with no old tables present.
