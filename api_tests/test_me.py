def test_get_me(client, auth):
    resp = client.get("/v1/me", headers=auth)
    assert resp.status_code == 200
    body = resp.json()
    assert "id" in body
    assert body["role"] == "agent"


def test_list_keys(client, auth):
    resp = client.get("/v1/me/keys", headers=auth)
    assert resp.status_code == 200
    assert isinstance(resp.json()["keys"], list)


def test_create_and_revoke_key(client, auth):
    resp = client.post("/v1/me/keys", headers=auth, json={
        "label": "test-key",
        "scopes": ["quiz.read"],
    })
    assert resp.status_code == 201
    key_id = resp.json()["id"]

    resp = client.delete(f"/v1/me/keys/{key_id}", headers=auth)
    assert resp.status_code == 204


def test_create_and_delete_webhook(client, auth):
    resp = client.post("/v1/me/webhooks", headers=auth, json={
        "url": "https://example.com/hook",
        "events": ["attempt.graded"],
    })
    assert resp.status_code == 201
    body = resp.json()
    assert "id" in body
    wh_id = body["id"]

    resp = client.delete(f"/v1/me/webhooks/{wh_id}", headers=auth)
    assert resp.status_code == 204
