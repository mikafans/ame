"use client";

import Link from "next/link";
import { ArrowRight, Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Logo } from "@/components/Logo";
import { useColorMode } from "@/components/ThemeRegistry";
import { useAuth } from "@/hooks/useAuth";

const benefits = [
  {
    number: "01",
    title: "Name the domain and goal",
    body: "Tell your agent what you want to understand or make. There is no separate AME form to complete first.",
  },
  {
    number: "02",
    title: "Let the agent build the beginning",
    body: "Your agent discovers AME, creates the durable journey, and turns the goal into a course-shaped first session.",
  },
  {
    number: "03",
    title: "Open the course when it is useful",
    body: "The learning desk is where you read, practise, and continue the course your agent has prepared—not another setup flow.",
  },
];

const faqs = [
  {
    question: "Do I need an AME account to explore the agent setup?",
    answer:
      "No. The agent guide, machine-readable contract, catalog, and preview endpoints are public. A durable learner identity is created only when an agent starts a journey.",
  },
  {
    question: "Can I use AME without an AI assistant?",
    answer:
      "The learning desk remains a normal browser workspace. AME is designed so an agent can do the setup work first, then hand you a coherent course to use.",
  },
  {
    question: "Can I self-host AME?",
    answer:
      "Yes. The local stack is Postgres, Valkey, API, web, and Caddy. The self-hosting guide is available from the footer.",
  },
];

const agentResources = [
  { href: "/agent", label: "Agent guide" },
  { href: "/public/llms.txt", label: "llms.txt" },
  { href: "/public/skill.json", label: "Skill manifest" },
  { href: "/public/openapi.yaml", label: "OpenAPI" },
];

export default function LandingPage() {
  const { user } = useAuth();
  const { mode, toggle } = useColorMode();

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="mx-auto max-w-7xl px-4 sm:px-8">
        <header className="flex min-h-20 items-center justify-between gap-4 border-b border-border">
          <Logo size={32} />
          <div className="flex items-center gap-2 sm:gap-6">
            <nav className="hidden items-center gap-6 text-sm text-muted-foreground sm:flex">
              <a className="transition hover:text-primary" href="#how-it-works">
                How it works
              </a>
              <a className="transition hover:text-primary" href="#agents">
                For agents
              </a>
              <a className="transition hover:text-primary" href="#faq">
                FAQ
              </a>
            </nav>
            <button
              type="button"
              aria-label={mode === "dark" ? "Use light mode" : "Use dark mode"}
              onClick={toggle}
              className="inline-flex size-8 items-center justify-center rounded-lg text-muted-foreground outline-none transition hover:bg-accent hover:text-accent-foreground focus-visible:ring-2 focus-visible:ring-ring"
            >
              {mode === "dark" ? (
                <Sun className="size-4" />
              ) : (
                <Moon className="size-4" />
              )}
            </button>
            {user ? (
              <Button asChild className="rounded-full px-5">
                <Link href="/learning">Open learning desk</Link>
              </Button>
            ) : (
              <Link
                href="/login"
                className="text-sm font-medium text-muted-foreground transition hover:text-foreground"
              >
                Sign in
              </Link>
            )}
          </div>
        </header>

        <main>
          <section className="relative overflow-hidden border border-primary/30 bg-[var(--ame-ink)] px-6 py-14 text-[var(--ame-sugar)] shadow-[0_24px_70px_rgba(90,45,100,0.22)] sm:px-10 md:px-14 md:py-20">
            <div className="absolute inset-y-0 right-0 hidden w-1/3 border-l border-primary/30 md:block" />
            <div className="relative grid items-center gap-12 md:grid-cols-[0.85fr_1.15fr] md:gap-20">
              <div className="space-y-8">
                <span className="inline-flex border-l-2 border-primary pl-3 font-mono text-[11px] tracking-[0.14em] text-primary">
                  AME / AGENT-NATIVE LEARNING
                </span>
                <h1 className="max-w-xl text-[42px] font-extrabold leading-[0.96] tracking-[-0.06em] sm:text-6xl md:text-[72px]">
                  Give your agent a goal. Meet it in a course.
                </h1>
                <p className="max-w-xl text-[17px] leading-7 text-[var(--ame-sugar)]/75 sm:text-[19px]">
                  AME is the durable learning workspace behind your agent. Say
                  what you want to learn; the agent can set up the journey,
                  shape the course, and leave you a clear next move.
                </p>
                <div className="flex flex-wrap items-center gap-4">
                  <Button
                    asChild
                    className="rounded-full bg-primary px-6 py-3 font-bold text-primary-foreground hover:bg-primary/90"
                  >
                    <Link href="/agent">
                      Set up your agent
                      <ArrowRight className="size-4" />
                    </Link>
                  </Button>
                  <a
                    className="text-sm font-medium text-[var(--ame-sugar)]/75 underline-offset-4 transition hover:text-[var(--ame-sugar)] hover:underline"
                    href="/public/llms.txt"
                  >
                    Read llms.txt
                  </a>
                </div>
              </div>

              <div className="border border-primary/30 bg-black/20 p-5 shadow-[16px_16px_0_rgba(0,0,0,0.18)] sm:p-7">
                <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-[var(--ame-sugar)]/60">
                  In your agent
                </p>
                <div className="mt-5 border border-primary/30 bg-black/20 p-4 text-sm leading-6 text-[var(--ame-sugar)] sm:p-5">
                  Help me learn distributed systems well enough to design a
                  small event-processing service.
                </div>
                <div className="mt-3 border-l-2 border-primary bg-[var(--ame-sugar)] px-4 py-4 text-sm leading-6 text-[var(--ame-ink)] sm:px-5">
                  I&apos;ll use AME to create your learning journey, prepare the
                  first lesson, and keep the next useful activity visible.
                </div>
                <div className="mt-5 flex items-center justify-between gap-3 border-t border-primary/30 pt-4 text-xs text-[var(--ame-sugar)]/65">
                  <span>One learner model · one durable journey</span>
                  <span className="font-mono">01 / DISCOVER</span>
                </div>
              </div>
            </div>
          </section>

          <section
            id="how-it-works"
            className="grid gap-10 border-b border-border px-2 py-16 sm:px-4 md:grid-cols-[0.7fr_1.3fr] md:py-24"
          >
            <div>
              <p className="mb-2 font-mono text-xs tracking-[0.14em] text-primary">
                02 / THE HANDOFF
              </p>
              <h2 className="mb-8 max-w-3xl text-3xl font-extrabold leading-tight tracking-[-0.04em] sm:text-4xl">
                The agent does the setup. You do the learning.
              </h2>
            </div>
            <div className="grid gap-0 border-t border-border">
              {benefits.map((benefit) => (
                <article
                  key={benefit.number}
                  className="grid gap-4 border-b border-border py-5 sm:grid-cols-[60px_0.8fr_1.2fr] sm:items-start"
                >
                  <span className="font-mono text-sm text-primary">
                    {benefit.number}
                  </span>
                  <h3 className="text-lg font-bold">{benefit.title}</h3>
                  <p className="leading-6 text-muted-foreground">
                    {benefit.body}
                  </p>
                </article>
              ))}
            </div>
          </section>

          <section
            id="agents"
            className="flex flex-col gap-5 border-b border-border px-2 py-8 sm:px-4 md:flex-row md:items-center md:justify-between"
          >
            <div className="flex gap-4">
              <span className="font-mono text-xs text-primary">03</span>
              <div>
                <h2 className="font-bold">A public start for every agent</h2>
                <p className="text-sm text-muted-foreground">
                  Discovery and setup stay outside the learner&apos;s browser
                  session. Use the guide or the machine-readable contract.
                </p>
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              {agentResources.map(({ href, label }) => (
                <Button
                  key={href}
                  asChild
                  variant="outline"
                  size="sm"
                  className="rounded-full border-border font-mono text-xs font-normal"
                >
                  <Link href={href}>{label}</Link>
                </Button>
              ))}
            </div>
          </section>

          <section
            id="faq"
            className="grid gap-12 px-2 py-16 sm:px-4 md:grid-cols-[1.4fr_1fr] md:py-24"
          >
            <div>
              <h2 className="mb-3 text-3xl font-extrabold tracking-[-0.04em]">
                Questions?
              </h2>
              <div className="divide-y divide-border">
                {faqs.map((faq) => (
                  <div key={faq.question} className="py-5">
                    <h3 className="mb-2 font-bold">{faq.question}</h3>
                    <p className="leading-6 text-muted-foreground">
                      {faq.answer}
                    </p>
                  </div>
                ))}
              </div>
            </div>
            <div className="self-start border border-border bg-card p-7">
              <p className="font-mono text-xs tracking-[0.14em] text-primary">
                READY WHEN YOU ARE
              </p>
              <h2 className="mb-3 mt-3 text-3xl font-extrabold leading-tight tracking-[-0.04em]">
                Give your agent the AME guide.
              </h2>
              <p className="mb-6 leading-6 text-muted-foreground">
                Your agent can start with the public contract, then hand you a
                course worth opening.
              </p>
              <Button asChild className="rounded-full font-bold">
                <Link href="/agent">
                  Open agent guide
                  <ArrowRight className="size-4" />
                </Link>
              </Button>
            </div>
          </section>
        </main>
      </div>

      <footer className="border-t border-border px-4 py-6 text-sm text-muted-foreground">
        <div className="mx-auto flex max-w-6xl flex-wrap justify-between gap-4">
          <span>© 2026 AME</span>
          <div className="flex gap-5">
            <Link href="/agent" className="transition hover:text-primary">
              Agent API
            </Link>
            <a
              href="https://github.com/mikafans/ame"
              className="transition hover:text-primary"
            >
              GitHub
            </a>
            <Link
              href="/self-hosting"
              className="transition hover:text-primary"
            >
              Self-hosting
            </Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
