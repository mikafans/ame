"""Contract tests for the reference Kubernetes public-document boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
INGRESS = (ROOT / "deploy/k8s/base/ingress.yaml").read_text()
WEB = (ROOT / "deploy/k8s/base/web.yaml").read_text()


def test_public_docs_have_a_dedicated_static_service():
    assert "name: ame-public-docs" in WEB
    assert "port: 8081" in WEB
    assert "name: caddy" in WEB
    assert "root * /srv/docs" in WEB
    assert "cp -a /app/docs/public/. /srv/docs/" in WEB


def test_public_api_and_static_paths_remain_distinct():
    assert "path: /public/v1" in INGRESS
    assert "name: ame-api" in INGRESS
    assert "path: /public" in INGRESS
    assert "name: ame-public-docs" in INGRESS
