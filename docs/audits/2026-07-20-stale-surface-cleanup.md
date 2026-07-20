# 0.3 stale-surface cleanup

The clean-slate 0.3 contract is now the only active design surface. The
retired pre-0.3 documents describing agent accounts, agent tokens, coarse
scopes, the `/v1/agents/run` door, root discovery routes, or legacy product
resources were removed from active documentation paths.

The canonical references are:

- [`2026-07-19-agent-first-learning-rework.md`](../plans/2026-07-19-agent-first-learning-rework.md)
  for product stories, identity rules, API namespaces, and release gates;
- [`2026-07-19-learning-baseline-schema.md`](../specs/2026-07-19-learning-baseline-schema.md)
  for the clean database contract;
- [`docs/public`](../public/) for the generated machine-facing contract and
  self-host documentation.

The cleanup did not change runtime behavior, migrate data, or remove any
canonical 0.3 source. A black-box contract asserts that the retired files are
absent so they cannot silently re-enter the active documentation surface.
