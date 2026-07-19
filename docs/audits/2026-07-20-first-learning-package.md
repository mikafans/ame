# First learning package

The C1 bootstrap now creates a five-step first package from the versioned
template registry:

1. starter check;
2. explanation;
3. worked example;
4. diagnostic/practice step;
5. recommendation.

The explanation and worked-example payloads use the same `{{goal}}` data
placeholder as the rest of onboarding. They are persisted as ordinary learner
activities, linked to objectives, and rendered by the journey page. No Flink
or other topic-specific branch was added.

Evidence:

- the onboarding contract asserts five activities, ordering, content types,
  objective links, and prompt rendering;
- the browser contract starts the first activity, completes the starter check,
  discovers both package activities through the API, starts the explanation,
  and renders its goal-specific heading/body;
- `make check` passed with 67 Rust tests and 24 frontend tests;
- `make local-uiux` passed all 4 browser behavior tests against the restarted
  local stack.

The package is intentionally a bounded orientation scaffold, not fabricated
subject knowledge. An external agent can use the unified question and deep-dive
contracts to add grounded content and assessment work for the learner's actual
topic.
