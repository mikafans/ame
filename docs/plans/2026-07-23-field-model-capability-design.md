# AME field-model & capability-platform design (draft for review)

Status: **draft for review**. Governing intent = the M0–M8 Milestones
(`2026-07-22-agent-native-learning-platform-roadmap.md`). Treat `CLAUDE.md` as
stale where it disagrees. Supersedes the earlier "richer bootstrap generator"
framing.

## Thesis (corrected)

AME is a **capability platform**. It supplies dependable, templated learning
capabilities — typed activities, quizzes, tasks, grading, progress, evidence,
provenance — and **agents compose** a field-appropriate course out of them. The
learner experiences a coherent Coursera-style course; the agent is the author.
AME is the LMS + grading engine; the agent is the instructor.

## Two axes

- **Field (primary) — the subject**: CS, music, physics, … Fields **share a
  common spine** and differ only in their **specialized activity + grading
  types**.
- **Format (secondary) — the goal shape**: understand-a-subject / exam-prep /
  build-a-project (today's templates). Orthogonal to field.

## The commons (shared by every field)

The universal course shell (the edX/Coursera pattern): `Journey → Chapter →
Activity`, running the pedagogical cycle **concept → worked example → practice →
graded assessment → review**, plus progress, per-objective mastery/evidence,
recommendations, resume, generation-run provenance + publish gating, and owner
scoping. This spine is identical across all fields.

## Field specialization (the real per-field work)

A **field template = the commons + which specialized activity/grading types are
enabled**:

- CS → auto-graded code / exemplar (later: labs, notebooks)
- Physics/Math → numeric-answer + LaTeX problems
- Music/Arts → audio / performance submission + rubric peer/manual review
- Language → listening/speaking (later)

Most fields use only a subset; the skeleton never changes.

## Capability surface (what agents use)

AME exposes **typed-activity authoring within the course structure**
(edX-Studio-like, but programmatic for agents): author content into activity
slots — reusing the existing `PATCH …/activities/{id}/content`, `POST /questions`
(+ versions), `POST /assessments`, and task submission/review — all
**provenance-gated and owner-scoped**. Structure is **field-templated at
bootstrap**; free structural composition (agent adds new chapters) is **deferred
to the adaptation milestone (M4/M5)**, not built now.

## Borrow vs. reject (Coursera / edX / Udemy)

- **Borrow**: typed-component content model, practice-vs-graded distinction,
  rubric + peer/manual review, mastery-per-objective, the module spine, and a
  subject catalog built on a shared commons.
- **Reject / defer**: video hosting, marketplace/payments, forums/social,
  certificates/accreditation, scheduled cohort deadlines. (Already de-scoped in
  the roadmap.)
- **Invert**: their human/instructor authoring → AME's **agent authoring** on
  the capability platform.

## Milestone re-mapping (to validate in the revise step)

- **M0 golden stories**: keep Flink (CS) **and** add a non-CS field (music or
  physics) to prove *commons + specialization*, not just "not hardcoded". Seed
  both deterministically.
- **M1 versioned curriculum**: the commons spine. Re-examine — is the delivered
  2-chapter template the intended commons, or too thin for a course?
- **M2 rich activities**: the typed capability set. Re-examine — do the
  field-specialized grading types (code / numeric / audio / rubric) exist, or
  only explanation/example?
- **M3 assessment/task/evidence**: re-examine graded / pending / rubric coverage
  per field.
- **M4 agent ops**: the capability/authoring surface + provenance. Re-examine
  how far agent authoring actually reaches.
- **M5 course-quality UX**: re-examine against the Coursera-shaped, multi-chapter,
  per-field experience.

## Decisions (2026-07-23)

1. **Second field = physics** (not music). It proves cross-field generality
   while reusing the objective-grading commons (numeric/short-answer + MCQ),
   with minimal new machinery. **Music is deferred** as the later field that
   validates the subjective-grading + media path.
2. **Keep format as a lightweight emphasis modifier**, not an N×M template
   matrix. Field picks the specialized activity/grading types + content; format
   tunes the activity mix over the same spine (exam-prep → more assessments;
   project → more tasks; understand → more explanation/example).
3. **Free structural composition is deferred to M4/M5** — no `create_chapter` /
   `create_activity` HTTP endpoints now; structure comes from field templates.
4. **Grading build required for the golden fields** — *revised after the edX
   research (below)*. The earlier assumption was wrong: `acceptedAnswers` is
   string-exact and does **not** grade physics numeric (`9.3e7 != 93000000`) nor
   real CS code. Minimal set to build:
   - **numeric-answer with tolerance/range** (physics-blocking; pure-Rust, cheap);
   - **sandboxed code runner** (CS-blocking; exemplar exact-match can't grade real
     programs — adopt edX's external-grader shape);
   - **structured rubric object** (upgrades manual task review; enables later peer).
   Defer formula/symbolic equivalence and audio grading.

## edX capability gaps & minimal borrow set (from 2026-07-23 research)

Open edX confirms the model: a **universal course spine + grading policy** that
is identical across every subject, with differentiation living almost entirely
in the **problem/component layer**. Sources: Open edX docs (course structure,
problem types, numerical-input tolerance, grading policy, ORA, external graders).

Hierarchy analog: edX `Course → Section → Subsection → Unit → Component` maps to
our `Journey → Chapter → (—) → Activity → Question/Task`. **Divergence, not a
gap:** edX's grading unit is the *subsection* and it rolls up to a *course
grade*; we have no subsection level and roll up to *per-objective mastery*
instead. Our generation-run provenance + publish-gating has **no edX analog** —
it's our differentiator.

**Have:** MCQ (bare-string options), true/false, short-answer (exact/trimmed),
essay + task submissions, self-review, manual/agent review, practice-vs-graded
mode, per-item points, per-assessment passing score.

**Lack (matters):**

- numeric-answer with tolerance/range — **physics-blocking**;
- sandboxed code execution — **CS-blocking** (only exemplar exact-match today);
- structured rubric object (criteria × scored options);
- multi-select + per-option partial credit; formula/symbolic equivalence; ORA
  peer assessment.

**Minimal borrow set (priority):**

1. numeric-answer type with tolerance/range (pure-Rust grader, cheap) — physics.
2. code grading via a sandboxed external runner (edX XQueue/xqueue-watcher shape)
   feeding `tb_task_submissions.evaluation_method='automatic'` — CS.
3. first-class rubric object reused by manual review now, peer review later.
4. formula/symbolic-equivalence — defer unless the physics journey needs it.

**Do NOT borrow now** (no real-user break under our model): weighted
assignment-type policy + drop-lowest, ORA peer assessment, showanswer/seed
authoring knobs, course-level pass/fail certification. Our per-objective
evidence/mastery roll-up replaces edX's weighted course-grade machinery.
