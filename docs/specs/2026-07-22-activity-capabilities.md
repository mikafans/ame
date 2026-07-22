# Activity capability contract

M2 keeps activity metadata stable while making the learner payload extensible. The `payload.content` object is discriminated by its `type` field and is validated at the authoring boundary before approved content is published.

The first supported capabilities are:

- `explanation`: heading, body, and non-empty key points.
- `worked_example`: heading, prompt, ordered steps, and reflection.
- `rich_text`: heading and Markdown body.
- `diagram`: title, Mermaid source, and accessible alternative text.
- `code_example`: title, language, code, and explanation.
- `scenario`: context, prompt, and at least two labelled choices.

The activity kind constrains which capabilities are valid: explanations can carry explanatory text or diagrams, examples can carry worked examples or code, and practice/application activities can carry scenarios. This keeps the semantic learning contract separate from presentation technology.

The server rejects unknown or malformed capabilities. The browser also validates the payload before rendering and shows a safe unavailable-capability state for a future format. Unsupported formats remain inspectable through the activity metadata rather than silently disappearing.

Scenario choice state is intentionally local in M2. Persistent answers, feedback, grading, and executable labs belong to the assessment/task milestones; rendering a choice is not treated as evidence of mastery.
