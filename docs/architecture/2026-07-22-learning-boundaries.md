# Learning architecture boundaries

Status: adopted before M1; AME version remains `0.3.0`

AME is moving from a single learning implementation inside the `api` crate to
explicit Cargo workspace boundaries. The goal is to let curriculum, agent, and
frontend capabilities grow without coupling business rules to Axum or SQLx.

## Workspace crates

```text
ame-learning-domain
  Pure learning entities, value/state types, validation, and errors.

ame-learning-application
  Learning repository ports, in-memory implementation, and application-level
  contract tests/use-case helpers.

ame-learning-postgres
  SQLx repository adapter, PostgreSQL row mapping, transactions, and adapter
  integration tests.

ame-api
  HTTP routes, authentication, OpenAPI DTOs, runtime composition, and the
  remaining platform adapters during the incremental extraction.
```

Dependency direction:

```text
ame-learning-domain
        ↑
ame-learning-application
        ↑
ame-learning-postgres
        ↑
ame-api  → HTTP/runtime composition
```

The API currently re-exports the learning crates through compatibility module
paths. This keeps assessment, progress, onboarding, and HTTP consumers stable
while the rest of the backend is extracted incrementally.

## Rules

- Domain code does not import Axum, SQLx, PostgreSQL, Valkey, or provider SDKs.
- Application code depends on repository/service traits, not concrete storage.
- PostgreSQL code owns SQL, row mapping, and transaction boundaries only.
- HTTP code translates requests/responses and invokes application contracts; it
  does not implement learning state transitions.
- New curriculum capabilities must enter through domain/application contracts
  before being added to migrations or browser renderers.
- Each extraction or behavior milestone has its own conventional commit.

## Follow-up extraction

The remaining platform modules can move through the same pattern after the
learning boundary is stable:

```text
ame-content-domain / ame-content-application
ame-agent-application
ame-persistence-postgres
ame-http
ame-server
```

This is intentionally incremental. The current task establishes the boundary
that M1 needs; it does not split every existing AME feature into a new crate at
once.
