# Screenshots — Harus Assessment Platform

These are reference captures of the design at 1280px width. Use them alongside the source files in `../source/` to verify what each screen should look like when implemented in your target stack.

> File sizes are small because the design uses a deliberately restrained palette and lots of empty space — this is the intended visual density. Do not "fill" the empty space with decoration.

| # | File | What it shows |
|---|---|---|
| 01 | `01-signup.png` | Marketing/auth split layout. Left: serif hero + 2x2 stat grid + compliance footer. Right: tabbed form (Create account / Sign in) with role picker and SSO/access-code fallbacks. The "Agent shortcut" callout below the form is the entry point for programmatic registration. |
| 02 | `02-library.png` | Library screen with the "Up next" hero card (CS 311 — Algorithms — Graph Traversal). The hero shows status tags, course/difficulty, three KV stats, primary CTA, and a Cohort context KV list on the right with recommended prep. **Learning objectives** appear inside the hero below the description. |
| 03 | `03-exams.png` | Exams screen — split list/detail. Left list shows 4 sample exams with status tags (Scheduled / Active / Draft) and a manual-vs-agent marker. Right detail panel shows the selected exam's full composition. |
| 04 | `04-quiz-setup.png` | "Compose your quiz" configurator. Numbered blocks: Session mode (Practice/Timed/Adaptive) → Categories (pill chips) → Tags (mono micro-chips) → Difficulty → Question types → Length → Source → Options. Right rail summarizes the session and shows the **agent-equivalent** `POST /v1/sessions` snippet. |
| 05 | `05-quiz-active-mc.png` | Active quiz, Question 1 of 5, multiple choice. Letter-badged options (A/B/C/D) in a stacked column. Header shows attempt + course + course title, a live timer, and Save & exit. Right rail: question palette grid, legend, integrity checks, allowed materials. |
| 06 | `06-quiz-active-code.png` | Active quiz, Question 4 of 5, code question. Editor box with file header (`solution.py · python 3.11`), Run tests / Reset buttons, mock test runner output band at the bottom. |
| 07 | `07-results.png` | Results review. Top row: donut score card (left, 1.3fr) + cohort distribution histogram (right). Below: per-item review list with status badges, per-item notes, points earned (right-aligned serif), Share + See solution actions per row. |
| 08 | `08-progress-dashboard.png` | Progress dashboard. 5-up top stat strip (avg / attempts / hours / streak / mastered topics) followed by Score trend (line chart) and By subject (horizontal bars). At `statsDepth: full` the screen also includes Cohort comparison, Item analysis (IRT scatter), Study plan, and Mastery map. |
| 09 | `09-author-studio.png` | Author studio — three-pane editor. Left: questions list. Middle: editor for the selected question (prompt, options with fillable correctness circles, points/tag/difficulty, explanation). Right rail: Distribution KV, Rubric autograding panel with mono criteria block, Recent activity feed. |
| 10 | `10-agent-api-keys.png` | Agent integration → API keys subsection. Left: list of 3 keys with masked prefixes, scope tags, Copy/Rotate/Revoke actions. Right: Authentication explainer with sample request CodeBlock and scope reference. |
| 11 | `11-agent-mcp-tools.png` | Agent integration → MCP tools subsection. Left: scrollable tool list with `quiz.import`, `quiz.generate`, `exam.compose`, `exam.get`, `exam.stats`, `attempt.get`, `stats.cohort`, `feedback.send`, `plan.create`. Right: selected tool detail (inputs/returns + curl example + MCP descriptor JSON). |
| 12 | `12-agent-import-demo.png` | Agent integration → Import demo. Left: JSON/MD textarea pre-filled with a sample quiz import payload. Right: response card showing the parsed result (quizId, questionCount, warnings) after clicking "Send request". |
| 13 | `13-share-modal-link.png` | Share modal — Link tab. Copy-URL input, Quick actions (Copy as Markdown / Download PDF / Print), QR for the room. Right rail: "What travels with it" toggles, Privacy segmented control, Agent-equivalent `POST /v1/shares` snippet. |
| 14 | `14-share-modal-card.png` | Share modal — Card tab. 1200×630 social card preview rendered live on the page (logo, kicker, serif prompt, attribution, accent radial highlight). Below: Download PNG / Copy image / Edit style actions; OG hint. |
| 15 | `15-share-modal-socials.png` | Share modal — Socials tab. 3-column grid of platform buttons (X, LinkedIn, Reddit, Hacker News, Email, Mastodon) with colored badges. Below: live preview of the composed message + Copy composed message. |

## How to read these

- **Don't pixel-match decoration** — match the structure, hierarchy, and spacing. Token values are in `../tokens.md`.
- **Cropping note** — the captures are 914×540 (preview iframe size), which is narrower than the production target (≥1280px). At 1280px columns will be wider and rails won't wrap; the relative proportions are still accurate.
- **The "Share" and "Learning objectives" patterns** are reused across screens — see the `LearningObjectives` and `ShareModal` components in `../source/src/share.jsx`.

## Re-capturing

The HTML supports dev navigation via `window.__harus.go(routeId)` once mounted — useful if you want to re-capture different states. Valid routes: `library`, `exams`, `quiz`, `results`, `dashboard`, `author`, `agent`.
