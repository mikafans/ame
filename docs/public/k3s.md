# k3s image build (harus-pi edge)

Build infrastructure for the container images the k3s deployment runs. Builds default to `linux/amd64` (x64); set `PLATFORM=linux/arm64` for the harus-pi edge. The **ame repo owns the build** (the build context is this source tree); the cluster repo [`harus-k3s/06-edge/ame/`](https://github.com/azusachino/harus-k3s) carries only the Kubernetes manifests and patches the image `TAG` to match what was pushed here.

## Files

| File | Purpose |
| --- | --- |
| `build-images.sh` | Build + push `docker.io/azusachino/ame-{api,web}:<TAG>` |
| `Dockerfile.api` | API image — cargo-zigbuild → static musl → distroless (target arch from `TARGETARCH`) |
| `Dockerfile.web` | web image — BUILDPLATFORM split → Next.js standalone on multi-arch node-slim |

Both Dockerfiles are `BUILDPLATFORM`-split: the heavy compile runs natively on amd64 and only the COPY-only runtime stage is the target arch, so **no qemu/binfmt** is needed. Run from `harus-mini` (amd64), with `podman login docker.io` as `azusachino`.

## Release a new version

```bash
# 1. Bump the package version first (the script derives TAG from it):
#      api/Cargo.toml  +  web/package.json
# 2. Build + push (x64 by default; TAG defaults to the api/Cargo.toml version):
../../deploy/k3s/build-images.sh
#    For the arm64 harus-pi edge image:
PLATFORM=linux/arm64 ../../deploy/k3s/build-images.sh
# 3. In harus-k3s, bump image: tags in 06-edge/ame/{api,web}.yaml to the new TAG,
#    then:  make apply-edge && kubectl -n ame rollout status deploy/ame-api deploy/ame-web
```

`TAG=0.2.1 ../../deploy/k3s/build-images.sh` pins explicitly (e.g. to re-push). The web image bakes `NEXT_PUBLIC_API_URL=""` (relative API calls) so it's served same-origin behind the `ame-platform` front door — see the k3s repo README for topology, secrets, and the bootstrap-admin Job.
