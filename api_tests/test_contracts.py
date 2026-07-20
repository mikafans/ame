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
