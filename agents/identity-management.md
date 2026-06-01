# Identity Management

Tools for agents to understand their own state and persist memory.

## Profile

### profile.get

Get the agent's profile including label, focus, and memory, PLUS the owner's shared truth (level/ratings).

**Request:**
```json
{
  "tool": "profile.get",
  "params": {}
}
```

**Response:**
```json
{
  "ok": true,
  "tool": "profile.get",
  "result": {
    "agent": {
      "label": "Agent Rust",
      "focusTags": ["rust", "async"],
      "currentGoal": "Improve learner's understanding of async lifetimes",
      "nextTarget": "Generate practice quiz for 'tokio'",
      "memory": { "last_run": "2026-05-30T13:00:00Z" }
    },
    "owner": {
      "id": "0194...",
      "ratings": [
        { "tag": "rust", "rating": 1550.5, "attempts": 10 }
      ]
    }
  }
}
```

## Memory

### memory.set

Overwrite the agent's freeform JSON memory store.

**Request:**
```json
{
  "tool": "memory.set",
  "params": {
    "memory": { "key": "value" }
  }
}
```

### memory.append

Top-level shallow merge a JSON object into the agent's existing memory.

**Request:**
```json
{
  "tool": "memory.append",
  "params": {
    "append": { "nested": { "key": 123 } }
  }
}
```

## Goals

### target.set

Update the agent's current goal or next specific target.

**Request:**
```json
{
  "tool": "target.set",
  "params": {
    "currentGoal": "Master async Rust",
    "nextTarget": "Read Tokio docs"
  }
}
```
