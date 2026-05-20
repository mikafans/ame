def test_list_quizzes(client, auth):
    resp = client.get("/v1/quizzes", headers=auth)
    assert resp.status_code == 200
    body = resp.json()
    assert "quizzes" in body
    assert "total" in body


def test_list_quizzes_all_statuses(client, auth):
    for status in ("draft", "active", "archived"):
        resp = client.get(f"/v1/quizzes?status={status}", headers=auth)
        assert resp.status_code == 200


def test_get_nonexistent_quiz(client, auth):
    resp = client.get("/v1/quizzes/00000000-0000-0000-0000-000000000000", headers=auth)
    assert resp.status_code == 404


def test_generate_stub(client, auth):
    resp = client.post("/v1/quizzes/generate", headers=auth, json={
        "source": "test content",
        "questionCount": 3,
    })
    assert resp.status_code == 200
    body = resp.json()
    assert "candidates" in body
    assert "warnings" in body
