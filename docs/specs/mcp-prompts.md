# MCP Role Prompts

Sample prompts for validating ame's MCP surface in a live Claude session.
Prerequisite: ame running at http://localhost:8080 with MCP configured.

## Setup

### Get an API key

`POST /v1/agents/register` is unauthenticated — call it directly to mint a key:

```bash
curl -s -X POST http://localhost:8080/v1/agents/register \
  -H "Content-Type: application/json" \
  -d '{"label":"my-agent","scopes":["quiz.read","quiz.write","stats.read","plan.read","plan.write"]}' \
  | jq -r '.key'
```

### Configure Claude

Add to Claude's MCP config:

```json
{
  "mcpServers": {
    "ame": {
      "url": "http://localhost:8080/v1/agents/mcp.json",
      "headers": { "Authorization": "Bearer <key-from-above>" }
    }
  }
}
```

## Learner prompt

> "List the available quizzes, start a session on the first active one, answer each question (pick a reasonable answer for each type), finish the session, and tell me my final score."

Expected: Claude calls `quiz.list` → `session.create` → `session.answer` × N → `session.finish` → `session.get` and reports score.

## Instructor prompt

> "Create a 3-question multiple-choice quiz on Rust ownership, add an explanation to each question, publish it, then show me its stats."

Expected: Claude calls `question.create` → `quiz.import` (or `quiz.update`) → `quiz.update` (status=active) → `stats.cohort`.

## Agent knowledge-base prompt

> "Import these 3 questions into the bank, promote them to live, then generate a 10-question quiz from this source text: 'Rust uses an ownership model to manage memory without a garbage collector. Each value has a single owner. When the owner goes out of scope, the value is dropped.' Finally, create a study plan with the goal: improve Rust memory management in 3 weeks."

Expected: Claude calls `question.create` → `question.promote` × 3 → `quiz.generate` → `plan.create`.

## Stats prompt

> "Fetch my personal learning stats for the last 12 weeks and summarize my progress."

Expected: Claude calls `stats.user` with `window=12w` and returns a human-readable summary.
