import os
import uuid

BASE_URL = os.getenv("AME_API_URL", "http://localhost:8080")

def test_register_creates_human_user(client):
    email = f"pytest-human-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Human User",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    body = resp.json()
    assert "token" in body
    assert body["user"]["email"] == email


def test_register_rejects_admin_role(client):
    email = f"pytest-human-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Fake Admin",
        "password": "password123",
        "role": "admin"
    })
    assert resp.status_code == 422


def test_create_agent_success(client):
    # Register human
    email = f"pytest-human-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Human Creator",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # Create agent
    resp = client.post("/v1/me/agents", headers=headers, json={
        "label": "my-agent",
        "scopes": ["assessment.read"]
    })
    assert resp.status_code == 201
    body = resp.json()
    assert "apiKey" in body
    assert "id" in body


def test_create_agent_rejects_admin_scope(client):
    # Register human
    email = f"pytest-human-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Human Creator",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # Try to create agent with admin scope
    resp = client.post("/v1/me/agents", headers=headers, json={
        "label": "my-agent",
        "scopes": ["assessment.read", "admin"]
    })
    assert resp.status_code == 403


def test_create_agent_rejects_empty_scopes(client):
    # Register human
    email = f"pytest-human-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Human Creator",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # Try to create agent with empty scopes
    resp = client.post("/v1/me/agents", headers=headers, json={
        "label": "my-agent",
        "scopes": []
    })
    assert resp.status_code == 422


def test_create_agent_rejects_unknown_scope(client):
    # Register human
    email = f"pytest-human-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Human Creator",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}

    # Try to create agent with unknown scope
    resp = client.post("/v1/me/agents", headers=headers, json={
        "label": "my-agent",
        "scopes": ["not-a-real-scope"]
    })
    assert resp.status_code == 422


def test_unauthenticated_returns_401():
    import httpx
    with httpx.Client(base_url=BASE_URL) as fresh_client:
        resp = fresh_client.get("/v1/me")
        assert resp.status_code == 401


def test_bad_token_returns_401():
    import httpx
    with httpx.Client(base_url=BASE_URL) as fresh_client:
        resp = fresh_client.get("/v1/me", headers={"Authorization": "Bearer totally-fake"})
        assert resp.status_code == 401
