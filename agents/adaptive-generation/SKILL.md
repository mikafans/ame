# adaptive-generation

Adaptive loop: read weakest tags → check exam pool gaps → generate targeted questions.
Handles `pool_insufficient` gracefully by reducing question count and retrying.

## Prerequisites

- API key with scopes: `quiz.read`, `quiz.write`, `stats.read`
- Base URL: your AME instance (e.g. `https://ame.example.com/v1`)

## Loop

```
1. GET /v1/quizzes/{quizId}/stats
   → collect items where correctRate < 0.5

2. For each weak tag:
   a. POST /v1/quizzes/generate  (source = tag description, questionCount = 5)
      → on 422 pool_insufficient: retry with questionCount - 1 (min 1)
      → on warnings[]: log but continue
   b. PATCH /v1/quizzes/{quizId}  { status: "active" }

3. POST /v1/shares  { kind: "quiz", id: quizId }
   → surface embedUrl for instructor review
```

## Example: generate with retry

```http
POST /v1/quizzes/generate
Authorization: Bearer <your-key>
Content-Type: application/json

{
  "source": "Rust ownership and borrowing rules: move semantics, references, lifetimes.",
  "questionCount": 5
}
```

On `422 pool_insufficient`:

```json
{
  "error": {
    "code": "exam_pool_insufficient",
    "details": { "section": "...", "required": 5, "available": 3 }
  }
}
```

Retry with `questionCount: 3`.

## Sharing generated quizzes

```http
POST /v1/shares
Authorization: Bearer <your-key>
Content-Type: application/json

{
  "kind": "quiz",
  "id": "019...",
  "visibility": "public",
  "includeExplanation": true
}
```

Response includes `embedUrl` — paste into a course page or LMS for anonymous preview.

## Tool discovery

The full tool list is available at:

```http
GET /v1/agents/skill.json
```

Use `?strict=1` to drop the ame-specific fields (`method`, `path`, `scope`).
