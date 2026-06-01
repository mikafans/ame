def test_register_creates_agent(client):
    resp = client.post("/v1/agents/register", json={
        "label": "test-agent",
        "scopes": ["assessment.read"],
    })
    assert resp.status_code == 201
    body = resp.json()
    assert "apiKey" in body
    assert "userId" in body


def test_register_rejects_admin_scope(client):
    resp = client.post("/v1/agents/register", json={
        "label": "bad",
        "scopes": ["admin"],
    })
    assert resp.status_code == 422


def test_register_rejects_empty_scopes(client):
    resp = client.post("/v1/agents/register", json={"label": "bad", "scopes": []})
    assert resp.status_code == 422


def test_register_rejects_unknown_scope(client):
    resp = client.post("/v1/agents/register", json={
        "label": "bad",
        "scopes": ["not-a-real-scope"],
    })
    assert resp.status_code == 422


def test_unauthenticated_returns_401(client):
    resp = client.get("/v1/me")
    assert resp.status_code == 401


def test_bad_token_returns_401(client):
    resp = client.get("/v1/me", headers={"Authorization": "Bearer totally-fake"})
    assert resp.status_code == 401
