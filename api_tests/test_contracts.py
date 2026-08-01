import json
import uuid
from pathlib import Path


def test_operational_and_static_public_contracts(client):
    health = client.get("/healthz")
    assert health.status_code == 200
    assert health.json() == {"status": "ok"}

    readiness = client.get("/readyz")
    assert readiness.status_code == 200
    assert readiness.json()["postgres"] == "ok"

    llms = client.get("/public/llms.txt")
    assert llms.status_code == 200
    assert "POST /public/v1/auth/register" in llms.text
    assert "POST /public/v1/auth/login" in llms.text
    assert "POST /public/v1/onboarding/start" in llms.text

    skill = client.get("/public/skill.json")
    assert skill.status_code == 200
    assert skill.json()["entrypoint"] == "/public/llms.txt"

    openapi = client.get("/public/openapi.yaml")
    assert openapi.status_code == 200
    assert "/api/v1/learning/journeys:" in openapi.text


def test_public_registration_and_authenticated_api_namespace(client):
    email = f"contract-{uuid.uuid4()}@example.test"
    registration = client.post(
        "/public/v1/auth/register",
        json={"email": email, "name": "Contract Learner", "password": "password123"},
    )
    assert registration.status_code == 201, registration.text
    token = registration.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    profile = client.get("/api/v1/me", headers=headers)
    assert profile.status_code == 200, profile.text
    assert profile.json()["email"] == email
    assert profile.json()["role"] == "user"

    assert client.get("/v1/me", headers=headers).status_code != 200
    assert client.get("/public/v1/skill.json").status_code == 404


def test_active_docs_do_not_keep_retired_agent_surface():
    retired_docs = [
        "docs/audits/2026-05-28-principal-swe-audit.md",
        "docs/audits/2026-06-07-agent-surface-workflow-audit.md",
        "docs/plans/2026-06-03-admin-token-audit.md",
        "docs/plans/2026-06-14-identity-unification-plan.md",
        "docs/reports/2026-06-14-audit-report.md",
        "docs/specs/2026-05-19-question-exam-platform-design.md",
        "docs/specs/2026-05-20-harus-platform-design.md",
        "docs/specs/2026-05-25-role-simulation-design.md",
        "docs/specs/2026-05-28-flashcards-design.md",
        "docs/specs/2026-05-30-agent-identity-refactor.md",
        "docs/specs/2026-05-31-assessment-unification.md",
        "docs/specs/2026-06-08-agent-only-api-tokens.md",
        "docs/specs/2026-06-14-learn-deeper.md",
        "docs/specs/mcp-prompts.md",
    ]
    assert [path for path in retired_docs if Path(path).exists()] == []


def test_llms_entry_doc_lists_every_manifest_api_path():
    manifest = json.loads(Path("docs/public/skill.json").read_text())
    llms = Path("docs/public/llms.txt").read_text()
    missing = sorted(
        {
            tool["path"].split("?", 1)[0]
            for tool in manifest["tools"]
            if tool["path"].split("?", 1)[0] not in llms
        }
    )
    assert missing == []


def test_manifest_exposes_public_account_operations():
    manifest = json.loads(Path("docs/public/skill.json").read_text())
    tools = {(tool["name"], tool["method"], tool["path"]) for tool in manifest["tools"]}
    assert ("identity.register", "POST", "/public/v1/auth/register") in tools
    assert ("identity.login", "POST", "/public/v1/auth/login") in tools


def test_native_catalog_exposes_structured_reviewed_provenance(client):
    response = client.get("/public/v1/catalog/journeys")
    assert response.status_code == 200, response.text

    journeys = response.json()
    assert journeys
    for journey in journeys:
        assert journey["sourceSummary"]
        assert journey["sources"]
        for source in journey["sources"]:
            assert source["title"]
            assert source["url"].startswith("https://")
            assert source["license"]
        assert journey["contentReview"]["status"] == "reviewed"
        assert journey["contentReview"]["reviewedAt"]
        assert journey["contentReview"]["reviewer"]
