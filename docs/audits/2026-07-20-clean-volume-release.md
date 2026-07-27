# Clean-volume 0.3 release rehearsal

The recommended local stack was exercised from isolated disposable volumes on
2026-07-20. The normal `ame` volumes were not removed.

```text
podman compose -p ame-clean -f docker-compose.local.yml down --volumes --remove-orphans
podman compose -p ame-clean -f docker-compose.local.yml up -d --build
```

The isolated project started Postgres, Valkey, API, web, and containerized
Caddy. The following direct probes all returned HTTP 200:

```text
/healthz
/readyz
/public/llms.txt
/public/skill.json
/public/openapi.yaml
```

The clean API path then registered/logged in `haru@example.com`, started a
Flink learner journey, and read it back through `/api/v1/learning/journeys/{id}`.
The prompt was preserved as `goal.rawIntent`. A second canonical demo journey
was seeded because the existing returning-learner browser contract deliberately
asserts that fixture.

`make local-uiux` passed all four behavior tests against the isolated origin:

1. intent to first evidence-backed next step;
2. returning learner resume;
3. agent-provided assessment and deep dive;
4. manual-review assessment without false progress.

The public-document boundary was also verified: `docs/public` is served
directly by Caddy, while only `/public/v1/*` is API-backed. No Rust handler or
backend response path is needed for the static public files.
