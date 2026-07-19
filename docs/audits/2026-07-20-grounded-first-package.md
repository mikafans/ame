# Grounded first-package authoring

The first learning package remains registry-driven and topic-neutral, but an
external agent can now replace its explanation or worked-example scaffold with
reviewed content grounded in the learner's actual topic.

The unified owner-scoped operation is:

```text
PATCH /api/v1/learning/activities/{activity_id}/content
```

It requires a published `learning.activity.content.compose` generation run,
one or more source references, and `reviewStatus: "approved"`. The operation
accepts the existing `explanation` and `worked_example` payload shapes only
when the content type matches the activity kind. Prose fields and step/key
point lists are strict non-empty strings. Provenance is stored beside
`payload.content`. The operation rejects malformed content, empty sources,
unpublished or mismatched generation runs, cross-owner access, and rewrites
after the activity is completed.

This is deliberately payload-only: the clean 0.3 baseline needs no migration,
new identity type, agent scope, or separate resource model. The same activity
returned by the learner API is rendered by the browser, so the external agent
and the learner do not receive divergent content paths. Public machine-facing
documentation is generated into `docs/public` and served directly by Caddy;
this authoring route remains API-backed because it is authenticated and
owner-scoped.

Evidence:

- the API behavior contract covers happy, malformed, missing-source,
  unpublished, wrong-operation, cross-owner, completed-activity, kind-mismatch,
  non-string-field, empty-list, and unapproved-review paths;
- the browser contract authors an explanation through the API and renders the
  resulting heading and body in the learner journey;
- the Rust library suite passes 67 tests;
- the black-box API suite passes 6 tests after rebuilding the API
  container and restarting Caddy to refresh its service address;
- `make check` passes with 67 Rust tests and 24 frontend tests;
- `make test-api` passes all 6 black-box tests;
- `make local-uiux` passes all 4 browser behavior tests;
- the live Caddy origin serves `/public/llms.txt`, `/public/skill.json`, and
  `/public/openapi.yaml` directly while `/public/v1/*` remains API-backed.
