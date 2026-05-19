# Research Notes

External sources consulted during the 2026-05-19 brainstorming session, and
how each shaped the final design.

## Postgres data-type checks

- [PG18 docs: Character Types](https://www.postgresql.org/docs/current/datatype-character.html) — confirmed `text` ≡ `varchar` performance; chose `text`.
- [PG18 docs: JSON Types](https://www.postgresql.org/docs/current/datatype-json.html) — confirmed `jsonb` is correct (decomposed binary, GIN-indexable). Postgres has no `bson` type.
- [Airbyte: Postgres TEXT vs VARCHAR](https://airbyte.com/data-engineering-resources/postgres-text-vs-varchar) — additional confirmation, cost-of-schema-change argument for `text + CHECK`.
- [SitePoint: JSONB Performance & Indexing](https://www.sitepoint.com/postgresql-jsonb-query-performance-indexing/) — GIN operator class trade-offs (`jsonb_ops` vs `jsonb_path_ops`); we defer indexing payload until search appears.
- [AWS: Postgres as a JSON database](https://aws.amazon.com/blogs/database/postgresql-as-a-json-database-advanced-patterns-and-best-practices/) — expression indexes for hot paths.
- [Crunchy Data: Enums vs CHECK constraints](https://www.crunchydata.com/blog/enums-vs-check-constraints-in-postgres) — community consensus prefers `text + CHECK` for migration flexibility (no ACCESS EXCLUSIVE rewrite to remove values).
- [Close: Native enums or CHECK constraints](https://making.close.com/posts/native-enums-or-check-constraints-in-postgresql/) — corroborating real-world story.

## Learning-platform architecture

- [Moodle: Question data structures](https://docs.moodle.org/dev/Question_data_structures) — bank/engine split; question definitions vs attempt state.
- [Moodle: Question database structure](https://docs.moodle.org/dev/Question_database_structure) — teacher-data tables vs interaction tables; question-type-specific tables as extensions of `question`.
- [Moodle: Questions API](https://moodledev.io/docs/5.0/apis/subsystems/question) — plugin types (qtype, qbank, import/export).
- [Yet Analytics: xAPI in Moodle](https://www.yetanalytics.com/articles/the-value-of-an-advanced-xapi-enablement-of-moodle-considering-in-the-context-of-k12) — 135 trackable events at question/answer granularity. Our `attempts` table maps cleanly to xAPI actor/verb/object/result.

**Applied:** bank/engine module split, question versioning, per-attempt presentation state (option order).

## Assessment standards

- [IMS QTI 3.0 Overview](https://www.imsglobal.org/spec/qti/v3p0/oview) — Assessment/Section/Item (ASI) information model.
- [IMS QTI 3.0 Best Practices](https://www.imsglobal.org/spec/qti/v3p0/impl) — XML binding, content packaging.
- [QTI Wikipedia](https://en.wikipedia.org/wiki/QTI) — historical context.

**Applied:** exam blueprint mirrors ASI (test → sections → items). Separation of "item body" (prompt) from "response processing" (payload + grading) acknowledged. QTI import/export deferred but compatible.

## Spaced repetition algorithms

- [StudyCardsAI: Anki FSRS Explained](https://studycardsai.com/blog/anki-fsrs-algorithm) — DSR (Difficulty/Stability/Retrievability) model; FSRS beats SM-2 by 20–30% fewer reviews for same retention; Anki default since v23.10 (Nov 2023).
- [Anki FAQ: spaced repetition algorithm](https://faqs.ankiweb.net/what-spaced-repetition-algorithm) — current Anki algorithm reference.

**Applied (forward-compatible only):** `attempts` table records timestamp + correctness per (user, question) — exactly FSRS input. Scheduler itself deferred until usage corpus exists.

## Ability estimation: IRT vs Elo

- [Pelánek: Applications of the Elo rating system in adaptive educational systems](https://www.sciencedirect.com/science/article/abs/pii/S036013151630080X) — Elo works online without batch calibration; handles changing ability (IRT assumes constant); validated for learning environments.
- [Item Response Theory (Wikipedia)](https://en.wikipedia.org/wiki/Item_response_theory) — IRT advantages and pretest-sample requirements.
- [Comparing Elo, Glicko, IRT, Bayesian IRT (ScholarWorks dissertation)](https://scholarworks.uark.edu/etd/3201/) — Bayesian IRT 1PL most accurate at small sample; Elo competitive online.
- [Multidimensional Elo (M-ERS)](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC6450197/) — items spanning multiple skills should update multiple ability dimensions simultaneously.
- [Estimating abilities with an Elo-informed growth model](https://arxiv.org/pdf/2411.07028) — handling ability change over time.

**Applied:** confirmed Elo as the right primitive. Calibration phase for fresh questions (first 20 attempts: K_user=0, K_question=48) protects user rating from agent-misestimated difficulty. Per-tag delta splitting on multi-tag questions is "M-ERS lite" and migration to true M-ERS would be additive (per-tag-per-question rating table).

## What we *didn't* end up needing

- BSON — not a Postgres type (it's MongoDB). Initial user question about it surfaced the `json` vs `jsonb` clarification.
- Bloom's taxonomy fields on questions — personal-use scale; agent can infer from tags.
- Hierarchical tag tree — flat tags with `:` namespace (`rust:async`) gives pseudo-hierarchy at lower cost.
- Native Postgres ENUM types — `text + CHECK` is the community recommendation for evolving schemas.
