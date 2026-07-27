# Agent session resume contract

The existing owner-scoped session read is now part of the public machine
contract:

```text
GET /api/v1/learning/sessions/{id}
```

This lets an external agent inspect or resume the same in-progress activity
session that the browser stores and reloads. The route remains the unified
learner API; no agent identity, scope, or second resource model was added.

The path is present in `skill.json`, `llms.txt`, and the generated OpenAPI
snapshot. The existing browser resume behavior and the full local-stack gates
continue to pass.
