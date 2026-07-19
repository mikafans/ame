"use client";
import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { LearningObjectives } from "@/components/LearningObjectives";
import { Button } from "@/components/ui/button";
interface Exam {
  id: string;
  name: string;
  description?: string;
  status: string;
  durationMin?: number;
  passingPoints?: number;
  objectives: string[];
  totalPoints: number;
  course?: string;
  completed?: boolean;
  lastSessionId?: string | null;
  sections:
    | {
        id: string;
        title: string;
        weight: number;
        itemsCount: number;
        mix?: string;
      }[]
    | null;
}
type Tab = "all" | "published" | "draft";
export default function ExamsPage() {
  const router = useRouter();
  const [exams, setExams] = useState<Exam[]>([]);
  const [selected, setSelected] = useState<Exam | null>(null);
  const [tab, setTab] = useState<Tab>("all");
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState(false);
  useEffect(() => {
    api
      .GET("/v1/assessments", { params: { query: { mode: "graded" } } })
      .then(({ data }) => {
        const list = data
          ? "assessments" in data
            ? (data as any).assessments
            : data
          : [];
        setExams(
          (list as any[]).map((d) => ({
            id: d.id,
            name: d.title,
            description: d.description,
            status: d.status === "active" ? "published" : d.status,
            durationMin: d.durationMin,
            passingPoints: d.passingPoints,
            objectives: d.objectives ?? [],
            totalPoints: d.totalPoints ?? 0,
            course: d.course,
            completed: d.completed,
            lastSessionId: d.lastSessionId,
            sections: null,
          })),
        );
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);
  const filtered =
    tab === "all" ? exams : exams.filter((e) => e.status === tab);
  const start = async () => {
    if (!selected) return;
    setStarting(true);
    try {
      const { data } = await (api as any).POST("/v1/sessions", {
        body: { assessmentId: selected.id },
      });
      const id = data?.sessionId ?? data?.session_id;
      if (id) router.push(`/sessions/${id}`);
    } finally {
      setStarting(false);
    }
  };
  if (loading)
    return <div className="p-12 text-muted-foreground">Loading exams…</div>;
  return (
    <main className="grid gap-6 px-6 py-12 lg:grid-cols-[320px_1fr] sm:px-12">
      <section>
        <header className="mb-6">
          <p className="text-xs uppercase tracking-widest text-muted-foreground">
            Assessments
          </p>
          <h1 className="mt-2 text-3xl font-medium">Exams</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            Take graded assessments and track your results.
          </p>
        </header>
        <div className="mb-4 flex rounded-md border border-border">
          {(["all", "published", "draft"] as Tab[]).map((value) => (
            <button
              key={value}
              className={`flex-1 px-3 py-2 text-xs capitalize ${tab === value ? "bg-primary text-primary-foreground" : "text-muted-foreground"}`}
              onClick={() => setTab(value)}
            >
              {value === "published"
                ? "Active"
                : value === "draft"
                  ? "Drafts"
                  : "All"}
            </button>
          ))}
        </div>
        <div className="space-y-2">
          {filtered.map((exam) => (
            <button
              key={exam.id}
              className={`w-full rounded-lg border p-4 text-left ${selected?.id === exam.id ? "border-primary bg-primary/5" : "border-border hover:bg-muted/40"}`}
              onClick={() => setSelected(exam)}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-medium">{exam.name}</span>
                <span className="rounded-full border px-2 py-0.5 text-xs">
                  {exam.status}
                </span>
              </div>
              <p className="mt-1 text-xs text-muted-foreground">
                {exam.course || "General"} · {exam.totalPoints} pts
              </p>
            </button>
          ))}
        </div>
        {filtered.length === 0 && (
          <p className="text-sm text-muted-foreground">No exams found.</p>
        )}
      </section>
      {selected ? (
        <article className="rounded-lg border border-border p-6 sm:p-8">
          <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
            <div>
              <p className="mb-2 text-xs uppercase tracking-widest text-muted-foreground">
                {selected.course || "Graded assessment"}
              </p>
              <h2 className="text-2xl font-medium">{selected.name}</h2>
              <p className="mt-2 text-sm text-muted-foreground">
                {selected.description || "No description provided."}
              </p>
            </div>
            <span className="rounded-full border px-2 py-1 text-xs uppercase">
              {selected.status}
            </span>
          </div>
          <div className="mb-6 flex gap-4 font-mono text-xs text-muted-foreground">
            <span>{selected.totalPoints} points</span>
            {selected.durationMin && (
              <span>{selected.durationMin} minutes</span>
            )}
            {selected.passingPoints && (
              <span>Pass: {selected.passingPoints}</span>
            )}
          </div>
          <LearningObjectives items={selected.objectives} compact />
          {selected.sections?.length ? (
            <div className="mt-6 space-y-2">
              {selected.sections.map((section) => (
                <div
                  key={section.id}
                  className="flex justify-between rounded border border-border p-3 text-sm"
                >
                  <span>{section.title}</span>
                  <span className="text-muted-foreground">
                    {section.itemsCount} questions
                  </span>
                </div>
              ))}
            </div>
          ) : null}
          <div className="mt-8 flex flex-wrap gap-3">
            <Button
              onClick={start}
              disabled={starting || selected.status === "draft"}
            >
              {starting
                ? "Starting…"
                : selected.status === "draft"
                  ? "Draft cannot start"
                  : selected.completed
                    ? "Retake exam"
                    : "Start exam"}
            </Button>
            {selected.lastSessionId && (
              <Button
                variant="outline"
                onClick={() =>
                  router.push(`/sessions/${selected.lastSessionId}/results`)
                }
              >
                View last result
              </Button>
            )}
          </div>
        </article>
      ) : (
        <div className="flex min-h-96 items-center justify-center rounded-lg border border-dashed border-border text-sm text-muted-foreground">
          Select an exam to view details.
        </div>
      )}
    </main>
  );
}
