"""Contract tests for the reference Kubernetes public-document boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
INGRESS = (ROOT / "deploy/k8s/base/ingress.yaml").read_text()
WEB = (ROOT / "deploy/k8s/base/web.yaml").read_text()


def test_public_docs_are_served_by_the_api_image():
    assert "name: ame-public-docs" not in WEB
    assert "image: caddy:2.10-alpine" not in WEB


def test_one_public_prefix_routes_all_contracts_to_the_api():
    assert "path: /public" in INGRESS
    assert "name: ame-api" in INGRESS
