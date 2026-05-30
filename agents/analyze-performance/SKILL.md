# analyze-performance

Read a learner's tag-level stats and recent attempt history to surface learning gaps.

## Prerequisites

- API key with scopes: `stats.read`, `attempt.read`
- Base URL: your AME instance (e.g. `https://ame.example.com/v1`)

## Steps

### 1. Get per-tag stats for a quiz

```http
GET /v1/quizzes/{quizId}/stats
Authorization: Bearer <your-key>
```

Response:

```json
{
  "avg": 0.72,
  "median": 0.75,
  "distribution": [
    { "bucket": "0-20", "count": 2 },
    { "bucket": "80-100", "count": 14 }
  ],
  "items": [
    {
      "questionId": "019...",
      "correctRate": 0.31,
      "avgTimeMs": 14200,
      "discriminationIdx": 0.55
    }
  ]
}
```

Items with `correctRate < 0.5` are struggle points.

### 2. Get exam-level stats

```http
GET /v1/exams/{examId}/stats
Authorization: Bearer <your-key>
```

Response includes `passRate`, `sectionAvgs[]`, `timeP50`, `timeP95`.

### 3. Check agent activity log

```http
GET /v1/agents/activity?limit=20
Authorization: Bearer <your-key>
```

Review recent tool calls, status codes, and paths to audit what the agent has done.

## Interpreting results

| Metric | Threshold | Action |
|--------|-----------|--------|
| `correctRate` | < 0.5 | Tag needs more practice questions |
| `discriminationIdx` | < 0.2 | Question may be poorly written |
| `avgTimeMs` | > 30 000 | Question may be too complex |
| `sectionAvgs` below pass rate | — | Focus exam prep on that section |
