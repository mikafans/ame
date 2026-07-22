# M0 issue drafts

These drafts are ready to become GitHub issues after review. Creating the
issues is intentionally separate from this commit so the roadmap can be
adjusted locally without creating external project state prematurely.

## Parent: M0 — product contract and golden user stories

Labels: `roadmap`, `milestone:m0`, `product`, `content`, `verification`

### M0.1 — approve the learner experience contract

Define the browser-visible journey from intent through lesson, example, quiz,
task, feedback, recommendation, and resume. Include empty, failure,
review-pending, and interrupted-attempt states.

Done when the story contract and browser-state checklist are reviewed and
linked from the roadmap.

### M0.2 — approve the external-agent operating contract

Define the commands an authorized external agent may use to create, inspect,
revise, publish, assess, review, and recommend learning resources. Preserve
one learner-owned resource model and require provenance for generated content.

Done when agent operations have explicit inputs, outputs, ownership rules,
failure behavior, and non-goals.

### M0.3 — approve the Flink golden journey

Review the four-chapter Flink fixture, objective statements, capability types,
assessment mix, task prompts, evidence transitions, and adaptation rules.

Done when a learner can understand what the journey teaches and how knowledge
will be tested without reading implementation details.

### M0.4 — approve the topic-neutral acceptance fixture

Use the database-indexing fixture to prove that the journey model is not
hardcoded for Flink.

Done when the same structural contract works with different content,
objectives, assessment items, and task evaluation.

### M0.5 — map M0 stories to implementation and verification

Create the M1–M5 child issues from the issue-ready boundaries in the story
contract. Each issue must identify its one conventional commit boundary,
contract tests, and local-stack/browser evidence.

Done when every M0 acceptance criterion has an owning issue and no issue mixes
analytics, rich content, curriculum structure, and unrelated release work.
