import Link from "next/link";
import { ArrowLeft } from "lucide-react";
import { Logo } from "@/components/Logo";
import { MarkdownView } from "@/components/MarkdownView";
import { readPublicDoc } from "@/lib/publicDocs";

export const metadata = {
  title: "Self-hosting AME",
  description: "Run AME on your own Linux host with Docker or Podman Compose.",
};

export default function SelfHostingPage() {
  const guide = readPublicDoc("self-hosting.md");

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
          <MarkdownView content={guide} />
        </article>
      </main>
    </div>
  );
}
