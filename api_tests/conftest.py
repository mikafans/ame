import os
import pytest
import httpx

BASE_URL = os.getenv("AME_API_URL", "http://localhost:8080")

ALL_SCOPES = [
    "human",
    "assessment.read",
    "assessment.write",
    "attempt.read",
    "attempt.write",
    "stats.read",
    "feedback.write",
    "plan.read",
    "plan.write",
    "agent:write-questions",
    "agent:read-only",
]


@pytest.fixture(scope="session")
def client():
    with httpx.Client(base_url=BASE_URL, timeout=10.0) as c:
        yield c


@pytest.fixture(scope="session")
def api_key(client):
    resp = client.post("/v1/agents/register", json={"label": "pytest", "scopes": ALL_SCOPES})
    assert resp.status_code == 201, resp.text
    return resp.json()["apiKey"]


@pytest.fixture(scope="session")
def auth(api_key):
    return {"Authorization": f"Bearer {api_key}"}
