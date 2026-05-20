# Plan 8: Agent Skills

**Goal:** Establish the `agents/` folder with `SKILL.md` bundles that teach external agents (like Claude) how to interact with the system via the API.

**Prerequisites:** API and OpenAPI spec finalized.

---

## Task 1: Generate Questions Skill

- [ ] **Step 1: `agents/generate-questions/SKILL.md`**
  Write a markdown file dictating the process for an agent to:
  1. Fetch `/me/weakest-tags`.
  2. Generate high-quality questions for those tags.
  3. POST batches to `/questions` using an `agent:write-questions` token.
  Include exact JSON examples and instructions not to guess schemas.

## Task 2: Analyze Performance Skill

- [ ] **Step 1: `agents/analyze-performance/SKILL.md`**
  Write instructions for an agent to:
  1. Read `/tags/:name/stats` and `/me/recent-attempts`.
  2. Synthesize qualitative feedback for the user on why they are failing certain tags.

## Task 3: Adaptive Generation Skill

- [ ] **Step 1: `agents/adaptive-generation/SKILL.md`**
  Write an advanced loop combining both:
  Read weakest tags -> check exam pool insufficiency errors -> generate targeted questions to plug gaps in dynamic blueprints.

## Task 4: Final Polish

- [ ] **Step 1: Documentation linkage**
  Ensure `AGENTS.md` and `README.md` point clearly to the `agents/` folder as the definitive source for agent operations.
- [ ] **Step 2: Session Memory Update**
  Clear `.agents/CURRENT_TASK.md`, marking the implementation phase complete!
