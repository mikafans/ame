# Prompt-grounded onboarding

AME's first journey now keeps the template catalog as the source of copy and
renders the learner's normalized goal into each generated journey. The
renderer handles objective statements, first-activity title/context/
instructions/questions, and follow-up activity titles and payloads.

The catalog uses the data placeholder `{{goal}}`; it does not contain a Flink
branch or any other topic-specific condition. The goal is substituted as data,
including when it contains `{{goal}}` or HTML-like text. It is not evaluated as
template code and is later escaped by the browser renderer.

Evidence:

- two onboarding unit tests cover a concrete Flink prompt and an odd prompt
  containing a placeholder plus HTML-like text;
- `make check` passed: 67 Rust tests, 24 frontend tests, generated schema
  parity, and formatting/lint gates;
- `make test-api` passed all 5 black-box API tests;
- `make local-uiux` passed all 4 browser behavior tests against the restarted
  local stack, including the assertion that the first activity title contains
  the submitted prompt.
