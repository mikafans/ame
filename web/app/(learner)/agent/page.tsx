import Link from "next/link";
import { ArrowRight, ExternalLink } from "lucide-react";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";

const onboardingExample = `POST /public/v1/onboarding/start
{
  "email": "learner@example.com",
  "displayName": "Learner",
  "prompt": "Help me learn distributed systems",
  "idempotencyKey": "first-journey"
}`;

const publicResources = [
  ["Agent guide", "/public/llms.txt"],
  ["Skill manifest", "/public/skill.json"],
  ["OpenAPI", "/public/openapi.yaml"],
  ["Learning contract", "/public/learning-contract.json"],
] as const;

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
              Public agent setup · OpenAPI 3.1
            </p>
            <h1 className="mt-4 max-w-3xl text-4xl font-bold tracking-[-0.045em] sm:text-5xl">
              Give your agent AME. Keep the learner out of setup.
            </h1>
            <p className="mt-5 max-w-2xl text-base leading-7 text-muted-foreground sm:text-lg">
              Your agent discovers the public contract, creates the durable
              journey, and shapes the course. The learner opens AME when there
              is something useful to learn.
            </p>
            <div className="mt-7 flex flex-wrap gap-3">
              {publicResources.map(([label, href]) => (
                <Button key={href} asChild variant="outline" size="sm">
                  <a href={href} target="_blank" rel="noreferrer">
                    {label} <ExternalLink className="size-3.5" />
                  </a>
                </Button>
              ))}
            </div>
          </div>

          <section className="min-w-0 border border-border bg-card p-5 sm:p-6">
            <p className="font-mono text-xs uppercase tracking-[0.14em] text-muted-foreground">
              The first durable action
            </p>
            <pre className="mt-4 overflow-x-auto border border-border bg-muted p-4 text-xs leading-6 text-foreground sm:text-sm">
              <code>{onboardingExample}</code>
            </pre>
            <p className="mt-4 text-sm leading-6 text-muted-foreground">
              An email identifier establishes the learner-owned journey. It is
              not a separate agent identity or another browser signup step.
            </p>
          </section>
        </section>

        <section className="py-12">
          <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
            Three moves
          </p>
          <div className="mt-5 grid gap-4 md:grid-cols-3">
            <GuideStep
              number="01"
              title="Read the boundary"
              body="Read the public learning contract before changing state. It keeps generated material separate from learner evidence."
            />
            <GuideStep
              number="02"
              title="Shape the beginning"
              body="Preview a goal or select a reviewed native journey. The agent can make the first lesson useful before the learner arrives."
            />
            <GuideStep
              number="03"
              title="Hand off a course"
              body="Use the same owner-scoped API to continue, assess, and recommend. The browser sees the exact durable state the agent created."
            />
          </div>
        </section>

        <section className="grid gap-6 border-t border-border pt-10 md:grid-cols-[minmax(0,1fr)_auto] md:items-center">
          <div>
            <h2 className="text-2xl font-semibold tracking-[-0.03em]">
              The browser is the course workspace.
            </h2>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
              AME does not make the learner repeat the agent&apos;s setup. Once
              a journey exists, the learner can inspect, practise, and continue
              it in the learning desk.
            </p>
          </div>
          <Button asChild className="w-fit rounded-full">
            <a href="/public/llms.txt" target="_blank" rel="noreferrer">
              Read the agent guide <ArrowRight className="size-4" />
            </a>
          </Button>
        </section>
      </main>
    </div>
  );
}

function GuideStep({
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
