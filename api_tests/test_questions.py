import pytest

MC_QUESTION = {
    "kind": "mc",
    "prompt": "What is 2 + 2?",
    "payload": {
        "options": ["3", "4", "5", "6"],
        "correct_index": 1,
    },
    "tags": ["math"],
    "points": 1,
}


@pytest.fixture(scope="module")
def question_id(client, auth):
    resp = client.post("/v1/questions", headers=auth, json={"questions": [MC_QUESTION]})
    assert resp.status_code == 201, resp.text
    return resp.json()["questions"][0]["id"]


def test_create_question(client, auth):
    resp = client.post("/v1/questions", headers=auth, json={"questions": [MC_QUESTION]})
    assert resp.status_code == 201
    body = resp.json()
    assert len(body["questions"]) == 1
    q = body["questions"][0]
    assert q["kind"] == "mc"
    assert q["prompt"] == "What is 2 + 2?"
    assert q["status"] == "draft"


def test_list_questions(client, auth):
    resp = client.get("/v1/questions", headers=auth)
    assert resp.status_code == 200
    assert isinstance(resp.json()["questions"], list)


def test_get_question(client, auth, question_id):
    resp = client.get(f"/v1/questions/{question_id}", headers=auth)
    assert resp.status_code == 200
    assert resp.json()["id"] == question_id


def test_patch_question(client, auth, question_id):
    resp = client.patch(f"/v1/questions/{question_id}", headers=auth, json={
        "prompt": "What is 2 + 2? (updated)",
    })
    assert resp.status_code == 200
    assert resp.json()["prompt"] == "What is 2 + 2? (updated)"


def test_list_versions(client, auth, question_id):
    resp = client.get(f"/v1/questions/{question_id}/versions", headers=auth)
    assert resp.status_code == 200
    assert isinstance(resp.json(), list)


def test_promote_question(client, auth, question_id):
    resp = client.post(f"/v1/questions/{question_id}/promote", headers=auth)
    assert resp.status_code == 200
    assert resp.json()["status"] == "live"


def test_archive_question(client, auth, question_id):
    resp = client.post(f"/v1/questions/{question_id}/archive", headers=auth)
    assert resp.status_code == 200
    assert resp.json()["status"] == "archived"


def test_get_nonexistent_question(client, auth):
    resp = client.get("/v1/questions/00000000-0000-0000-0000-000000000000", headers=auth)
    assert resp.status_code == 404
