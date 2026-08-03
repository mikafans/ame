import { MarkdownView } from "@/components/MarkdownView";
import { readPublicDoc } from "@/lib/publicDocs";

export default function AboutPage() {
  const principles = readPublicDoc("learning-principles.md");

  return (
    <main className="mx-auto max-w-3xl space-y-6 py-4">
      <header>
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          About
        </p>
        <h1 className="mt-3 text-3xl font-bold tracking-tight">
          How AME works.
        </h1>
      </header>
      <article className="border border-border bg-card p-6 sm:p-8">
        <MarkdownView content={principles} />
      </article>
    </main>
  );
}
