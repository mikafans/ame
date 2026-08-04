#!/usr/bin/env bash
# Build the ame-api / ame-web images and import them straight into the k3s
# node's containerd — bypasses any registry, same as daphne/suzuran's
# `image-local` Makefile target.
#
# This lives in the ame repo (the build context is the ame source). The k3s
# repo (harus-k3s/06-edge/ame/) only carries the k8s manifests and patches the
# image TAG to match what this script imported.
#
# Run on harus-mini (single-node amd64 cluster — no cross-arch build needed).
#   ./build-images.sh             # TAG defaults to api/Cargo.toml version
#   TAG=0.4.1 ./build-images.sh   # pin the tag explicitly
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AME_REPO="${AME_REPO:-$(cd "$HERE/../.." && pwd)}"
# Tracks the ame package version (api/Cargo.toml) by default — never use a
# floating tag. Override with TAG=... only to re-import an existing version.
TAG="${TAG:-$(grep -m1 '^version' "$AME_REPO/api/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')}"
LOCAL_IMAGE=azusachino.icu
# Empty = relative API calls, so the web is served same-origin behind the
# ame-platform front door (front.yaml) and the session cookie stays first-party.
# Use `-` (not `:-`) so an explicitly empty NEXT_PUBLIC_API_URL is honored.
API_URL="${NEXT_PUBLIC_API_URL-}"

echo ">> Building $LOCAL_IMAGE/ame-api:$TAG"
podman build \
	-f "$HERE/Dockerfile.api" \
	-t "$LOCAL_IMAGE/ame-api:$TAG" \
	"$AME_REPO"

echo ">> Building $LOCAL_IMAGE/ame-web:$TAG (NEXT_PUBLIC_API_URL=$API_URL)"
podman build \
	-f "$HERE/Dockerfile.web" \
	--build-arg NEXT_PUBLIC_API_URL="$API_URL" \
	-t "$LOCAL_IMAGE/ame-web:$TAG" \
	"$AME_REPO"

echo ">> Importing into k3s containerd"
podman save "$LOCAL_IMAGE/ame-api:$TAG" | sudo k3s ctr images import -
podman save "$LOCAL_IMAGE/ame-web:$TAG" | sudo k3s ctr images import -

echo ">> Done. Next: in harus-k3s, bump image: tags to $TAG in 06-edge/ame/{api,web}.yaml,"
echo ">>       then: make apply-edge ; kubectl -n harus-edge rollout status deploy/ame-api deploy/ame-web"
