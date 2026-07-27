# OpenAPI and agent-manifest parity

The reviewed activity-content authoring operation is now present in every
generated machine contract:

```text
PATCH /api/v1/learning/activities/{activity_id}/content
```

The handler is registered in `ApiDoc`, and `make openapi` regenerated
`api/openapi.yaml`, `docs/public/openapi.yaml`, and the frontend TypeScript
schema. The machine manifest and `llms.txt` already advertised the same route.

The integration contract
`api/tests/agent_manifest.rs::advertised_activity_authoring_is_present_in_openapi`
asserts both the manifest advertisement and the generated OpenAPI path/method.
No runtime route, database migration, or separate agent interface was added.
