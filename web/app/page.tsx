"use client";

import Link from "next/link";
import { ArrowRight, Check, Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Logo } from "@/components/Logo";
import { useColorMode } from "@/components/ThemeRegistry";
import { useAuth } from "@/hooks/useAuth";

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

export default function LandingPage() {
  const { user } = useAuth();
  const { mode, toggle } = useColorMode();
  const signedIn = !!user;
  const entryHref = signedIn ? "/explore" : "/login?tab=signup";
  const entryLabel = signedIn ? "Open Explore" : "Sign up free";
  const sampleHref = signedIn ? "/explore" : "#benefits";
  const sampleLabel = signedIn ? "Open Explore" : "See how it works";

  return (
    <div className="min-h-screen bg-white text-neutral-950 dark:bg-neutral-950 dark:text-neutral-100">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <header className="flex min-h-16 items-center justify-between gap-4">
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
          <section className="rounded-none bg-neutral-950 px-6 py-14 text-white sm:px-10 md:rounded-lg md:px-14 md:py-20">
            <div className="grid items-center gap-12 md:grid-cols-2 md:gap-16">
              <div className="space-y-6">
                <span className="inline-flex rounded-full border border-lime-700 px-3 py-1 font-mono text-[11px] tracking-[0.12em] text-lime-300">
                  OPEN · SELF-HOSTABLE
                </span>
                <h1 className="max-w-xl text-[42px] font-extrabold leading-[0.99] tracking-[-0.06em] sm:text-6xl md:text-[66px]">
                  Study what you don&apos;t know yet.
                </h1>
                <p className="max-w-xl text-[17px] leading-7 text-neutral-400 sm:text-[19px]">
                  AME builds question banks, tracks every answer, and adapts
                  each session to your weakest topics — so no minute of studying
                  is wasted.
                </p>
                <div className="flex flex-col gap-3 sm:flex-row">
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
              <QuizPreview />
            </div>
          </section>

          <section id="benefits" className="px-2 py-16 sm:px-4 md:py-20">
            <p className="mb-2 font-mono text-xs tracking-[0.14em] text-blue-600">
              WHY AME
            </p>
            <h2 className="mb-8 max-w-3xl text-3xl font-extrabold leading-tight tracking-[-0.04em] sm:text-4xl">
              Everything between{" "}
              <span className="whitespace-nowrap">
                &quot;I should study&quot;
              </span>{" "}
              and{" "}
              <span className="whitespace-nowrap">&quot;I passed.&quot;</span>
            </h2>
            <div className="grid gap-4 md:grid-cols-3">
              {benefits.map((benefit) => (
                <article
                  key={benefit.number}
                  className="rounded-xl border border-neutral-200 p-6 shadow-[0_4px_0_#f0f0f0] dark:border-neutral-800 dark:shadow-[0_4px_0_#171717]"
                >
                  <span className="mb-4 inline-flex size-9 items-center justify-center rounded-full bg-blue-100 font-mono text-sm text-blue-700 dark:bg-blue-950 dark:text-blue-300">
                    {benefit.number}
                  </span>
                  <h3 className="mb-2 text-lg font-bold">{benefit.title}</h3>
                  <p className="leading-6 text-neutral-600 dark:text-neutral-400">
                    {benefit.body}
                  </p>
                </article>
              ))}
            </div>
          </section>

          <section
            id="agents"
            className="flex flex-col gap-4 bg-neutral-100 px-6 py-6 dark:bg-neutral-900 md:flex-row md:items-center md:justify-between"
          >
            <div>
              <h2 className="font-bold">Bring your AI assistant</h2>
              <p className="text-sm text-neutral-600 dark:text-neutral-400">
                First-class agent API — connect an assistant to your assessments
                and study workflow.
              </p>
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
            className="grid gap-12 px-2 py-16 sm:px-4 md:grid-cols-[1.4fr_1fr] md:py-20"
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
              href="/llms.txt"
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
