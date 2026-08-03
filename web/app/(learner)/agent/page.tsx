import Link from "next/link";
import { ExternalLink } from "lucide-react";
import { Logo } from "@/components/Logo";
import { AgentHandoff } from "@/components/agent/AgentHandoff";
import { Button } from "@/components/ui/button";

const setupBrief = `Use AME to build my course.
Goal: [what I want to be able to do]
Current level: [what I already know]
Time: [time available each week]
Constraints or sources: [optional]

Read /public/skill.json first. Ground the course in sources, validate it,
review it, publish it, then give me the first activity in AME.`;

export default function AgentPage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border">
        <div className="mx-auto flex min-h-16 max-w-6xl items-center justify-between gap-4 px-4 sm:px-6">
          <Logo size={24} />
          <div className="flex items-center gap-4 text-sm">
            <Link
              href="/"
              className="font-medium text-muted-foreground transition hover:text-foreground"
            >
              Home
            </Link>
            <Link
              href="/login"
              className="font-medium text-muted-foreground transition hover:text-foreground"
            >
              Sign in
            </Link>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-4 py-12 sm:px-6 sm:py-16">
        <section className="grid gap-10 border-b border-border pb-12 lg:grid-cols-[minmax(0,1fr)_minmax(0,0.9fr)] lg:items-end">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
              AME / agent handoff
            </p>
            <h1 className="mt-4 max-w-3xl text-4xl font-bold tracking-[-0.045em] sm:text-5xl">
              Tell your agent what you want to learn. It sets up the course.
            </h1>
            <p className="mt-5 max-w-2xl text-base leading-7 text-muted-foreground sm:text-lg">
              You do not configure AME or assemble lessons. Give an authorized
              agent your learning goal; it creates a reviewed, source-grounded
              course and hands you the first useful activity.
            </p>
            <div className="mt-7 flex flex-wrap gap-3">
              <Button asChild className="rounded-full">
                <a href="#agent-handoff">Create agent handoff</a>
              </Button>
              <Button asChild variant="outline" className="rounded-full">
                <a href="/public/skill.json" target="_blank" rel="noreferrer">
                  Agent manifest <ExternalLink className="size-4" />
                </a>
              </Button>
            </div>
          </div>
          <section className="min-w-0 border border-border bg-card p-5 sm:p-6">
            <p className="font-mono text-xs uppercase tracking-[0.14em] text-muted-foreground">
              Give this to your agent
            </p>
            <pre className="mt-4 overflow-x-auto border border-border bg-muted p-4 text-xs leading-6 text-foreground sm:text-sm">
              <code>{setupBrief}</code>
            </pre>
            <p className="mt-4 text-sm leading-6 text-muted-foreground">
              The agent reads the machine contract. You only need to state the
              outcome you want.
            </p>
          </section>
        </section>

        <section className="py-12">
          <div id="agent-handoff">
            <AgentHandoff />
          </div>
        </section>

        <section className="border-t border-border py-12">
          <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
            The correct handoff
          </p>
          <div className="mt-5 grid gap-4 md:grid-cols-4">
            <Step
              number="01"
              owner="You"
              title="State the outcome"
              body="Give your agent a domain, goal, current level, time budget, and any useful sources or constraints."
            />
            <Step
              number="02"
              owner="Your agent"
              title="Create and check"
              body="It uses the temporary course-authoring handoff you created, then grounds the course in sources, creates lessons and checks, and validates the full course."
            />
            <Step
              number="03"
              owner="AME"
              title="Publish durable work"
              body="Only the reviewed course becomes visible. AME preserves the course, evidence, assessment feedback, and next action."
            />
            <Step
              number="04"
              owner="You"
              title="Learn the first step"
              body="Open the learning desk when the agent hands off a course. Answer checks honestly; AME and your agent adapt the next step."
            />
          </div>
        </section>

        <section className="grid gap-6 border-t border-border pt-10 md:grid-cols-[minmax(0,1fr)_auto] md:items-center">
          <div>
            <h2 className="text-2xl font-semibold tracking-[-0.03em]">
              For an agent: one source of truth.
            </h2>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
              <code>/public/skill.json</code> points to the compact{" "}
              <code>/public/llms.txt</code> entry contract and typed{" "}
              <code>/public/openapi.yaml</code>. Read those before mutation; do
              not infer undocumented state.
            </p>
          </div>
          <Button asChild variant="outline" className="w-fit rounded-full">
            <a href="/public/llms.txt" target="_blank" rel="noreferrer">
              Read agent contract <ExternalLink className="size-4" />
            </a>
          </Button>
        </section>
        <p className="mt-8 text-sm leading-6 text-muted-foreground">
          No agent available?{" "}
          <Link
            href="/start"
            className="font-medium text-primary underline-offset-4 hover:underline"
          >
            Set up AME yourself
          </Link>
          . This fallback asks you for the details an agent would otherwise
          provide.
        </p>
      </main>
    </div>
  );
}

function Step({
  number,
  owner,
  title,
  body,
}: {
  number: string;
  owner: string;
  title: string;
  body: string;
}) {
  return (
    <article className="border border-border bg-card p-5">
      <p className="font-mono text-xs tracking-[0.14em] text-primary">
        {number} / {owner}
      </p>
      <h2 className="mt-5 text-lg font-semibold">{title}</h2>
      <p className="mt-2 text-sm leading-6 text-muted-foreground">{body}</p>
    </article>
  );
}
