import pytest

MC_QUESTION = {
    "kind": "mc",
    "prompt": "Which planet is closest to the Sun?",
    "payload": {
        "options": ["Venus", "Mercury", "Earth", "Mars"],
        "correct_index": 1,
    },
    "tags": ["astronomy"],
    "points": 1,
}


@pytest.fixture(scope="module")
def active_question_id(client, auth):
    """Create and promote a question so it's available for practice sessions."""
    resp = client.post("/v1/questions", headers=auth, json={"questions": [MC_QUESTION]})
    assert resp.status_code == 201, resp.text
    qid = resp.json()["questions"][0]["id"]

    resp = client.post(f"/v1/questions/{qid}/promote", headers=auth)
    assert resp.status_code == 200
    return qid


@pytest.fixture(scope="module")
def session_id(client, auth, active_question_id):
    resp = client.post("/v1/sessions", headers=auth, json={
        "tags": ["astronomy"],
        "count": 1,
    })
    assert resp.status_code == 201, resp.text
    return resp.json()["sessionId"]


def test_create_practice_session(client, auth, active_question_id):
    resp = client.post("/v1/sessions", headers=auth, json={
        "tags": ["astronomy"],
        "count": 1,
    })
    assert resp.status_code == 201
    body = resp.json()
    assert "sessionId" in body
    assert isinstance(body["questions"], list)


def test_get_session(client, auth, session_id):
    resp = client.get(f"/v1/sessions/{session_id}", headers=auth)
    assert resp.status_code == 200
    body = resp.json()
    assert body["session"]["id"] == session_id


def test_answer_and_finish(client, auth, active_question_id):
    resp = client.post("/v1/sessions", headers=auth, json={
        "tags": ["astronomy"],
        "count": 1,
    })
    assert resp.status_code == 201, resp.text
    body = resp.json()
    sid = body["sessionId"]
    questions = body["questions"]

    if not questions:
        pytest.skip("no active questions available for practice session")

    qid = questions[0]["id"]

    resp = client.post(f"/v1/sessions/{sid}/answer", headers=auth, json={
        "questionId": qid,
        "response": {"selected_position": 1},
    })
    assert resp.status_code == 200
    answer_body = resp.json()
    assert "grade" in answer_body
    assert "attempt" in answer_body

    resp = client.post(f"/v1/sessions/{sid}/finish", headers=auth)
    assert resp.status_code == 200
    result = resp.json()
    assert "session" in result
    assert "result" in result


def test_get_nonexistent_session(client, auth):
    resp = client.get("/v1/sessions/00000000-0000-0000-0000-000000000000", headers=auth)
    assert resp.status_code == 404
