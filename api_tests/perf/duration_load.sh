#!/usr/bin/env bash
#
# Duration (Soak) Load test using oha
#
# Usage:
#   ./api_tests/perf/duration_load.sh [DURATION] [BASE_URL]
#
# Example (runs for 2 minutes):
#   ./api_tests/perf/duration_load.sh 2m http://localhost:28080

set -euo pipefail

# Raise file descriptor limits for concurrent load testing sockets
ulimit -n 65535 2>/dev/null || ulimit -n 4096 2>/dev/null || true

DURATION="${1:-1m}"
BASE_URL="${2:-http://localhost:28080}"

echo "============================================="
echo "Starting Duration/Soak Load Test"
echo "Duration: ${DURATION}"
echo "Target Base URL: ${BASE_URL}"
echo "============================================="

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
    "title": "Duration Soak Assessment",
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
        "tags": ["soak-perf"],
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

# 3. Create initial session for answering load test
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
echo "Environment setup complete."
echo "Running oha load tests..."

echo ""
echo "--------------------------------------------------------------------------"
echo "Soak Phase 1: Authenticated reads (/v1/me) for ${DURATION}..."
echo "--------------------------------------------------------------------------"
  oha -z "${DURATION}" -c 30 -q 200 \
    -H "Authorization: Bearer ${TOKEN}" \
    "${BASE_URL}/v1/me"

  echo ""
  echo "--------------------------------------------------------------------------"
  echo "Soak Phase 2: Session creation (/v1/sessions) for ${DURATION}..."
  echo "--------------------------------------------------------------------------"
  oha -z "${DURATION}" -c 15 -q 200 -m POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${TOKEN}" \
    -d "{\"assessmentId\":\"${ASSESSMENT_ID}\"}" \
    "${BASE_URL}/v1/sessions"

  echo ""
  echo "--------------------------------------------------------------------------"
  echo "Soak Phase 3: Answering (/v1/sessions/{id}/answer) for ${DURATION}..."
  echo "--------------------------------------------------------------------------"
  oha -z "${DURATION}" -c 15 -q 200 -m POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${TOKEN}" \
    -d "{\"questionId\":\"${QUESTION_ID}\",\"response\":{\"selected_position\":1}}" \
    "${BASE_URL}/v1/sessions/${SESSION_ID}/answer"

  echo "Soak test finished successfully."
