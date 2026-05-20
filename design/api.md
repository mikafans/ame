# Harus — API surface sketch (OpenAPI 3.1 outline)

This is a minimal, paste-able starting point. Flesh out schemas with your real Zod / Pydantic / Drizzle models. Auth, pagination envelopes, and error shapes are sketched but not exhaustive.

```yaml
openapi: 3.1.0
info:
  title: Harus Assessment API
  version: 0.1.0
  description: |
    Unified surface for learners, authors, and agents. Every screen the app renders
    is backed by an endpoint in this document.
servers:
  - url: https://api.harus.app/v1
security:
  - bearerAuth: []
tags:
  - { name: quiz,    description: Single-quiz CRUD and generation }
  - { name: exam,    description: Composed exams (bundled quizzes with weighted sections) }
  - { name: session, description: Ad-hoc practice sessions from the item bank }
  - { name: attempt, description: A learner's response to a quiz or exam }
  - { name: stats,   description: Aggregate analytics }
  - { name: agent,   description: Agent integration: keys, MCP manifest, webhooks }

components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: hk_<env>_<id>
  schemas:
    Question:
      type: object
      required: [id, type, prompt, points]
      properties:
        id:     { type: string }
        type:   { type: string, enum: [mc, tf, short, essay, code] }
        prompt: { type: string }
        points: { type: integer, minimum: 0 }
        options:
          type: array
          items:
            type: object
            properties:
              id:   { type: string }
              text: { type: string }
        correct:
          oneOf:
            - { type: string }
            - { type: boolean }
            - { type: array, items: { type: string } }
        starter:    { type: string, description: "Code questions only" }
        language:   { type: string, description: "Code questions only" }
        minWords:   { type: integer, description: "Essay questions only" }
        explanation:{ type: string }
        tags:       { type: array, items: { type: string } }
        difficulty: { type: string, enum: [intro, inter, adv] }
    Quiz:
      type: object
      required: [id, title, course, questions]
      properties:
        id:         { type: string }
        title:      { type: string }
        course:     { type: string }
        author:     { type: string }
        difficulty: { type: string }
        duration:   { type: integer, description: "Minutes" }
        objectives: { type: array, items: { type: string }, description: "3-4 outcome bullets — 'what you'll learn'" }
        questions:  { type: array, items: { $ref: '#/components/schemas/Question' } }
        tags:       { type: array, items: { type: string } }
        status:     { type: string, enum: [draft, active, archived] }
    ExamSection:
      type: object
      required: [quizId, weight]
      properties:
        id:      { type: string }
        title:   { type: string }
        quizId:  { type: string }
        weight:  { type: integer }
        items:   { type: integer }
        mix:     { type: string }
    Exam:
      type: object
      required: [id, title, course, sections]
      properties:
        id:          { type: string }
        title:       { type: string }
        course:      { type: string }
        composedBy:  { type: string }
        method:      { type: string, enum: [manual, agent] }
        status:      { type: string, enum: [draft, scheduled, active, archived] }
        open:        { type: string, format: date-time }
        close:       { type: string, format: date-time }
        duration:    { type: integer }
        totalPoints: { type: integer }
        passing:     { type: integer }
        objectives:  { type: array, items: { type: string }, description: "3-4 outcome bullets" }
        sections:    { type: array, items: { $ref: '#/components/schemas/ExamSection' } }
    Attempt:
      type: object
      properties:
        id:        { type: string }
        userId:    { type: string }
        quizId:    { type: string }
        examId:    { type: string }
        submitted: { type: string, format: date-time }
        duration:  { type: string }
        score:     { type: number }
        total:     { type: number }
        percent:   { type: number }
        answers:
          type: array
          items:
            type: object
            properties:
              qid:     { type: string }
              given:   {}
              correct: { type: boolean }
              points:  { type: number }
              max:     { type: number }
              note:    { type: string }

paths:
  /quizzes:
    get:
      tags: [quiz]
      summary: List quizzes
      parameters:
        - { in: query, name: course, schema: { type: string } }
        - { in: query, name: status, schema: { type: string } }
        - { in: query, name: tag,    schema: { type: string } }
      responses: { '200': { description: OK } }
    post:
      tags: [quiz]
      summary: Import a quiz (JSON or Markdown)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [source, format]
              properties:
                source:   { type: string, description: "Raw payload or external URL" }
                format:   { type: string, enum: [json, md] }
                courseId: { type: string }
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                type: object
                properties:
                  quizId:        { type: string }
                  questionCount: { type: integer }
                  warnings:      { type: array, items: { type: string } }

  /quizzes/{id}:
    get:
      tags: [quiz]
      parameters: [{ in: path, name: id, required: true, schema: { type: string } }]
      responses: { '200': { description: OK } }
    patch:
      tags: [quiz]
      parameters: [{ in: path, name: id, required: true, schema: { type: string } }]
      responses: { '200': { description: OK } }

  /quizzes/generate:
    post:
      tags: [quiz]
      summary: Generate a quiz from a source document
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [source, questionCount]
              properties:
                source:         { type: string }
                questionCount:  { type: integer, minimum: 1 }
                types:          { type: array, items: { type: string } }
                difficulty:     { type: string }
      responses: { '201': { description: Created } }

  /quizzes/{id}/stats:
    get:
      tags: [stats]
      parameters:
        - { in: path,  name: id,       required: true, schema: { type: string } }
        - { in: query, name: cohortId, schema: { type: string } }
        - { in: query, name: window,   schema: { type: string, enum: [last30d, all] } }
      responses: { '200': { description: OK } }

  /exams:
    get:
      tags: [exam]
      responses: { '200': { description: OK } }
    post:
      tags: [exam]
      summary: Compose an exam from existing quizzes
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [title, sections, duration]
              properties:
                title:    { type: string }
                duration: { type: integer }
                sections:
                  type: array
                  items:
                    type: object
                    required: [quizId, weight]
                    properties:
                      quizId: { type: string }
                      weight: { type: integer }
                      items:  { type: integer }
                window:
                  type: object
                  properties:
                    open:  { type: string, format: date-time }
                    close: { type: string, format: date-time }
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                type: object
                properties:
                  examId:      { type: string }
                  totalPoints: { type: integer }
                  sections:    { type: array, items: { $ref: '#/components/schemas/ExamSection' } }
                  warnings:    { type: array, items: { type: string } }

  /exams/{id}:
    get:
      tags: [exam]
      parameters: [{ in: path, name: id, required: true, schema: { type: string } }]
      responses: { '200': { description: OK } }

  /exams/{id}/stats:
    get:
      tags: [stats]
      parameters:
        - { in: path,  name: id,       required: true, schema: { type: string } }
        - { in: query, name: cohortId, schema: { type: string } }
      responses: { '200': { description: OK } }

  /sessions:
    post:
      tags: [session]
      summary: Start a session — quiz attempt, exam attempt, or ad-hoc practice
      description: |
        Three request body flavours, distinguished by which top-level keys are present:
        - **Quiz attempt**: `{ quizId }` — server hydrates the quiz's questions.
        - **Exam attempt**: `{ examId }` — server hydrates per-section composition.
        - **Practice**: `{ cats[], types[], count, ... }` — ad-hoc draw from the item bank.
      requestBody:
        content:
          application/json:
            schema:
              oneOf:
                - type: object
                  required: [quizId]
                  properties:
                    quizId: { type: string }
                - type: object
                  required: [examId]
                  properties:
                    examId: { type: string }
                - type: object
                  required: [cats, types, count]
                  properties:
                    cats:     { type: array, items: { type: string } }
                    tags:     { type: array, items: { type: string } }
                    types:    { type: array, items: { type: string, enum: [mc, tf, short, essay, code] } }
                    diff:     { type: string, enum: [intro, inter, adv, mixed] }
                    count:    { type: integer }
                    duration: { type: integer }
                    mode:     { type: string, enum: [practice, timed, adaptive] }
                    shuffle:  { type: boolean }
                    explain:  { type: boolean }
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                type: object
                properties:
                  sessionId: { type: string }
                  questions:
                    type: array
                    items: { $ref: '#/components/schemas/Question' }

  /attempts/{id}:
    get:
      tags: [attempt]
      parameters: [{ in: path, name: id, required: true, schema: { type: string } }]
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema: { $ref: '#/components/schemas/Attempt' }

  /messages:
    post:
      tags: [agent]
      summary: Send feedback or a reminder to a learner
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [userId, channel, body]
              properties:
                userId:      { type: string }
                channel:     { type: string, enum: [in_app, email] }
                body:        { type: string }
                linkQuizId:  { type: string }
      responses: { '201': { description: Queued } }

  /plans:
    post:
      tags: [agent]
      summary: Generate a study plan from a learner's attempt history
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [userId, goal]
              properties:
                userId:        { type: string }
                goal:          { type: string }
                lookbackDays:  { type: integer, default: 30 }
      responses: { '201': { description: Created } }

  /agents/register:
    post:
      tags: [agent]
      summary: One-shot bootstrap — mint an agent user + API key
      security: []  # unauthenticated; rate-limited per IP
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                label:  { type: string }
                scopes:
                  type: array
                  items: { type: string, enum: [quiz.read, quiz.write, attempt.read, attempt.write, stats.read, feedback.write, plan.read, plan.write] }
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                type: object
                properties:
                  apiKey:         { type: string, description: "hk_<env>_<full> — shown once" }
                  userId:         { type: string }
                  openapiUrl:     { type: string, format: uri }
                  mcpManifestUrl: { type: string, format: uri }

  /agents/activity:
    get:
      tags: [agent]
      summary: Paginated ActivityLog for the current agent
      parameters:
        - { in: query, name: cursor, schema: { type: string } }
        - { in: query, name: limit,  schema: { type: integer, minimum: 1, maximum: 200 } }
      responses: { '200': { description: OK } }

  /shares:
    post:
      tags: [agent]
      summary: Create a shareable link for a quiz, exam, or single question
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [kind, id]
              properties:
                kind:       { type: string, enum: [quiz, exam, item] }
                id:         { type: string }
                visibility: { type: string, enum: [public, cohort], default: public }
                includeExplanation: { type: boolean, default: false }
                includeScore:       { type: boolean, default: false }
                includeAttribution: { type: boolean, default: true }
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                type: object
                properties:
                  shareId:  { type: string }
                  url:      { type: string, format: uri }
                  embedUrl: { type: string, format: uri }
                  og:
                    type: object
                    properties:
                      title:       { type: string }
                      description: { type: string }
                      image:       { type: string, format: uri, description: "1200x630 generated PNG" }

  /shares/{id}:
    get:
      tags: [agent]
      summary: Resolve a share — public, read-only
      security: []  # no auth required for public shares
      parameters: [{ in: path, name: id, required: true, schema: { type: string } }]
      responses: { '200': { description: OK } }
```

## Webhooks (recommended)

| Event               | Payload (sketch)                                      |
| ------------------- | ----------------------------------------------------- |
| `attempt.submitted` | `{ attemptId, userId, quizId, examId?, submittedAt }` |
| `attempt.graded`    | `{ attemptId, score, total, percent, gradedAt }`      |
| `quiz.published`    | `{ quizId, publishedBy, publishedAt }`                |
| `exam.opened`       | `{ examId, openedAt }`                                |
| `exam.closed`       | `{ examId, closedAt, attemptsCount }`                 |
| `plan.created`      | `{ planId, userId, generatedAt }`                     |

## MCP manifest (export shape)

```json
{
  "schema_version": "v1",
  "name": "harus",
  "description": "Read and write quizzes, exams, and attempts on Harus.",
  "auth": { "type": "bearer" },
  "tools": [
    {
      "name": "quiz.import",
      "description": "Create a new quiz from JSON or Markdown source.",
      "input_schema": {
        "type": "object",
        "required": ["source", "format"],
        "properties": {
          "source": { "type": "string" },
          "format": { "type": "string", "enum": ["json", "md"] },
          "courseId": { "type": "string" }
        }
      }
    }
  ]
}
```

Generate one tool entry per row of the table in `README.md` § API surface. The descriptor preview UI in `src/screen-agent.jsx` (`mcpDescriptor`) shows the intended on-screen format.
