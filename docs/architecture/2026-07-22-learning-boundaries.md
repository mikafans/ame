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

ame-platform-application
  Platform use cases, repository ports, in-memory implementations, and
  application-level validation.

ame-platform-postgres
  SQLx adapters, PostgreSQL row mapping, transactions, and migrations.

ame-api
  HTTP routes, authentication extraction, OpenAPI DTOs, and runtime
  composition.
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

The API re-exports the platform crates through compatibility module paths. This
keeps existing HTTP consumers stable while removing business logic and SQLx
adapters from the API crate.

## Rules

- Domain code does not import Axum, SQLx, PostgreSQL, Valkey, or provider SDKs.
- Application code depends on repository/service traits, not concrete storage.
- PostgreSQL code owns SQL, row mapping, and transaction boundaries only.
- HTTP code translates requests/responses and invokes application contracts; it
  does not implement learning state transitions.
- New curriculum capabilities must enter through domain/application contracts
  before being added to migrations or browser renderers.
- Each extraction or behavior milestone has its own conventional commit.

## Future growth

New bounded contexts may add dedicated domain/application crates when their
contracts become substantial. They must preserve the same dependency direction
and must not place SQLx or business state transitions back in `ame-api`.
