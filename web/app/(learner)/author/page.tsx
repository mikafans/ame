"use client";
import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { Plus, Trash2 } from "lucide-react";
interface Assessment {
  id: string;
  title: string;
  status: string;
  course?: string;
  questionCount?: number;
}
export default function AuthorIndexPage() {
  const router = useRouter();
  const [drafts, setDrafts] = useState<Assessment[] | null>(null);
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<Assessment | null>(null);
  const load = () =>
    api
      .GET("/v1/assessments", { params: { query: { status: "draft" } } })
      .then(({ data }) => {
        const list = data
          ? "assessments" in data
            ? (data as any).assessments
            : data
          : [];
        setDrafts(
          (list as any[]).map((d) => ({
            id: d.id,
            title: d.title,
            status: d.status,
            course: d.course ?? undefined,
            questionCount: d.questionCount,
          })),
        );
      })
      .catch(() => setDrafts([]));
  useEffect(() => {
    load();
  }, []);
  async function discard(id: string) {
    setConfirm(null);
    setDeleting(id);
    setDrafts((d) => d?.filter((q) => q.id !== id) ?? d);
    try {
      await api.DELETE("/v1/assessments/{id}", { params: { path: { id } } });
    } catch {
      load();
    } finally {
      setDeleting(null);
    }
  }
  async function createNew() {
    setCreating(true);
    try {
      const { data } = await api.POST("/v1/assessments", {
        body: {
          title: "Untitled assessment",
          description: null,
          mode: "practice",
          objectives: [],
          course: null,
          durationMin: null,
          timeLimitSeconds: null,
          passingPoints: null,
          showResultsDuring: false,
          affectsRating: true,
          method: "manual",
        },
      });
      if (data?.id) router.push(`/author/${data.id}`);
    } finally {
      setCreating(false);
    }
  }
  return (
    <PageShell
      kicker="Teach"
      title="Author studio"
      action={
        <Button onClick={createNew} disabled={creating}>
          <Plus size={16} />
          {creating ? "Creating…" : "New assessment"}
        </Button>
      }
    >
      {drafts === null ? (
        <div className="flex justify-center py-12">
          <span className="size-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        </div>
      ) : drafts.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No drafts yet — create a new assessment to get started.
        </p>
      ) : (
        <div className="space-y-3">
          <p className="font-mono text-xs text-muted-foreground">
            {drafts.length} draft{drafts.length !== 1 ? "s" : ""}
          </p>
          {drafts.map((q) => (
            <div
              key={q.id}
              className="flex items-stretch overflow-hidden rounded-lg border border-border"
            >
              <button
                className="flex flex-1 items-center justify-between gap-4 px-5 py-4 text-left hover:bg-muted/40"
                onClick={() => router.push(`/author/${q.id}`)}
              >
                <div>
                  <p className="text-sm font-medium">{q.title}</p>
                  {q.course && (
                    <p className="mt-1 text-xs text-muted-foreground">
                      {q.course}
                    </p>
                  )}
                </div>
                <div className="flex items-center gap-3 text-xs text-muted-foreground">
                  {q.questionCount != null && (
                    <span className="font-mono">{q.questionCount} q</span>
                  )}
                  <span className="rounded-full border px-2 py-0.5">draft</span>
                </div>
              </button>
              <button
                className="border-l border-border px-4 text-muted-foreground hover:bg-red-500/10 hover:text-red-500"
                onClick={() => setConfirm(q)}
                disabled={deleting === q.id}
                title="Discard draft"
              >
                <Trash2 size={17} />
              </button>
            </div>
          ))}
        </div>
      )}{" "}
      {confirm && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
          role="dialog"
          aria-modal="true"
        >
          <div className="w-full max-w-sm rounded-xl border border-border bg-background p-6 shadow-xl">
            <h2 className="text-lg font-semibold">Discard draft?</h2>
            <p className="mt-3 text-sm text-muted-foreground">
              <strong>&ldquo;{confirm.title}&rdquo;</strong> and all its
              questions will be permanently deleted. This cannot be undone.
            </p>
            <div className="mt-6 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setConfirm(null)}>
                Cancel
              </Button>
              <Button variant="destructive" onClick={() => discard(confirm.id)}>
                Discard
              </Button>
            </div>
          </div>
        </div>
      )}
    </PageShell>
  );
}
