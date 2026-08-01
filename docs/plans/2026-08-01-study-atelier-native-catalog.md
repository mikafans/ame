# Study Atelier and native journey catalog v1

Status: approved for implementation

## Product decision

AME v1 adds a focused learner workspace and a small native journey catalog. The
catalog is an additive entry point: a learner may select a maintained journey
or create a freeform, agent-shaped journey from an intent. Both paths produce
the same owner-scoped, versioned journey model and use the same evidence,
provenance, review, retention, and export contracts.

The workspace adopts the useful parts of established course platforms without
copying their marketplace or instructor workflows: one clear resume action,
an understandable course path, visible completion state, and concise progress
signals. AME keeps its differentiator: the next action is explained by learner
evidence and an authorized agent can work on the exact same journey.

## Personas and stories

### N1 — A learner finds a suitable starting journey

A learner opens the Learning Desk with no active journey. They can see native
journey cards grouped by purpose, compare level, expected effort, outcomes,
and source/provenance, and choose one without knowing AME's API or composing a
curriculum. They can still start from a freeform intent.

Acceptance:

- every card names a stable catalog ID, title, level, estimated effort,
  outcomes, and source/provenance summary;
- selecting a card creates or resumes exactly one learner-owned journey using
  an idempotency key;
- the selected catalog version is persisted on the resulting goal and journey;
- a catalog selection never creates an agent identity, a second learner, or a
  second resource API;
- unavailable, draft, or unreviewed catalog content is not selectable.

### N2 — A learner resumes meaningful work

A returning learner sees one dominant "Continue" action that names the next
activity, its purpose, and its estimated time. The journey outline distinguishes
completed, current, ready, and planned work without relying on color alone.
Progress signals support the action rather than competing with it.

Acceptance:

- the resume action opens the deterministic recommendation already produced by
  the learner domain;
- the outline preserves chapter and activity state after refresh and restart;
- completion, review-pending, and evidence states have text or icon labels in
  addition to color;
- analytics remain available but do not displace the next action on the page.

### N3 — A learner chooses a comfortable visual environment

A learner can select one of four named visual themes: Study Atelier, Night
Study, Paper & Moss, and High Contrast. The choice persists locally and affects
all semantic UI roles consistently.

Acceptance:

- the selector is keyboard operable and exposes its selected theme to assistive
  technology;
- theme values are applied through semantic tokens for surface, text, border,
  action, focus, success, warning, destructive, and provenance roles;
- the selected theme survives reload before the first visible render;
- all text and essential UI controls meet WCAG AA contrast, and focus indicators
  meet the 3:1 non-text contrast requirement;
- High Contrast is a deliberate theme, not an arbitrary inversion of another
  palette.

### N4 — An agent and the browser share native journeys

An authorized agent can inspect, start, and continue the same catalog-created
journey the browser shows. The catalog must not fork the agent-first contract.

Acceptance:

- catalog journeys use the existing public onboarding and authenticated learner
  API boundaries; no catalog-only identity or progress endpoint is introduced;
- source-backed content and review/citation states remain inspectable to the
  learner and agent at the appropriate boundary;
- retries do not duplicate a selected native journey, activities, evidence, or
  scheduled reviews.

## Experience rules

Study Atelier is the default visual language. It uses a cool, low-chroma canvas
and ink surfaces; the AME coral action is reserved for the primary next action
and destructive states remain semantically separate. Mint indicates positive
completion, and lilac indicates provenance or agent context. Color must never
be the sole status signal.

The theme registry owns named theme selection and maps a stable semantic token
set to each theme. Components consume roles, not theme-specific values. This
allows Night Study and Paper & Moss to be genuinely useful reading environments
and allows High Contrast to prioritize clarity without creating a parallel
component system.

The Learning Desk hierarchy is:

1. Resume the recommended activity.
2. Understand the selected journey and its course path.
3. Inspect concise progress signals.
4. Browse or start another journey.
5. Access portability and detailed analytics.

## Native catalog v1

Each entry is a reviewed, versioned topic blueprint with stable identity,
learner-facing metadata, aliases, objectives, chapters, activity payloads, and
source/provenance declarations. A catalog entry may reuse an existing blueprint
only after its metadata and content meet this contract.

| ID | Set | Field | Existing state |
| --- | --- | --- | --- |
| `learning-science-starter` | Learning how to learn | learning science | new |
| `rust-ownership-starter` | Rust foundations | CS | new |
| `python-foundations-starter` | Python foundations | CS | new |
| `sql-foundations-starter` | SQL foundations | data | new |
| `linear-algebra-starter` | Linear algebra essentials | mathematics | new |
| `physics-mechanics-starter` | Classical mechanics | physics | promote existing |
| `flink-cs-starter` | Apache Flink foundations | CS | promote existing |
| `technical-writing-starter` | Technical writing for engineers | communication | new |

Every set must include a source declaration suitable for its material, clear
licensing/attribution notes where needed, and an explicit content review state.
No set may claim automatic code execution, symbolic grading, audio grading, or
another capability AME does not provide. The existing physics numeric-tolerance
and Flink manual-review paths remain the golden specialist proofs.

## Certification matrix

The release is complete only when the following run on a clean local stack:

| Boundary | Proof |
| --- | --- |
| Catalog contract | Catalog schema rejects duplicate IDs, missing outcomes, invalid source/review metadata, and unassigned activities. |
| Onboarding | A learner can select each native set and a freeform intent; retries resume without duplication. |
| Browser | Theme selection persists; catalog discovery, selection, resume, outline states, and freeform fallback are visible and keyboard accessible. |
| Learning behavior | One foundation set and both specialist sets complete their declared activity and grading paths without false progress. |
| API and persistence | Browser and authorized agent observe the same catalog journey, source state, evidence, and recommendation. |
| Accessibility | Light, dark, alternate, and high-contrast themes pass automated checks plus focused keyboard/focus verification. |

## Scope boundaries

This version does not add a marketplace, payments, certificates, public course
ratings, cohort scheduling, social learning, a generic authoring studio, or a
second agent API. It also does not turn native data into a migration-owned
database catalog: curated definitions stay versioned in the repository and are
instantiated through the existing journey contract.

The implementation order is: semantic themes; resume-first workspace; catalog
selection integration; four foundation sets; four specialist sets; deterministic
seeds and golden journeys; final live certification and documentation.
