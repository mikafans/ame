import os
import uuid
import pytest
import httpx

BASE_URL = os.getenv("AME_API_URL", "http://localhost:8080")

def test_security_headers():
    with httpx.Client(base_url=BASE_URL) as client:
        resp = client.get("/healthz")
        assert resp.status_code == 200
        headers = resp.headers
        assert headers.get("X-Frame-Options") == "DENY"
        assert headers.get("X-Content-Type-Options") == "nosniff"
        assert headers.get("Referrer-Policy") == "strict-origin-when-cross-origin"
        assert headers.get("Content-Security-Policy") == "default-src 'none'; frame-ancestors 'none'"
        assert headers.get("Strict-Transport-Security") == "max-age=31536000; includeSubDomains; preload"


def test_admin_route_guards_reject_non_admin(client):
    # Register a normal user
    email = f"pytest-sec-user-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Sec User",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    token = resp.json()["token"]
    
    # Attempt to access admin endpoint
    resp = client.get("/v1/admin/users", headers={"Authorization": f"Bearer {token}"})
    assert resp.status_code == 403


def test_token_revocation_on_demotion(client):
    # 1. Log in as seed admin
    admin_login = client.post("/v1/auth/login", json={
        "email": "admin@example.com",
        "password": "password123"
    })
    if admin_login.status_code != 200:
        pytest.skip("Seed admin account not available")
    
    admin_token = admin_login.json()["token"]
    admin_headers = {"Authorization": f"Bearer {admin_token}"}
    
    # 2. Register a new user
    email = f"pytest-demote-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Demote Target",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    target_id = resp.json()["user"]["id"]
    
    # 3. Promote target user to admin using seed admin
    resp = client.patch(
        f"/v1/admin/users/{target_id}",
        headers=admin_headers,
        json={"role": "admin"}
    )
    assert resp.status_code == 204
    
    # 4. Log in as the promoted user to get their token
    target_login = client.post("/v1/auth/login", json={
        "email": email,
        "password": "password123"
    })
    assert target_login.status_code == 200
    target_token = target_login.json()["token"]
    target_headers = {"Authorization": f"Bearer {target_token}"}
    
    # Verify target user can access admin endpoint now
    resp = client.get("/v1/admin/users", headers=target_headers)
    assert resp.status_code == 200
    
    # 5. Demote target user using seed admin
    resp = client.patch(
        f"/v1/admin/users/{target_id}",
        headers=admin_headers,
        json={"role": "user"}
    )
    assert resp.status_code == 204
    
    # 6. Verify target user's token is now revoked (returns 401)
    resp = client.get("/v1/admin/users", headers=target_headers)
    assert resp.status_code == 401


def test_agent_quota_limits(client):
    # Load config to check the quota limit dynamically
    limit = 1
    for config_path in ["ame.dev.toml", "ame.toml"]:
        if os.path.exists(config_path):
            with open(config_path, "rb") as f:
                import tomllib
                try:
                    config = tomllib.load(f)
                    limit = config.get("quota", {}).get("agents", {}).get("free", limit)
                    break
                except Exception:
                    pass
    if limit > 10:
        pytest.skip(f"Agent quota limit is configured to {limit} in dev, skipping quota enforcement test")

    # 1. Register a new free user
    email = f"pytest-quota-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Free User",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}
    
    # 2. Create the first agent (quota is 1 for free plan)
    resp = client.post("/v1/me/agents", headers=headers, json={
        "label": "agent-1",
        "scopes": ["assessment.read"]
    })
    assert resp.status_code == 201
    
    # 3. Try to create a second agent (should exceed quota and return 429)
    resp = client.post("/v1/me/agents", headers=headers, json={
        "label": "agent-2",
        "scopes": ["assessment.read"]
    })
    assert resp.status_code == 429
    assert resp.json()["code"] == "quota_exceeded"


def test_batch_size_limits(client):
    # 1. Register a new free user
    email = f"pytest-batch-{uuid.uuid4()}@example.com"
    resp = client.post("/v1/auth/register", json={
        "email": email,
        "name": "Free User",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    token = resp.json()["token"]
    headers = {"Authorization": f"Bearer {token}"}
    
    # 2. Create an agent with assessment.write scope
    resp = client.post("/v1/me/agents", headers=headers, json={
        "label": "batch-agent",
        "scopes": ["assessment.write"]
    })
    assert resp.status_code == 201
    agent_key = resp.json()["apiKey"]
    agent_headers = {"Authorization": f"Bearer {agent_key}"}
    
    # 3. Call tool assessment.batchCreate with 51 items (exceeds free batch limit of 50)
    items = [{"title": f"Assessment {i}"} for i in range(51)]
    resp = client.post("/v1/agents/run", headers=agent_headers, json={
        "tool": "assessment.batchCreate",
        "params": {
            "items": items
        }
    })
    assert resp.status_code == 422
    assert "exceeds the maximum of 50" in resp.text


def test_email_canonicalization(client):
    email_base = f"pytest-canonical-{uuid.uuid4()}"
    email_mixed = f"  {email_base}+subaddress@Example.Com  "
    
    # 1. Register with mixed/subaddressed email
    resp = client.post("/v1/auth/register", json={
        "email": email_mixed,
        "name": "Canonical User",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 201
    
    # 2. Verify duplicate registration is rejected with 422 for canonicalized equivalent
    resp = client.post("/v1/auth/register", json={
        "email": f"{email_base.lower()}@example.com",
        "name": "Duplicate User",
        "password": "password123",
        "role": "user"
    })
    assert resp.status_code == 422
    
    # 3. Log in using the simplified canonical email address
    resp = client.post("/v1/auth/login", json={
        "email": f"{email_base.lower()}@example.com",
        "password": "password123"
    })
    assert resp.status_code == 200
    assert "token" in resp.json()
