// k6 load test for the backend answer path + authenticated reads.
//
// Exercises the hottest request flow under concurrency: token verification
// (every request), practice-session creation, answer grading + Elo update,
// and the /v1/me read that fires on every page load.
//
// Setup (instructor) seeds one promoted MC question; each VU iteration runs as
// a learner. Requires the API running and demo users seeded (`make db-seed`).
//
//   make bench-load
//   BASE_URL=http://localhost:28080 k6 run api_tests/perf/answer_load.js

import http from "k6/http";
import { check, fail } from "k6";
import { Rate, Trend } from "k6/metrics";

const BASE_URL = __ENV.BASE_URL || "http://localhost:28080";
const TAG = "perf-load";

const errorRate = new Rate("app_errors");
const answerLatency = new Trend("answer_latency", true);

export const options = {
  scenarios: {
    answer_flow: {
      executor: "ramping-vus",
      startVUs: 1,
      stages: [
        { duration: "15s", target: 10 },
        { duration: "30s", target: 10 },
        { duration: "10s", target: 0 },
      ],
    },
  },
  thresholds: {
    http_req_failed: ["rate<0.01"],
    "http_req_duration{name:me}": ["p(95)<150"],
    "http_req_duration{name:answer}": ["p(95)<300"],
    app_errors: ["rate<0.01"],
  },
};

function login(email, password) {
  const res = http.post(
    `${BASE_URL}/v1/auth/login`,
    JSON.stringify({ email, password }),
    { headers: { "Content-Type": "application/json" } },
  );
  if (res.status !== 200) {
    fail(`login failed for ${email}: ${res.status} ${res.body}`);
  }
  return res.json("token");
}

function authHeaders(token) {
  return {
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
  };
}

export function setup() {
  const instructorToken = login("instructor@example.com", "password123");
  const learnerToken = login("learner@example.com", "password123");

  const create = http.post(
    `${BASE_URL}/v1/questions`,
    JSON.stringify({
      questions: [
        {
          kind: "mc",
          prompt: "Which planet is closest to the Sun?",
          payload: {
            options: ["Venus", "Mercury", "Earth", "Mars"],
            correct_index: 1,
          },
          tags: [TAG],
          points: 1,
        },
      ],
    }),
    authHeaders(instructorToken),
  );
  if (create.status !== 201) {
    fail(`question create failed: ${create.status} ${create.body}`);
  }
  const questionId = create.json("questions.0.id");

  const promote = http.post(
    `${BASE_URL}/v1/questions/${questionId}/promote`,
    null,
    authHeaders(instructorToken),
  );
  if (promote.status !== 200) {
    fail(`question promote failed: ${promote.status} ${promote.body}`);
  }

  return { learnerToken };
}

export default function (data) {
  const auth = authHeaders(data.learnerToken);

  const session = http.post(
    `${BASE_URL}/v1/sessions`,
    JSON.stringify({ tags: [TAG], count: 1 }),
    Object.assign({ tags: { name: "create_session" } }, auth),
  );
  const sessionOk = check(session, {
    "session created": (r) => r.status === 201,
  });
  errorRate.add(!sessionOk);
  if (!sessionOk) return;

  const sessionId = session.json("sessionId");
  const questions = session.json("questions") || [];

  if (questions.length > 0) {
    const answer = http.post(
      `${BASE_URL}/v1/sessions/${sessionId}/answer`,
      JSON.stringify({
        questionId: questions[0].id,
        response: { selected_position: 1 },
      }),
      Object.assign({ tags: { name: "answer" } }, auth),
    );
    answerLatency.add(answer.timings.duration);
    errorRate.add(
      !check(answer, { "answer graded": (r) => r.status === 200 }),
    );
  }

  const me = http.get(
    `${BASE_URL}/v1/me`,
    Object.assign({ tags: { name: "me" } }, auth),
  );
  errorRate.add(!check(me, { "me ok": (r) => r.status === 200 }));
}
