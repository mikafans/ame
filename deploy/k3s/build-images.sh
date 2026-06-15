#!/usr/bin/env bash
# Build the arm64 ame-api / ame-web images for harus-pi and push them to the
# Docker Hub repos the k3s deployments pull from (docker.io/azusachino/ame-*).
#
# This lives in the ame repo (the build context is the ame source). The k3s repo
# (harus-k3s/06-edge/ame/) only carries the k8s manifests and patches the image
# TAG to match what this script pushed.
#
# Uses BUILDPLATFORM-split Dockerfiles (Dockerfile.api / Dockerfile.web): the
# heavy compile runs natively on this amd64 box and only COPY-only runtime
# stages are arm64, so NO qemu/binfmt is needed. Run from harus-mini.
#
# Requires: podman logged in to docker.io as azusachino.
#   ./build-images.sh                 # TAG defaults to the api/Cargo.toml version
#   TAG=0.2.0 ./build-images.sh       # pin explicitly
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AME_REPO="${AME_REPO:-$(cd "$HERE/../.." && pwd)}"
# Tracks the ame package version (api/Cargo.toml) by default — AGENTS.md forbids
# floating tags. Override with TAG=... only to re-push an existing version.
TAG="${TAG:-$(grep -m1 '^version' "$AME_REPO/api/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')}"
PLATFORM=linux/arm64
REGISTRY=docker.io/azusachino
# Empty = relative API calls, so the web is served same-origin behind the
# ame-platform front door (front.yaml) and the session cookie stays first-party.
# Use `-` (not `:-`) so an explicitly empty NEXT_PUBLIC_API_URL is honored.
API_URL="${NEXT_PUBLIC_API_URL-}"

echo ">> Building $REGISTRY/ame-api:$TAG ($PLATFORM, cross via zigbuild)"
podman build --platform="$PLATFORM" \
	-f "$HERE/Dockerfile.api" \
	-t "$REGISTRY/ame-api:$TAG" \
	"$AME_REPO"

echo ">> Building $REGISTRY/ame-web:$TAG ($PLATFORM) NEXT_PUBLIC_API_URL=$API_URL"
podman build --platform="$PLATFORM" \
	-f "$HERE/Dockerfile.web" \
	--build-arg NEXT_PUBLIC_API_URL="$API_URL" \
	-t "$REGISTRY/ame-web:$TAG" \
	"$AME_REPO"

echo ">> Pushing"
podman push "$REGISTRY/ame-api:$TAG"
podman push "$REGISTRY/ame-web:$TAG"

echo ">> Done. Next: in harus-k3s, bump image: tags to $TAG in 06-edge/ame/{api,web}.yaml,"
echo ">>       then: make apply-edge ; kubectl -n ame rollout status deploy/ame-api deploy/ame-web"
