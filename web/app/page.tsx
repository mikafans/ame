"use client";

import { useState } from "react";
import Link from "next/link";
import { ArrowRight, Check, Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Logo } from "@/components/Logo";
import { useColorMode } from "@/components/ThemeRegistry";
import { useAuth } from "@/hooks/useAuth";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";

type LearningPreview = components["schemas"]["PreviewLearningResponse"];

const benefits = [
  {
    number: "01",
    title: "Start with a real intention",
    body: "Name the thing you want to learn in plain language. ame turns that first signal into a promise, objectives, and a useful beginning.",
  },
  {
    number: "02",
    title: "Learn in small moves",
    body: "Each journey gives you one next activity with a reason. Explanations, practice, and reflection can grow as your understanding does.",
  },
  {
    number: "03",
    title: "Keep the whole story",
    body: "Your intent, work, evidence, and next step stay together. An agent and the web app can continue the same journey without a parallel model.",
  },
];

const faqs = [
  {
    question: "Can I use ame without an AI assistant?",
    answer:
      "Yes. The web app and agents use the same learning journey API. Start in either place and continue in the other.",
  },
  {
    question: "Can I self-host AME?",
    answer:
      "Yes. The recommended local stack is Postgres, Valkey, API, web, and Caddy. The self-hosting guide is available from the footer.",
  },
  {
    question: "What should I do first?",
    answer:
      "Write one prompt about what you want to learn. Preview the first journey, add an email identifier, and begin.",
  },
];

function QuizPreview() {
  return (
    <div
      role="img"
      aria-label="Preview of a first learning activity"
      className="rounded-xl bg-card p-4 text-card-foreground shadow-[0_24px_60px_oklch(0.1_0.06_300_/_0.28)] sm:p-6"
    >
      <div className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <span className="font-mono text-xs text-muted-foreground">
            STEP 1 / START
          </span>
          <span className="rounded-full bg-primary/10 px-3 py-1 text-[11px] text-primary">
            First signal · 5 min
          </span>
        </div>
        <div className="h-1 overflow-hidden rounded-full bg-muted">
          <div className="h-full w-[35%] bg-primary" />
        </div>
        <p className="pt-1 text-base font-bold sm:text-[17px]">
          What would make this topic useful to you?
        </p>
        <div className="space-y-2">
          <div className="rounded-lg border border-border px-3 py-2.5">
            Understand the core ideas
          </div>
          <div className="flex items-center justify-between rounded-lg border-2 border-primary bg-primary/10 px-3 py-2 text-card-foreground">
            <span>Build something small</span>
            <Check className="size-5 text-primary" />
          </div>
          <div className="rounded-lg border border-border px-3 py-2.5">
            <span>Explain it clearly</span>
          </div>
        </div>
        <div className="flex items-center justify-between gap-3 pt-1">
          <span className="text-xs text-muted-foreground">
            Your next move · made visible
          </span>
          <span className="rounded-full bg-foreground px-4 py-2 text-center text-sm text-background">
            Next
          </span>
        </div>
      </div>
    </div>
  );
}

function LearningPreviewCard({
  preview,
  prompt,
}: {
  preview: LearningPreview;
  prompt: string;
}) {
  return (
    <div className="border border-border bg-card p-5 text-card-foreground shadow-[10px_10px_0_oklch(0.64_0.2_25_/_0.24)] sm:p-7">
      <div className="space-y-5">
        <div>
          <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-primary">
            {preview.templateId} · v{preview.templateVersion}
          </p>
          <h2 className="mt-2 text-xl font-bold tracking-tight">
            {preview.promise}
          </h2>
        </div>
        <div>
          <p className="mb-2 text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
            Your first outcomes
          </p>
          <ul className="space-y-2 text-sm leading-5">
            {preview.objectives.map((objective) => (
              <li key={objective.statement} className="flex gap-2">
                <Check className="mt-0.5 size-4 shrink-0 text-primary" />
                <span>{objective.statement}</span>
              </li>
            ))}
          </ul>
        </div>
        <div className="rounded-lg bg-muted p-3 text-sm">
          <p className="font-semibold">
            First step: {preview.firstActivity.title}
          </p>
          <p className="mt-1 text-muted-foreground">
            {preview.firstActivity.purpose} · about{" "}
            {preview.firstActivity.estimatedMinutes} minutes
          </p>
        </div>
        <Button
          asChild
          className="w-full rounded-full bg-primary font-bold text-primary-foreground hover:bg-primary/90"
        >
          <Link href={`/start?prompt=${encodeURIComponent(prompt)}`}>
            Start this journey
            <ArrowRight className="size-4" />
          </Link>
        </Button>
      </div>
    </div>
  );
}

export default function LandingPage() {
  const { user } = useAuth();
  const { mode, toggle } = useColorMode();
  const [prompt, setPrompt] = useState("");
  const [preview, setPreview] = useState<LearningPreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const signedIn = !!user;
  const entryHref = signedIn ? "/learning" : "/login?tab=signup";
  const entryLabel = signedIn ? "Open learning desk" : "Sign up free";
  const sampleHref = signedIn ? "/learning" : "#benefits";
  const sampleLabel = signedIn ? "Open learning desk" : "See how it works";

  async function handlePreview(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPreviewError(null);
    setPreviewLoading(true);
    try {
      const { data, error, response } = await api.POST(
        "/v1/onboarding/preview",
        { body: { prompt } },
      );
      if (!response.ok || !data) {
        void error;
        throw new Error("Could not prepare a learning preview");
      }
      setPreview(data);
    } catch (error) {
      setPreviewError(
        error instanceof Error ? error.message : "Could not reach AME",
      );
    } finally {
      setPreviewLoading(false);
    }
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="mx-auto max-w-7xl px-4 sm:px-8">
        <header className="flex min-h-20 items-center justify-between gap-4 border-b border-border">
          <Logo size={32} />
          <div className="flex items-center gap-2 sm:gap-6">
            <nav className="hidden items-center gap-6 text-sm text-muted-foreground sm:flex">
              <a className="transition hover:text-primary" href="#benefits">
                Features
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
            <Button
              asChild
              className="rounded-full bg-primary px-5 text-primary-foreground hover:bg-primary/90"
            >
              <Link href={entryHref}>{entryLabel}</Link>
            </Button>
          </div>
        </header>

        <main>
          <section className="relative overflow-hidden border border-primary/30 bg-[var(--ame-ink)] px-6 py-14 text-[var(--ame-sugar)] shadow-[0_24px_70px_rgba(90,45,100,0.22)] sm:px-10 md:px-14 md:py-20">
            <div className="absolute inset-y-0 right-0 hidden w-1/3 border-l border-primary/30 bg-[var(--ame-ink)] md:block" />
            <div className="relative grid items-start gap-12 md:grid-cols-[0.85fr_1.15fr] md:gap-20">
              <div className="space-y-8">
                <span className="inline-flex border-l-2 border-primary pl-3 font-mono text-[11px] tracking-[0.14em] text-primary">
                  AME / LEARNING DESK
                </span>
                <h1 className="max-w-xl text-[42px] font-extrabold leading-[0.96] tracking-[-0.06em] sm:text-6xl md:text-[72px]">
                  Study what you don&apos;t know yet.
                </h1>
                <p className="max-w-xl text-[17px] leading-7 text-[var(--ame-sugar)]/75 sm:text-[19px]">
                  Tell ame what you want to learn. It turns your intent into a
                  focused first journey, then gives you one clear next step.
                </p>
                <form onSubmit={handlePreview} className="max-w-xl space-y-2">
                  <label htmlFor="learning-intent" className="sr-only">
                    What would you like to learn?
                  </label>
                  <div className="flex flex-col gap-2 sm:flex-row">
                    <input
                      id="learning-intent"
                      value={prompt}
                      onChange={(event) => setPrompt(event.target.value)}
                      placeholder="I'd like to learn a new subject"
                      className="min-h-11 flex-1 rounded-full border border-primary/40 bg-black/20 px-5 text-sm text-[var(--ame-sugar)] outline-none placeholder:text-[var(--ame-sugar)]/60 focus:border-primary focus:ring-2 focus:ring-primary/30"
                    />
                    <Button
                      type="submit"
                      disabled={previewLoading || !prompt.trim()}
                      className="min-h-11 rounded-full bg-primary px-5 font-bold text-primary-foreground hover:bg-primary/90"
                    >
                      {previewLoading ? "Preparing…" : "See my plan"}
                      <ArrowRight className="size-4" />
                    </Button>
                  </div>
                  {previewError && (
                    <p role="alert" className="text-sm text-red-300">
                      {previewError}
                    </p>
                  )}
                </form>
                <div className="flex flex-col gap-3 border-t border-primary/30 pt-5 sm:flex-row">
                  <Button
                    asChild
                    className="rounded-full bg-primary px-6 py-3 font-bold text-primary-foreground hover:bg-primary/90"
                  >
                    <Link href={entryHref}>
                      {entryLabel}
                      <ArrowRight className="size-4" />
                    </Link>
                  </Button>
                  <Button
                    asChild
                    variant="outline"
                    className="rounded-full border-primary/50 bg-transparent px-6 py-3 text-[var(--ame-sugar)] hover:border-primary hover:bg-primary/10 hover:text-[var(--ame-sugar)]"
                  >
                    <Link href={sampleHref}>{sampleLabel}</Link>
                  </Button>
                </div>
              </div>
              <div className="space-y-4 md:pt-8">
                <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-[var(--ame-sugar)]/60">
                  01 / Start with intent
                </p>
                {preview ? (
                  <LearningPreviewCard preview={preview} prompt={prompt} />
                ) : (
                  <QuizPreview />
                )}
                <p className="max-w-md text-xs leading-5 text-[var(--ame-sugar)]/65">
                  A first signal is enough. ame turns it into a small, useful
                  beginning and leaves the next decision visible.
                </p>
              </div>
            </div>
          </section>

          <section
            id="benefits"
            className="grid gap-10 border-b border-border px-2 py-16 sm:px-4 md:grid-cols-[0.7fr_1.3fr] md:py-24"
          >
            <div>
              <p className="mb-2 font-mono text-xs tracking-[0.14em] text-primary">
                02 / The desk
              </p>
              <h2 className="mb-8 max-w-3xl text-3xl font-extrabold leading-tight tracking-[-0.04em] sm:text-4xl">
                Everything between{" "}
                <span className="whitespace-nowrap">
                  &quot;I want to learn&quot;
                </span>{" "}
                and{" "}
                <span className="whitespace-nowrap">
                  &quot;I know what to do next.&quot;
                </span>
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
                <h2 className="font-bold">Bring your agent</h2>
                <p className="text-sm text-muted-foreground">
                  One learner API — an agent can start and continue the same
                  journey you see in the web app.
                </p>
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              {["/llms.txt", "/skill.json", "/openapi.yaml"].map((href) => (
                <Button
                  key={href}
                  asChild
                  variant="outline"
                  size="sm"
                  className="rounded-full border-border font-mono text-xs font-normal"
                >
                  <Link href={href}>{href}</Link>
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
            <div className="self-start rounded-xl border border-border bg-card p-7">
              <h2 className="mb-3 text-3xl font-extrabold leading-tight tracking-[-0.04em]">
                Your next step is already clearer.
              </h2>
              <p className="mb-6 leading-6 text-muted-foreground">
                Start with one honest prompt and let ame shape a focused first
                learning moment.
              </p>
              <Button
                asChild
                className="rounded-full bg-primary font-bold text-primary-foreground hover:bg-primary/90"
              >
                <Link href={entryHref}>{entryLabel}</Link>
              </Button>
            </div>
          </section>
        </main>
      </div>

      <footer className="border-t border-border px-4 py-6 text-sm text-muted-foreground">
        <div className="mx-auto flex max-w-6xl flex-wrap justify-between gap-4">
          <span>© 2026 AME</span>
          <div className="flex gap-5">
            <Link href="/llms.txt" className="transition hover:text-primary">
              Docs
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
