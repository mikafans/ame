import Link from "next/link";
import { ArrowLeft } from "lucide-react";
import { Logo } from "@/components/Logo";
import { MarkdownView } from "@/components/MarkdownView";
import { readPublicDoc } from "@/lib/publicDocs";

export const metadata = {
  title: "About AME",
  description:
    "What AME is, what counts as learning progress, and what an agent may and may not do.",
};

export default function AboutPage() {
  const principles = readPublicDoc("learning-principles.md");

  return (
    <div className="min-h-screen bg-white text-neutral-950 dark:bg-neutral-950 dark:text-neutral-100">
      <header className="border-b border-neutral-200 dark:border-neutral-800">
        <div className="mx-auto flex max-w-5xl items-center justify-between px-4 py-4 sm:px-6">
          <Logo size={32} />
          <Link
            href="/"
            className="inline-flex items-center gap-2 text-sm font-medium text-neutral-600 transition hover:text-neutral-950 dark:text-neutral-300 dark:hover:text-white"
          >
            <ArrowLeft className="size-4" />
            Back to AME
          </Link>
        </div>
      </header>

      <main className="mx-auto max-w-5xl px-4 py-10 sm:px-6 sm:py-14">
        <article className="rounded-2xl border border-neutral-200 bg-white px-5 py-2 shadow-sm dark:border-neutral-800 dark:bg-neutral-900 sm:px-10">
          <MarkdownView content={principles} />
        </article>
      </main>
    </div>
  );
}
