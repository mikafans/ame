import os
import pytest
import httpx

BASE_URL = os.getenv("AME_API_URL", "http://localhost:8080")

ALL_SCOPES = [
    "assessment.read",
    "assessment.write",
    "attempt.read",
    "attempt.write",
    "stats.read",
    "feedback.write",
    "plan.read",
    "plan.write",
]


@pytest.fixture(scope="session")
def client():
    with httpx.Client(base_url=BASE_URL, timeout=10.0) as c:
        yield c


@pytest.fixture(scope="session")
def api_key(client):
    import uuid
    # 1. Register a human user
    email = f"pytest-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Pytest User",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201, resp.text
    token = resp.json()["token"]

    # 2. Create an agent for that user
    resp = client.post(
        "/v1/me/agents",
        headers={"Authorization": f"Bearer {token}"},
        json={
            "label": "pytest-agent",
            "scopes": ALL_SCOPES
        }
    )
    assert resp.status_code == 201, resp.text
    return resp.json()["apiKey"]


@pytest.fixture(scope="session")
def auth(api_key):
    return {"Authorization": f"Bearer {api_key}"}
