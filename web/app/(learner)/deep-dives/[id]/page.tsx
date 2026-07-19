"use client";
import { use, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { PageShell } from "@/components/PageShell";
import { MarkdownView } from "@/components/MarkdownView";
import { formatDate } from "@/utils/format";
import { Button } from "@/components/ui/button";
import { ArrowLeft, Archive, History } from "lucide-react";
interface DeepDive {
  id: string;
  questionId: string;
  status: string;
  reason?: string | null;
  bodyMarkdown?: string | null;
  createdAt: string;
  updatedAt: string;
  category?: string | null;
  userNote?: string | null;
  questionPrompt: string;
  questionKind: string;
  questionTags: string[];
  assessmentTitle?: string | null;
  course?: string | null;
}
interface Revision {
  id: string;
  revision: number;
  bodyMarkdown?: string | null;
  category?: string | null;
  userNote?: string | null;
  createdAt: string;
}
export default function DeepDiveDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const router = useRouter();
  const [item, setItem] = useState<DeepDive | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [note, setNote] = useState("");
  const [noteStatus, setNoteStatus] = useState("idle");
  const [revisions, setRevisions] = useState<Revision[]>([]);
  const [showRevisions, setShowRevisions] = useState(false);
  useEffect(() => {
    (api as any)
      .GET("/v1/deep-dives/{id}", { params: { path: { id } } })
      .then(({ data }: any) => {
        setItem(data ?? null);
        setNote(data?.userNote ?? "");
      })
      .catch(() => setError("Could not load this deep dive."))
      .finally(() => setLoading(false));
  }, [id]);
  useEffect(() => {
    if (!item || note === (item.userNote ?? "")) return;
    const timer = setTimeout(async () => {
      setNoteStatus("saving");
      try {
        const { data } = await (api as any).PATCH("/v1/deep-dives/{id}", {
          params: { path: { id } },
          body: { userNote: note },
        });
        if (data) setItem(data);
        setNoteStatus("saved");
      } catch {
        setNoteStatus("idle");
      }
    }, 1000);
    return () => clearTimeout(timer);
  }, [note, item, id]);
  useEffect(() => {
    if (!showRevisions) return;
    setRevisions([]);
    (api as any)
      .GET("/v1/deep-dives/{id}/revisions", { params: { path: { id } } })
      .then(({ data }: any) => setRevisions(data?.revisions ?? []))
      .catch(() => {});
  }, [showRevisions, id]);
  const updateStatus = async (status: "archived" | "requested") => {
    if (!item) return;
    setSaving(true);
    try {
      const { data } = await (api as any).PATCH("/v1/deep-dives/{id}", {
        params: { path: { id: item.id } },
        body: { status },
      });
      if (data) setItem(data);
    } catch {
      setError("Could not update this deep dive.");
    } finally {
      setSaving(false);
    }
  };
  if (loading)
    return <div className="p-12 text-muted-foreground">Loading deep dive…</div>;
  if (!item)
    return (
      <div className="p-12">
        <p className="mb-4 text-muted-foreground">
          {error || "Deep dive not found."}
        </p>
        <Button variant="outline" onClick={() => router.push("/deep-dives")}>
          Back to deep dives
        </Button>
      </div>
    );
  return (
    <PageShell
      kicker="Deep dive"
      title={item.questionPrompt}
      subtitle={`${item.questionKind} · ${item.status}`}
      maxWidth={1000}
    >
      <div className="mb-6 flex flex-wrap gap-2">
        <Button variant="outline" onClick={() => router.push("/deep-dives")}>
          <ArrowLeft size={16} />
          Back
        </Button>
        {item.status === "archived" ? (
          <Button
            variant="outline"
            onClick={() => updateStatus("requested")}
            disabled={saving}
          >
            Restore
          </Button>
        ) : (
          <Button
            variant="outline"
            onClick={() => updateStatus("archived")}
            disabled={saving}
          >
            <Archive size={16} />
            Archive
          </Button>
        )}
        <Button variant="ghost" onClick={() => setShowRevisions(true)}>
          <History size={16} />
          Revisions
        </Button>
      </div>
      {error && (
        <div
          role="alert"
          className="mb-4 rounded border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          {error}
        </div>
      )}
      <div className="grid gap-6 lg:grid-cols-[1fr_280px]">
        <article className="rounded-lg border border-border p-6">
          <p className="mb-5 text-xs text-muted-foreground">
            Updated {formatDate(item.updatedAt)}
            {item.category ? ` · ${item.category}` : ""}
          </p>
          {item.bodyMarkdown ? (
            <MarkdownView content={item.bodyMarkdown} />
          ) : (
            <p className="text-sm text-muted-foreground">
              No explanation has been published yet.
            </p>
          )}
        </article>
        <aside className="space-y-5">
          <section className="rounded-lg border border-border p-5">
            <h2 className="mb-3 text-sm font-semibold">Your note</h2>
            <textarea
              className="min-h-32 w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder="Add a personal note…"
            />
            <p className="mt-2 text-xs text-muted-foreground">
              {noteStatus === "saving"
                ? "Saving…"
                : noteStatus === "saved"
                  ? "Saved"
                  : "Autosaves after you stop typing."}
            </p>
          </section>
          <section className="rounded-lg border border-border p-5">
            <h2 className="mb-3 text-sm font-semibold">Question context</h2>
            <p className="text-sm text-muted-foreground">
              {item.assessmentTitle || "Unassigned assessment"}
            </p>
            <div className="mt-3 flex flex-wrap gap-1">
              {item.questionTags?.map((tag) => (
                <span
                  key={tag}
                  className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground"
                >
                  {tag}
                </span>
              ))}
            </div>
          </section>
        </aside>
      </div>
      {showRevisions && (
        <div className="fixed inset-0 z-50 flex justify-end bg-black/50">
          <aside className="h-full w-full max-w-md overflow-y-auto border-l border-border bg-background p-6">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold">Revision history</h2>
              <Button variant="ghost" onClick={() => setShowRevisions(false)}>
                Close
              </Button>
            </div>
            <div className="mt-6 space-y-3">
              {revisions.map((revision) => (
                <article
                  key={revision.id}
                  className="rounded border border-border p-4"
                >
                  <p className="text-xs text-muted-foreground">
                    Revision {revision.revision} ·{" "}
                    {formatDate(revision.createdAt)}
                  </p>
                  <p className="mt-2 line-clamp-4 text-sm">
                    {revision.bodyMarkdown || "No body"}
                  </p>
                </article>
              ))}
            </div>
          </aside>
        </div>
      )}
    </PageShell>
  );
}
