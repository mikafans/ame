import Link from "next/link";
import { ArrowRight, ExternalLink } from "lucide-react";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";

const contentExample = `{
  "content": {
    "type": "explanation",
    "heading": "Event time",
    "body": "Use the timestamp carried by the event.",
    "key_points": ["Event time is domain time."]
  }
}`;

export default function AgentGuidePage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border">
        <div className="mx-auto flex min-h-16 max-w-6xl items-center justify-between gap-4 px-4 sm:px-6">
          <Logo size={24} />
          <div className="flex items-center gap-4 text-sm">
            <Link
              href="/agent"
              className="font-medium text-muted-foreground hover:text-foreground"
            >
              Agent setup
            </Link>
            <Link
              href="/"
              className="font-medium text-muted-foreground hover:text-foreground"
            >
              Home
            </Link>
          </div>
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-4 py-12 sm:px-6 sm:py-16">
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          AME / course authoring guide
        </p>
        <h1 className="mt-4 max-w-3xl text-4xl font-bold tracking-[-0.045em] sm:text-5xl">
          Create a real course, then hand the learner a useful first step.
        </h1>
        <p className="mt-5 max-w-3xl text-base leading-7 text-muted-foreground sm:text-lg">
          This is the human-readable route through AME&apos;s public agent
          contract. An agent owns the setup work; the learner receives reviewed,
          source-grounded activities and honest feedback.
        </p>
        <div className="mt-8 flex flex-wrap gap-3">
          <Button asChild className="rounded-full">
            <a href="/public/openapi.yaml" target="_blank" rel="noreferrer">
              OpenAPI <ExternalLink className="size-4" />
            </a>
          </Button>
          <Button asChild variant="outline" className="rounded-full">
            <a href="/public/llms.txt" target="_blank" rel="noreferrer">
              Agent contract <ExternalLink className="size-4" />
            </a>
          </Button>
        </div>

        <section className="mt-14 grid gap-4 md:grid-cols-4">
          <Step
            number="01"
            title="Ground it"
            body="Import bounded source text and certify an exact citation before generating instruction."
          />
          <Step
            number="02"
            title="Shape it"
            body="Create a private course, objectives, chapters, explanations, examples, checks, and application work."
          />
          <Step
            number="03"
            title="Review it"
            body="Run generation through review, validate the complete course, record an approved review, then publish."
          />
          <Step
            number="04"
            title="Adapt it"
            body="Read learner evidence and assessment results to select the next durable activity."
          />
        </section>

        <section className="mt-14 grid gap-8 border-t border-border pt-10 lg:grid-cols-[1fr_0.9fr]">
          <div>
            <h2 className="text-2xl font-semibold tracking-[-0.03em]">
              What makes a course publishable
            </h2>
            <ul className="mt-5 space-y-3 text-sm leading-6 text-muted-foreground">
              <li>
                Every outcome has instruction, a worked example, a formative
                check with rationale and learner feedback, and a mastery or
                application path.
              </li>
              <li>
                Activities stay private until their content, sources, and
                reviews are ready; publishing is course-level, not a per-row
                shortcut.
              </li>
              <li>
                Formative checks must explain the result. Scenarios must either
                declare a correct answer and feedback or be honestly framed as
                ungraded reflection.
              </li>
            </ul>
          </div>
          <div className="border border-border bg-card p-5">
            <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
              Attach approved content
            </p>
            <pre className="mt-4 overflow-x-auto bg-muted p-4 text-xs leading-6">
              <code>{contentExample}</code>
            </pre>
            <p className="mt-4 text-sm leading-6 text-muted-foreground">
              Use this payload after the matching content-generation run reaches{" "}
              <code>published</code>. The complete capability shapes remain in
              the machine-readable contract.
            </p>
          </div>
        </section>
        <section className="mt-12 border-t border-border pt-10">
          <h2 className="text-2xl font-semibold tracking-[-0.03em]">
            Start with the contract, not assumptions.
          </h2>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
            The public manifest, OpenAPI, and Markdown are the authoritative API
            surfaces. Read them before mutation; retain the course revision IDs
            returned by every authoring response.
          </p>
          <Button asChild variant="outline" className="mt-5 rounded-full">
            <Link href="/agent">
              Back to agent setup <ArrowRight className="size-4" />
            </Link>
          </Button>
        </section>
      </main>
    </div>
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
    <article className="border border-border bg-card p-5">
      <p className="font-mono text-xs tracking-[0.14em] text-primary">
        {number}
      </p>
      <h2 className="mt-5 text-lg font-semibold">{title}</h2>
      <p className="mt-2 text-sm leading-6 text-muted-foreground">{body}</p>
    </article>
  );
}
