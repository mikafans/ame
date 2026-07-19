import uuid


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
