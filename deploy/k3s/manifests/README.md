> **Mirror, not canonical.** This directory is a reference copy of the
> manifests actually running this deployment. The source of truth lives in a
> private ops repo (`harus-k3s/06-edge/ame/`) — changes here don't deploy
> anything on their own; they're here so anyone working on `ame` can see the
> real deployment shape (routes, config, secrets layout) without needing
> access to that repo.

# AME — agent-driven assessment platform

Self-contained deployment of [AME](https://github.com/mikafans/ame) for **alpha
access**. Runs on `harus-mini` in the **`harus-edge` namespace** (a low-burst
side project) — it reuses none of the cluster's shared infra (own Postgres, own
Valkey).

> Posture: public via Cloudflare Tunnel (HTTPS edge → in-cluster HTTP). The app
> runs with `production = true` (Secure cookies + per-account login throttle) and
> `trusted_proxies = 2` (real client IP through the CF→cloudflared→nginx chain).
> Security baseline = AME PR #20 (admin-demote token revoke, trusted-proxy IP,
> login throttle, etc.). Registration is open (no email verification) — gate with
> Cloudflare Access if you want invite-only alpha.

## Deploy (quickstart)

From zero, run on **harus-mini** (`kubectl`/`kubeseal`/`podman` configured). Each
step is detailed in [One-time setup](#one-time-setup) below.

```bash
# 0. (once) images — built + imported straight into containerd as
#    azusachino.icu/ame-{api,web}:<version> (imagePullPolicy: Never, no registry).
#    The build lives in the ame repo (mikafans/ame); from an ame checkout on
#    harus-mini run `deploy/k3s/build-images.sh`. This repo only patches the
#    image: tag below.

# 1. seal secrets WITH the admin password (the bootstrap Job won't start without
#    the ADMIN_PASSWORD key), then commit the sealed *.yaml outputs.
read -s ADMIN_PW                                  # paste, Enter — keeps it out of shell history
PG=$(openssl rand -hex 24)
kubectl create secret generic ame-secrets -n harus-edge \
  --from-literal=POSTGRES_PASSWORD="$PG" \
  --from-literal=AME_AGENT_ACCESS_CODE="" \
  --from-literal=ADMIN_PASSWORD="$ADMIN_PW" \
  --dry-run=client -o yaml | kubeseal --format yaml > ame-secrets-sealedsecret.yaml

# 2. namespace FIRST (apply-edge applies files in unspecified order), then the rest
kubectl apply -f namespace.yaml
make apply-edge                                   # from repo root; applies all of 06-edge

# 3. watch it come up
kubectl -n harus-edge rollout status deploy/ame-platform deploy/ame-api deploy/ame-web
kubectl -n harus-edge logs job/ame-bootstrap-admin       # expect: [bootstrap-admin] ... is admin
```

Then reach the platform over Tailscale at the **single front-door origin**
`http://ame-platform.harus-edge.svc.cluster.local` and log in as `azusachino@proton.me`.
Updating a running deployment → [Operations](#operations).

## Topology

| Component        | Kind        | Access (over Tailscale)                        |
| ---------------- | ----------- | ---------------------------------------------- |
| **ame-platform** | Deployment  | **<http://ame-platform.harus-edge.svc.cluster.local>**  |
| ame-web          | Deployment  | internal — behind ame-platform (`/`)           |
| ame-api          | Deployment  | internal — behind ame-platform (`/api/`, `/public/`) |
| ame-postgres     | StatefulSet | in-namespace `ame-postgres:5432` (2Gi PVC)     |
| ame-valkey       | Deployment  | in-namespace `ame-valkey:6379`                 |

Public access is via a Cloudflare Tunnel hostname (`ame.azusachino.icu`) that
routes to `ame-platform.harus-edge.svc.cluster.local:80` (HTTP, No TLS Verify) — the CF
edge terminates HTTPS, the in-cluster hop stays HTTP. The same ClusterIP is also
reachable over Tailscale (the subnet router advertises the `10.43.0.0/16` service
CIDR and split DNS resolves `cluster.local`), but with `production = true` the
Secure session cookie only rides the HTTPS hostname — log in via `https://ame.azusachino.icu`.
No NodePort.

**Everything is one origin.** `ame-platform` (nginx, `front.yaml`) proxies
`/api/*` and `/public/*` to ame-api and everything else to ame-web. Authenticated
endpoints are `/api/v1/*`; unauthenticated endpoints are `/public/v1/*`; the old
`/v1/*` surface is not supported. Discovery documents are served from
`/public/llms.txt`, `/public/skill.json`, `/public/openapi.yaml`, and
`/public/learning-contract.json`. This is required, not cosmetic: the API
sets its session cookie `SameSite=Lax` over plain HTTP, so if the browser talked to
ame-web and ame-api as *different* hosts the cookie wouldn't ride the API calls and
login wouldn't stick. Behind the front door the browser is same-origin, so the
cookie is first-party and CORS never triggers. Consequently the web image is built
with **`NEXT_PUBLIC_API_URL=""`** (relative API calls) — it hardcodes no host, so
you can front `ame-platform` with any tailnet/public name later without a rebuild.
`cors_origins` in `ame.toml` is just a backstop pointed at the front door.

## Config (why ame.toml is mounted)

The API **requires** `ame.toml`: rate-limit tiers, quotas and batch sizes have no
env override and no built-in default that survives env injection. It's shipped via
the `ame-config` ConfigMap, mounted at `/etc/ame/ame.toml` (`AME_CONFIG_PATH`).
Secrets and the DB URL are injected as env on top (env overrides only apply when
the TOML loads successfully). Note the DB url env var is `AME_DATABASE_URL` (the
`AME_` prefix is required by `config.rs`).

`trusted_proxies = 2` — the Cloudflare Tunnel and ame-platform front door are
trusted L7 hops. nginx preserves the forwarding chain (see `front.yaml`), so the
API strips the two right-most hops to get the real client for per-IP rate
limiting; any client-supplied XFF stays left of the trusted hops and can't spoof
it.

## One-time setup

### 1. Build & import the images (local-only, no registry)

The build lives in the **ame repo** (`mikafans/ame`), under `deploy/k3s/` — its
build context is the ame source, and this repo only declares which image tag to
run. `Dockerfile.api` (musl → distroless) and `Dockerfile.web` (node-slim) build
natively on harus-mini's amd64 — no cross-arch tooling needed since the cluster
is single-node. Run on harus-mini:

```bash
# from an ame checkout:
deploy/k3s/build-images.sh   # builds + imports azusachino.icu/ame-{api,web}:<version> into containerd
```

The web image bakes `NEXT_PUBLIC_API_URL=""` (relative API calls) so it's served
same-origin behind the ame-platform front door — see [Topology](#topology). nginx
itself is the stock `mirror.gcr.io/library/nginx` image (no build/import needed).

### 2. Create the SealedSecrets (encrypted to this cluster)

```bash
# DB password + agent access code (empty = agent registration disabled)
# + root admin password (consumed once by the ame-bootstrap-admin Job).
PG=$(openssl rand -hex 24)
kubectl create secret generic ame-secrets -n harus-edge \
  --from-literal=POSTGRES_PASSWORD="$PG" \
  --from-literal=AME_AGENT_ACCESS_CODE="" \
  --from-literal=ADMIN_PASSWORD="$ADMIN_PW" \
  --dry-run=client -o yaml | kubeseal --format yaml > ame-secrets-sealedsecret.yaml
```

See the `*.template` file for details. The sealed `*.yaml` output is safe to
commit (decryptable only by this cluster's controller).

### 3. Deploy

```bash
make apply-edge           # applies all of 07-ame
kubectl -n harus-edge get pods -w
```

### 4. Seed the first admin

Registration always creates `role: user` (no self-escalation), so the first admin
is written to the DB out of band — the cluster equivalent of `make db-admin`. The
`ame-bootstrap-admin` Job argon2-hashes `ADMIN_PASSWORD` (from `ame-secrets`) and
upserts a `role=admin` row for `azusachino@proton.me`:

```bash
kubectl apply -f bootstrap-admin.yaml          # included in `make apply-edge`
kubectl -n harus-edge logs job/ame-bootstrap-admin    # expect: [bootstrap-admin] ... is admin
```

Idempotent: if that email already registered, the Job only flips its role and
keeps the account's own password; only a brand-new row uses `ADMIN_PASSWORD`.
It's a **Job, not an initContainer** on purpose — the upsert re-promotes on every
run, so an initContainer would re-promote on every API restart and you could never
demote the account in-app. To re-run after editing:

```bash
kubectl -n harus-edge delete job ame-bootstrap-admin && kubectl apply -f bootstrap-admin.yaml
```

Re-login afterwards — token scopes are fixed at login.

## Operations

```bash
kubectl -n harus-edge logs deploy/ame-api -f
kubectl -n harus-edge rollout restart deploy/ame-api deploy/ame-web   # after pushing a new image tag
kubectl -n harus-edge get pods -o wide                                # confirm pods are Running
make delete-edge                                               # tear down (PVC/data persists)
```

Bumping the image: build+import a new version from the ame repo
(`deploy/k3s/build-images.sh`), then update `image:` in `api.yaml`/`web.yaml`
here and re-apply. Never use a floating tag.

## Notes / known gaps for alpha

- **Backups**: `ame-postgres` is backed up by K8up via the `harus-edge` `Schedule`
  + `pg_dump` backupcommand annotation (see `01-infrastructure/k8up/`).
- **Storage**: PVCs use `local-path` (node-local on `harus-mini`).
- HSTS from the app is now effective over the Cloudflare HTTPS edge.
