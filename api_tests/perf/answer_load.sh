#!/usr/bin/env bash
#
# Load test using oha (replacing k6)
#
# Usage:
#   ./api_tests/perf/answer_load.sh [BASE_URL]

set -euo pipefail

# Raise file descriptor limits for concurrent load testing sockets
ulimit -n 65535 2>/dev/null || ulimit -n 4096 2>/dev/null || true

BASE_URL="${1:-http://localhost:28080}"
echo "Target Base URL: ${BASE_URL}"

# 1. Login to get token
echo "Logging in..."
LOGIN_RES=$(curl -s -X POST "${BASE_URL}/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"ada@example.com","password":"password123"}')

TOKEN=$(echo "${LOGIN_RES}" | jq -r '.token')
if [ "${TOKEN}" = "null" ] || [ -z "${TOKEN}" ]; then
  echo "Login failed: ${LOGIN_RES}"
  exit 1
fi
echo "Authenticated successfully."

# 2. Create an active assessment with an MC question
echo "Creating active assessment..."
CREATE_ASS_RES=$(curl -s -X POST "${BASE_URL}/v1/assessments" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${TOKEN}" \
  -d '{
    "title": "Oha Load Assessment",
    "mode": "practice",
    "status": "active",
    "questions": [
      {
        "kind": "mc",
        "prompt": "Which planet is closest to the Sun?",
        "payload": {
          "options": ["Venus", "Mercury", "Earth", "Mars"],
          "correct_index": 1
        },
        "tags": ["oha-perf"],
        "points": 1
      }
    ]
  }')

ASSESSMENT_ID=$(echo "${CREATE_ASS_RES}" | jq -r '.id')
if [ "${ASSESSMENT_ID}" = "null" ] || [ -z "${ASSESSMENT_ID}" ]; then
  echo "Assessment creation failed: ${CREATE_ASS_RES}"
  exit 1
fi
echo "Assessment created: ${ASSESSMENT_ID}"

# 3. Create an initial session to get its sessionId/questionId for answering load test
echo "Creating initial session..."
SESSION_RES=$(curl -s -X POST "${BASE_URL}/v1/sessions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${TOKEN}" \
  -d "{\"assessmentId\":\"${ASSESSMENT_ID}\"}")

SESSION_ID=$(echo "${SESSION_RES}" | jq -r '.sessionId')
QUESTION_ID=$(echo "${SESSION_RES}" | jq -r '.questions[0].id')

if [ "${SESSION_ID}" = "null" ] || [ -z "${SESSION_ID}" ] || [ "${QUESTION_ID}" = "null" ] || [ -z "${QUESTION_ID}" ]; then
  echo "Session creation failed: ${SESSION_RES}"
  exit 1
fi
echo "Session created: ${SESSION_ID}, Question: ${QUESTION_ID}"

# 4. Run load tests with oha
DURATION="${2:-}"

if [ -n "${DURATION}" ]; then
  echo "Running duration load tests for ${DURATION}..."

  echo ""
  echo "=========================================================================="
  echo "1. Running oha load test on: /v1/me (GET) - Authenticated reads"
  echo "=========================================================================="
  oha -z "${DURATION}" -c 20 \
    -H "Authorization: Bearer ${TOKEN}" \
    "${BASE_URL}/v1/me"

  echo ""
  echo "=========================================================================="
  echo "2. Running oha load test on: /v1/sessions (POST) - Session Creation writes"
  echo "=========================================================================="
  oha -z "${DURATION}" -c 10 -m POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${TOKEN}" \
    -d "{\"assessmentId\":\"${ASSESSMENT_ID}\"}" \
    "${BASE_URL}/v1/sessions"

  echo ""
  echo "=========================================================================="
  echo "3. Running oha load test on: /v1/sessions/{id}/answer (POST) - Submitting answers"
  echo "=========================================================================="
  oha -z "${DURATION}" -c 10 -m POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${TOKEN}" \
    -d "{\"questionId\":\"${QUESTION_ID}\",\"response\":{\"selected_position\":1}}" \
    "${BASE_URL}/v1/sessions/${SESSION_ID}/answer"
else
  echo ""
  echo "=========================================================================="
  echo "1. Running oha load test on: /v1/me (GET) - Authenticated reads"
  echo "=========================================================================="
  oha -n 1000 -c 20 \
    -H "Authorization: Bearer ${TOKEN}" \
    "${BASE_URL}/v1/me"

  echo ""
  echo "=========================================================================="
  echo "2. Running oha load test on: /v1/sessions (POST) - Session Creation writes"
  echo "=========================================================================="
  oha -n 500 -c 10 -m POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${TOKEN}" \
    -d "{\"assessmentId\":\"${ASSESSMENT_ID}\"}" \
    "${BASE_URL}/v1/sessions"

  echo ""
  echo "=========================================================================="
  echo "3. Running oha load test on: /v1/sessions/{id}/answer (POST) - Submitting answers"
  echo "=========================================================================="
  oha -n 500 -c 10 -m POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${TOKEN}" \
    -d "{\"questionId\":\"${QUESTION_ID}\",\"response\":{\"selected_position\":1}}" \
    "${BASE_URL}/v1/sessions/${SESSION_ID}/answer"
fi
