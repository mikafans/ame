import { ExternalLink, Sparkles } from "lucide-react";
import { HighlightedCode } from "@/components/HighlightedCode";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";

const startExample = `POST /public/v1/onboarding/start
Content-Type: application/json

{
  "email": "learner@example.com",
  "displayName": "Learner",
  "prompt": "I'd like to learn a new subject",
  "idempotencyKey": "first-music-journey"
}`;

const loopExample = `GET /api/v1/learning/journeys
GET /api/v1/learning/journeys/{id}
POST /api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start
POST /api/v1/learning/sessions/{id}/finish`;

export default function AgentPage() {
  return (
    <PageShell
      kicker="One learning API · OpenAPI 3.1"
      title="Bring your agent to ame"
      subtitle="An agent can create a learner, shape the first journey, and keep it moving. People and agents use the same owner-scoped data."
      action={
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" asChild>
            <a href="/public/skill.json" target="_blank" rel="noreferrer">
              Skill manifest <ExternalLink />
            </a>
          </Button>
          <Button variant="outline" asChild>
            <a href="/public/openapi.yaml" target="_blank" rel="noreferrer">
              OpenAPI <ExternalLink />
            </a>
          </Button>
          <Button variant="outline" asChild>
            <a
              href="/public/learning-contract.json"
              target="_blank"
              rel="noreferrer"
            >
              Learning contract <ExternalLink />
            </a>
          </Button>
        </div>
      }
    >
      <div className="grid gap-5 lg:grid-cols-3">
        <Step
          number="01"
          title="Start with intent"
          body="Send an email identifier and a first prompt. ame returns a learner token and creates the first journey."
        />
        <Step
          number="02"
          title="Read the next move"
          body="The journey exposes its promise, objectives, activities, and the next useful activity."
        />
        <Step
          number="03"
          title="Record evidence"
          body="Start a session, let the learner work, and finish it with responses. The web app sees the same result."
        />
      </div>

      <section className="mt-6 grid gap-6 rounded-2xl border border-border bg-card p-6 lg:grid-cols-2">
        <div>
          <div className="mb-4 flex size-10 items-center justify-center rounded-xl bg-primary/15 text-primary">
            <Sparkles className="size-5" />
          </div>
          <h2 className="text-xl font-semibold">
            Nothing separate to configure
          </h2>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            The token identifies the learner, not a second product identity.
            Authentication, rate limiting, request IDs, and audit records apply
            equally to browser and agent requests.
          </p>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            In self-host mode, email is an account identifier. ame does not send
            mail and does not require SMTP for this flow.
          </p>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            Before changing learner state, read the learning contract: an agent
            must not turn generated text, simulated answers, or pending review
            into a claim about what a learner knows.
          </p>
        </div>
        <div>
          <p className="mb-2 font-mono text-xs uppercase tracking-[0.2em] text-muted-foreground">
            First request
          </p>
          <HighlightedCode code={startExample} language="http" />
        </div>
      </section>

      <section className="mt-6 rounded-2xl border border-border bg-card p-6">
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-muted-foreground">
          The learning loop
        </p>
        <h2 className="mt-3 text-xl font-semibold">
          One contract, two experiences
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
          Agents can orchestrate learning without importing assessments or
          maintaining a parallel activity model. Learners can open the web app
          at any time and continue exactly where the agent left them.
        </p>
        <div className="mt-5">
          <HighlightedCode code={loopExample} language="http" />
        </div>
        <div className="mt-5 flex flex-wrap gap-3">
          <Button asChild>
            <a href="/public/llms.txt" target="_blank" rel="noreferrer">
              Read the agent guide <ExternalLink />
            </a>
          </Button>
          <Button variant="outline" asChild>
            <a href="/self-hosting">Self-hosting guide</a>
          </Button>
        </div>
      </section>
    </PageShell>
  );
}

function Step({
  number,
  title,
  body,
}: {
  number: string;
  title: string;
  body: string;
}) {
  return (
    <section className="rounded-2xl border border-border bg-card p-5">
      <p className="font-mono text-xs tracking-[0.2em] text-primary">
        {number}
      </p>
      <h2 className="mt-4 font-semibold">{title}</h2>
      <p className="mt-2 text-sm leading-6 text-muted-foreground">{body}</p>
    </section>
  );
}
