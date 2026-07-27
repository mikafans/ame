# Public-document delivery audit — 2026-07-20

## Contract

`docs/public` is the only source for unauthenticated machine-facing
documents. Every supported deployment serves those files directly at:

- `/public/llms.txt`
- `/public/skill.json`
- `/public/openapi.yaml`

`/public/v1/*` is a separate API namespace and must remain routed to the API.

## Deployment evidence

- Compose Caddy mounts `./docs` at `/srv/docs` and serves `/public/*` from the
  static directory before the web fallback.
- The reference Kubernetes web pod's init container copies `/app/docs/public`
  from the web image into a shared volume.
- The Kubernetes Caddy sidecar serves that volume on port `8081`.
- The `ame-public-docs` Service and ingress route `/public` to the sidecar;
  `/public/v1` continues to route to `ame-api`.
- `scripts/test_k8s_public_docs.py` verifies the routing and source contract.

The API has no runtime handlers for `llms.txt`, `skill.json`, or
`openapi.yaml`.
