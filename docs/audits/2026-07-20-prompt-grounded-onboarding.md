# Prompt-grounded onboarding

AME's first journey now keeps the template catalog as the source of copy and
renders a readable topic label into each generated journey. The exact learner
sentence remains `rawIntent`; `normalizedStatement` is the display label used
for objective statements, first-activity title/context/instructions/questions,
and follow-up activity titles and payloads.

The catalog uses the data placeholder `{{goal}}`; it does not contain a Flink
branch or any other topic-specific condition. Common request prefixes such as
“I would like to learn” are removed only for the display label. The label is
substituted as data, including when it contains `{{goal}}` or HTML-like text.
It is not evaluated as template code and is later escaped by the browser
renderer.

Evidence:

- onboarding tests cover raw-intent preservation, readable labels for natural
  language requests, a concrete Flink prompt, and an odd prompt containing a
  placeholder plus HTML-like text;
- `make check` passed: 67 Rust tests, 24 frontend tests, generated schema
  parity, and formatting/lint gates;
- `make test-api` passed all 5 black-box API tests;
- `make local-uiux` passed all 4 browser behavior tests against the rebuilt
  local stack, including readable first-activity copy and visible original
  intent.
