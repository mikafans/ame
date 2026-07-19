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
    title: "Study plans that adapt",
    body: "Plans start from your performance and change as you answer. Spend your next session where it matters most.",
  },
  {
    number: "02",
    title: "Questions, organized",
    body: "Collect questions from anywhere into tagged banks. Turn a bank into a timed assessment when you are ready.",
  },
  {
    number: "03",
    title: "Progress you can see",
    body: "Track movement by topic so you can replace 'I should study' with a clear next step.",
  },
];

const faqs = [
  {
    question: "Can I use AME without an AI assistant?",
    answer:
      "Yes. The web app works on its own. Agent support is an optional way to connect an assistant to your assessments and study workflow.",
  },
  {
    question: "Can I self-host AME?",
    answer:
      "AME is designed to be self-hostable. Follow the repository deployment documentation to run the platform on your own infrastructure.",
  },
  {
    question: "What should I do first?",
    answer:
      "Create an account, open Explore, and start an assessment from the question bank. Your first session gives you a useful baseline.",
  },
];

function QuizPreview() {
  return (
    <div
      role="img"
      aria-label="Preview of an adaptive multiple-choice question"
      className="rounded-xl bg-white p-4 text-neutral-900 shadow-[0_24px_60px_rgba(0,0,0,0.28)] sm:p-6"
    >
      <div className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <span className="font-mono text-xs text-neutral-500">Q 7 / 20</span>
          <span className="rounded-full bg-blue-100 px-3 py-1 text-[11px] text-blue-700">
            Adaptive · ELO 1480
          </span>
        </div>
        <div className="h-1 overflow-hidden rounded-full bg-neutral-200">
          <div className="h-full w-[35%] bg-blue-600" />
        </div>
        <p className="pt-1 text-base font-bold sm:text-[17px]">
          Which data structure gives O(1) average lookup?
        </p>
        <div className="space-y-2">
          <div className="rounded-lg border border-neutral-300 px-3 py-2.5">
            Binary search tree
          </div>
          <div className="flex items-center justify-between rounded-lg border-2 border-blue-600 bg-blue-100 px-3 py-2 text-blue-950">
            <span>Hash table</span>
            <Check className="size-5 text-lime-700" />
          </div>
          <div className="rounded-lg border border-neutral-300 px-3 py-2.5">
            Linked list
          </div>
        </div>
        <div className="flex items-center justify-between gap-3 pt-1">
          <span className="text-xs text-neutral-500">
            Hash maps · your weakest topic
          </span>
          <span className="rounded-full bg-neutral-950 px-4 py-2 text-center text-sm text-white">
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
    <div className="border border-neutral-200 bg-white p-5 text-neutral-900 shadow-[10px_10px_0_#dbeafe] sm:p-7">
      <div className="space-y-5">
        <div>
          <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-blue-600">
            {preview.templateId} · v{preview.templateVersion}
          </p>
          <h2 className="mt-2 text-xl font-bold tracking-tight">
            {preview.promise}
          </h2>
        </div>
        <div>
          <p className="mb-2 text-xs font-bold uppercase tracking-[0.12em] text-neutral-500">
            Your first outcomes
          </p>
          <ul className="space-y-2 text-sm leading-5">
            {preview.objectives.map((objective) => (
              <li key={objective.statement} className="flex gap-2">
                <Check className="mt-0.5 size-4 shrink-0 text-lime-700" />
                <span>{objective.statement}</span>
              </li>
            ))}
          </ul>
        </div>
        <div className="rounded-lg bg-neutral-100 p-3 text-sm">
          <p className="font-semibold">
            First step: {preview.firstActivity.title}
          </p>
          <p className="mt-1 text-neutral-600">
            {preview.firstActivity.purpose} · about{" "}
            {preview.firstActivity.estimatedMinutes} minutes
          </p>
        </div>
        <Button
          asChild
          className="w-full rounded-full bg-blue-600 font-bold text-white hover:bg-blue-700"
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
  const entryHref = signedIn ? "/explore" : "/login?tab=signup";
  const entryLabel = signedIn ? "Open Explore" : "Sign up free";
  const sampleHref = signedIn ? "/explore" : "#benefits";
  const sampleLabel = signedIn ? "Open Explore" : "See how it works";

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
    <div className="min-h-screen bg-[#f5f5f0] text-neutral-950 dark:bg-neutral-950 dark:text-neutral-100">
      <div className="mx-auto max-w-7xl px-4 sm:px-8">
        <header className="flex min-h-20 items-center justify-between gap-4 border-b border-neutral-200 dark:border-neutral-800">
          <Logo size={32} />
          <div className="flex items-center gap-2 sm:gap-6">
            <nav className="hidden items-center gap-6 text-sm text-neutral-600 dark:text-neutral-300 sm:flex">
              <a
                className="transition hover:text-neutral-950 dark:hover:text-white"
                href="#benefits"
              >
                Features
              </a>
              <a
                className="transition hover:text-neutral-950 dark:hover:text-white"
                href="#agents"
              >
                For agents
              </a>
              <a
                className="transition hover:text-neutral-950 dark:hover:text-white"
                href="#faq"
              >
                FAQ
              </a>
            </nav>
            <button
              type="button"
              aria-label={mode === "dark" ? "Use light mode" : "Use dark mode"}
              onClick={toggle}
              className="inline-flex size-8 items-center justify-center rounded-lg text-neutral-600 outline-none transition hover:bg-neutral-100 focus-visible:ring-2 focus-visible:ring-blue-600 dark:text-neutral-300 dark:hover:bg-neutral-800"
            >
              {mode === "dark" ? (
                <Sun className="size-4" />
              ) : (
                <Moon className="size-4" />
              )}
            </button>
            <Button
              asChild
              className="rounded-full bg-neutral-950 px-5 text-white hover:bg-neutral-800 dark:bg-white dark:text-neutral-950 dark:hover:bg-neutral-200"
            >
              <Link href={entryHref}>{entryLabel}</Link>
            </Button>
          </div>
        </header>

        <main>
          <section className="relative overflow-hidden border-x border-b border-neutral-200 bg-neutral-950 px-6 py-14 text-white sm:px-10 md:px-14 md:py-20">
            <div className="absolute inset-y-0 right-0 hidden w-1/3 border-l border-neutral-800 bg-[linear-gradient(90deg,transparent_0%,rgba(255,255,255,0.04)_100%)] md:block" />
            <div className="relative grid items-start gap-12 md:grid-cols-[0.85fr_1.15fr] md:gap-20">
              <div className="space-y-8">
                <span className="inline-flex border-l-2 border-lime-400 pl-3 font-mono text-[11px] tracking-[0.14em] text-lime-300">
                  AME / LEARNING DESK
                </span>
                <h1 className="max-w-xl text-[42px] font-extrabold leading-[0.96] tracking-[-0.06em] sm:text-6xl md:text-[72px]">
                  Study what you don&apos;t know yet.
                </h1>
                <p className="max-w-xl text-[17px] leading-7 text-neutral-400 sm:text-[19px]">
                  Tell AME what you want to learn. It turns your intent into a
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
                      placeholder="I'd like to learn music theory"
                      className="min-h-11 flex-1 rounded-full border border-neutral-700 bg-neutral-900 px-5 text-sm text-white outline-none placeholder:text-neutral-500 focus:border-lime-400 focus:ring-2 focus:ring-lime-400/30"
                    />
                    <Button
                      type="submit"
                      disabled={previewLoading || !prompt.trim()}
                      className="min-h-11 rounded-full bg-lime-400 px-5 font-bold text-neutral-950 hover:bg-lime-300"
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
                <div className="flex flex-col gap-3 border-t border-neutral-800 pt-5 sm:flex-row">
                  <Button
                    asChild
                    className="rounded-full bg-blue-600 px-6 py-3 font-bold text-white hover:bg-blue-700"
                  >
                    <Link href={entryHref}>
                      {entryLabel}
                      <ArrowRight className="size-4" />
                    </Link>
                  </Button>
                  <Button
                    asChild
                    variant="outline"
                    className="rounded-full border-neutral-600 bg-transparent px-6 py-3 text-white hover:border-white hover:bg-white/10 hover:text-white"
                  >
                    <Link href={sampleHref}>{sampleLabel}</Link>
                  </Button>
                </div>
              </div>
              <div className="space-y-4 md:pt-8">
                <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-neutral-500">
                  01 / Start with intent
                </p>
                {preview ? (
                  <LearningPreviewCard preview={preview} prompt={prompt} />
                ) : (
                  <QuizPreview />
                )}
                <p className="max-w-md text-xs leading-5 text-neutral-500">
                  A first signal is enough. AME turns it into a small, useful
                  beginning and leaves the next decision visible.
                </p>
              </div>
            </div>
          </section>

          <section
            id="benefits"
            className="grid gap-10 border-b border-neutral-200 px-2 py-16 sm:px-4 md:grid-cols-[0.7fr_1.3fr] md:py-24 dark:border-neutral-800"
          >
            <div>
              <p className="mb-2 font-mono text-xs tracking-[0.14em] text-blue-600">
                02 / The desk
              </p>
              <h2 className="mb-8 max-w-3xl text-3xl font-extrabold leading-tight tracking-[-0.04em] sm:text-4xl">
                Everything between{" "}
                <span className="whitespace-nowrap">
                  &quot;I should study&quot;
                </span>{" "}
                and{" "}
                <span className="whitespace-nowrap">&quot;I passed.&quot;</span>
              </h2>
            </div>
            <div className="grid gap-0 border-t border-neutral-200 dark:border-neutral-800">
              {benefits.map((benefit) => (
                <article
                  key={benefit.number}
                  className="grid gap-4 border-b border-neutral-200 py-5 sm:grid-cols-[60px_0.8fr_1.2fr] sm:items-start dark:border-neutral-800"
                >
                  <span className="font-mono text-sm text-blue-600">
                    {benefit.number}
                  </span>
                  <h3 className="text-lg font-bold">{benefit.title}</h3>
                  <p className="leading-6 text-neutral-600 dark:text-neutral-400">
                    {benefit.body}
                  </p>
                </article>
              ))}
            </div>
          </section>

          <section
            id="agents"
            className="flex flex-col gap-5 border-b border-neutral-200 px-2 py-8 sm:px-4 md:flex-row md:items-center md:justify-between dark:border-neutral-800"
          >
            <div className="flex gap-4">
              <span className="font-mono text-xs text-blue-600">03</span>
              <div>
                <h2 className="font-bold">Bring your AI assistant</h2>
                <p className="text-sm text-neutral-600 dark:text-neutral-400">
                  First-class agent API — connect an assistant to your
                  assessments and study workflow.
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
                  className="rounded-full border-neutral-300 font-mono text-xs font-normal dark:border-neutral-700"
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
              <div className="divide-y divide-neutral-200 dark:divide-neutral-800">
                {faqs.map((faq) => (
                  <div key={faq.question} className="py-5">
                    <h3 className="mb-2 font-bold">{faq.question}</h3>
                    <p className="leading-6 text-neutral-600 dark:text-neutral-400">
                      {faq.answer}
                    </p>
                  </div>
                ))}
              </div>
            </div>
            <div className="self-start rounded-xl border border-neutral-200 bg-neutral-50 p-7 dark:border-neutral-800 dark:bg-neutral-900">
              <h2 className="mb-3 text-3xl font-extrabold leading-tight tracking-[-0.04em]">
                Your next exam is already easier.
              </h2>
              <p className="mb-6 leading-6 text-neutral-600 dark:text-neutral-400">
                Start with a few questions and turn your weakest topics into a
                focused practice session.
              </p>
              <Button
                asChild
                className="rounded-full bg-blue-600 font-bold text-white hover:bg-blue-700"
              >
                <Link href={entryHref}>{entryLabel}</Link>
              </Button>
            </div>
          </section>
        </main>
      </div>

      <footer className="border-t border-neutral-200 px-4 py-6 text-sm text-neutral-500 dark:border-neutral-800">
        <div className="mx-auto flex max-w-6xl flex-wrap justify-between gap-4">
          <span>© 2026 AME</span>
          <div className="flex gap-5">
            <Link
              href="/llms.txt"
              className="transition hover:text-neutral-950 dark:hover:text-white"
            >
              Docs
            </Link>
            <a
              href="https://github.com/mikafans/ame"
              className="transition hover:text-neutral-950 dark:hover:text-white"
            >
              GitHub
            </a>
            <Link
              href="/self-hosting"
              className="transition hover:text-neutral-950 dark:hover:text-white"
            >
              Self-hosting
            </Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
