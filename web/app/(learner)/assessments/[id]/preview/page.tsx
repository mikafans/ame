"use client";
import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useColorMode } from "@/components/ThemeRegistry";
import { tagColor } from "@/lib/tagColor";
import { Button } from "@/components/ui/button";
interface PreviewQuestion {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  orderIndex: number;
}
interface AssessmentDetail {
  id: string;
  title: string;
  mode?: string;
  status?: string;
  course?: string;
  objectives?: string[];
  questions: PreviewQuestion[];
}
const KIND_LABEL: Record<string, string> = {
  mc: "MC",
  tf: "T/F",
  short: "Short",
  essay: "Essay",
  code: "Code",
};
export default function AssessmentPreviewPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { mode } = useColorMode();
  const router = useRouter();
  const [assessment, setAssessment] = useState<AssessmentDetail | null>(null);
  const [starting, setStarting] = useState(false);
  useEffect(() => {
    api
      .GET("/v1/assessments/{id}", { params: { path: { id } } })
      .then(({ data }) => {
        if (data) setAssessment(data as any);
      })
      .catch(console.error);
  }, [id]);
  const handleStart = async () => {
    if (!assessment) return;
    setStarting(true);
    try {
      const { data } = await (api as any).POST("/v1/sessions", {
        body: { assessmentId: id, count: assessment.questions.length },
      });
      const sessionId = data?.sessionId ?? data?.session_id;
      if (sessionId) router.push(`/sessions/${sessionId}`);
    } finally {
      setStarting(false);
    }
  };
  if (!assessment)
    return (
      <div className="flex min-h-screen items-center justify-center">
        <span className="size-7 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      </div>
    );
  const totalPoints = assessment.questions.reduce((s, q) => s + q.points, 0);
  const kindCounts = assessment.questions.reduce(
    (acc, q) => ({ ...acc, [q.kind]: (acc[q.kind] ?? 0) + 1 }),
    {} as Record<string, number>,
  );
  const summary = Object.entries(kindCounts)
    .map(([k, n]) => `${n} ${KIND_LABEL[k] ?? k}`)
    .join(" · ");
  const sorted = [...assessment.questions].sort(
    (a, b) => a.orderIndex - b.orderIndex,
  );
  return (
    <main className="max-w-[860px] px-4 pb-16 pt-10 sm:px-12">
      <div className="mb-3 flex items-center gap-2 font-mono text-xs uppercase tracking-wider text-muted-foreground">
        <button className="underline" onClick={() => router.push("/explore")}>
          Explore
        </button>
        <span>›</span>
        <span>{assessment.title}</span>
      </div>
      <header className="mb-7">
        {assessment.course && (
          <span
            className="mb-3 inline-block rounded-full border px-2 py-1 text-xs"
            style={
              tagColor(
                assessment.course,
                mode === "dark",
              ) as React.CSSProperties
            }
          >
            {assessment.course}
          </span>
        )}
        <div className="mb-3 flex flex-wrap items-center gap-3">
          <h1 className="text-3xl font-medium">{assessment.title}</h1>
          {assessment.status && (
            <span className="rounded-full border px-2 py-1 text-xs uppercase">
              {assessment.status}
            </span>
          )}
        </div>
        {assessment.objectives?.length ? (
          <ul className="list-disc pl-5 text-sm leading-7 text-muted-foreground">
            {assessment.objectives.map((obj, i) => (
              <li key={i}>{obj}</li>
            ))}
          </ul>
        ) : null}
      </header>
      <div className="mb-6 flex gap-3 font-mono text-xs uppercase tracking-wider text-muted-foreground">
        <span>{assessment.questions.length} questions</span>
        <span>·</span>
        <span>{totalPoints} pts</span>
        <span>·</span>
        <span>{summary}</span>
      </div>
      <div className="overflow-hidden rounded-lg border border-border">
        {sorted.map((q, i) => (
          <div
            key={q.id}
            className="grid grid-cols-[24px_52px_1fr_48px] gap-4 border-b border-border px-5 py-4 last:border-0 hover:bg-muted/40 sm:grid-cols-[28px_70px_1fr_60px]"
          >
            <span className="font-mono text-xs text-muted-foreground">
              {i + 1}
            </span>
            <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              {KIND_LABEL[q.kind] || q.kind}
            </span>
            <span className="line-clamp-2 text-sm leading-6">{q.prompt}</span>
            <span className="whitespace-nowrap text-right font-mono text-xs text-muted-foreground">
              {q.points} pt{q.points !== 1 ? "s" : ""}
            </span>
          </div>
        ))}
      </div>
      <footer className="flex flex-col-reverse gap-3 border-t border-border pt-6 sm:flex-row sm:items-center sm:justify-between">
        <Button
          variant="outline"
          className="sm:min-w-44"
          onClick={() => router.push("/explore")}
        >
          Back to Explore
        </Button>
        <Button
          className="sm:min-w-44"
          onClick={handleStart}
          disabled={
            starting ||
            assessment.status === "draft" ||
            assessment.questions.length === 0
          }
        >
          {starting
            ? "Starting…"
            : assessment.status === "draft"
              ? "Cannot start draft"
              : assessment.questions.length === 0
                ? "No questions available"
                : assessment.mode === "graded"
                  ? "Start exam"
                  : "Start assessment"}
        </Button>
      </footer>
    </main>
  );
}
