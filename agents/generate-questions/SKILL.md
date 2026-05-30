# generate-questions

Generate targeted questions for a learner's weakest topics and import them as a quiz.

## Prerequisites

- API key with scopes: `quiz.read`, `quiz.write`, `stats.read`
- Base URL: your AME instance (e.g. `https://ame.example.com/v1`)

## Steps

### 1. Find weakest tags

```http
GET /v1/tags?weakest=true
Authorization: Bearer <your-key>
```

Or use the stats endpoint for a specific quiz:

```http
GET /v1/quizzes/{quizId}/stats
Authorization: Bearer <your-key>
```

Response includes `items[]` sorted by `correct_rate` ascending — the lowest are your targets.

### 2. Generate questions for the weakest tag

```http
POST /v1/quizzes/generate
Authorization: Bearer <your-key>
Content-Type: application/json

{
  "source": "Explain binary search trees, including insertion, deletion, and traversal algorithms.",
  "questionCount": 5,
  "types": ["mcq", "free_text"],
  "difficulty": "medium"
}
```

Response:

```json
{
  "quizId": "019...",
  "questions": [...],
  "objectives": [
    "Understand BST insertion rules",
    "Explain in-order traversal output",
    "Identify deletion cases (leaf, one child, two children)"
  ],
  "warnings": []
}
```

If `warnings` is non-empty, the source was too thin — add more context and retry.

### 3. Publish the quiz

```http
PATCH /v1/quizzes/{quizId}
Authorization: Bearer <your-key>
Content-Type: application/json

{ "status": "active" }
```

## Error handling

| Code | Meaning |
|------|---------|
| 422 `pool_insufficient` | Not enough questions available — lower `questionCount` |
| 422 `validation_failed` | Check `fields[]` for body errors |
| 403 `scope_required` | Token missing `quiz.write` scope |
