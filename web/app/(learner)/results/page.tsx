"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { formatDateTime } from "@/utils/format";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
import { CalendarDays, ExternalLink, History } from "lucide-react";

interface SessionSummary {
  id: string;
  kind: "quiz" | "exam" | "practice";
  status: string;
  assessmentTitle?: string | null;
  pointsAwarded?: number | null;
  maxPoints?: number | null;
  finishedAt?: string | null;
  attemptNumber: number;
  totalAttempts: number;
}

const toneClasses = {
  neutral: "border-border text-muted-foreground",
  success: "border-emerald-500/40 text-emerald-600 dark:text-emerald-400",
  warning: "border-amber-500/40 text-amber-600 dark:text-amber-400",
  error: "border-red-500/40 text-red-600 dark:text-red-400",
};

export default function ResultsHistoryPage() {
  const router = useRouter();
  const [loading, setLoading] = useState(true);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [total, setTotal] = useState(0);

  useEffect(() => {
    setLoading(true);
    api
      .GET(
        "/v1/sessions" as never,
        {
          params: { query: { limit: rowsPerPage, offset: page * rowsPerPage } },
        } as never,
      )
      .then(({ data, error }) => {
        if (error) {
          if ((error as any).status !== 401)
            setError("Failed to load attempt history.");
        } else if (data) {
          setSessions((data as any).sessions || []);
          setTotal((data as any).total || 0);
        }
      })
      .catch((err) => {
        console.error(err);
        setError("Could not establish connection to the API.");
      })
      .finally(() => setLoading(false));
  }, [page, rowsPerPage]);

  if (loading)
    return (
      <div className="flex min-h-[80vh] items-center justify-center">
        <span className="size-8 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      </div>
    );

  return (
    <PageShell
      kicker="History"
      title="Attempt History"
      subtitle="Review your recent practice sessions, quiz attempts, and exam results."
    >
      {error && (
        <div
          role="alert"
          className="mb-6 rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          {error}
        </div>
      )}
      {sessions.length === 0 ? (
        <div className="flex flex-col items-center rounded-lg border border-border px-4 py-20 text-center">
          <div className="mb-4 flex size-14 items-center justify-center rounded-full bg-muted text-muted-foreground">
            <History size={28} />
          </div>
          <h2 className="text-lg font-medium">No recent attempts found</h2>
          <p className="mb-5 mt-2 max-w-[400px] text-sm text-muted-foreground">
            It looks like you haven&apos;t taken any quizzes or exams yet. Once
            you complete an assessment, your scores and logs will show up here.
          </p>
          <Button onClick={() => router.push("/explore")}>Go to Explore</Button>
        </div>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full min-w-[760px] text-sm">
            <thead className="border-b border-border bg-muted/40 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-4 py-3 font-medium">Assessment</th>
                <th className="px-4 py-3 font-medium">Mode</th>
                <th className="px-4 py-3 font-medium">Attempt</th>
                <th className="px-4 py-3 text-right font-medium">Score</th>
                <th className="px-4 py-3 text-right font-medium">Completed</th>
                <th className="px-4 py-3 text-center font-medium">Action</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {sessions.map((s) => {
                const percentage =
                  s.pointsAwarded != null && s.maxPoints
                    ? (s.pointsAwarded / s.maxPoints) * 100
                    : null;
                const tone =
                  percentage == null
                    ? "neutral"
                    : percentage >= 80
                      ? "success"
                      : percentage >= 60
                        ? "warning"
                        : "error";
                const modeLabel = s.kind === "exam" ? "Exam" : "Practice";
                return (
                  <tr key={s.id} className="hover:bg-muted/30">
                    <td className="px-4 py-4 font-medium">
                      {s.assessmentTitle || "Untitled Assessment"}
                    </td>
                    <td className="px-4 py-4">
                      <span className="rounded-full border px-2 py-0.5 text-xs">
                        {modeLabel}
                      </span>
                    </td>
                    <td className="px-4 py-4 text-muted-foreground">
                      {s.attemptNumber} of {s.totalAttempts}
                    </td>
                    <td className="px-4 py-4 text-right">
                      {s.pointsAwarded != null && s.maxPoints != null ? (
                        <span className="inline-flex items-center gap-2">
                          {s.pointsAwarded} / {s.maxPoints} pts{" "}
                          <span
                            className={`rounded-full border px-2 py-0.5 text-xs ${toneClasses[tone]}`}
                          >
                            {Math.round(percentage || 0)}%
                          </span>
                        </span>
                      ) : (
                        <span className="text-muted-foreground">
                          {s.pointsAwarded != null
                            ? `${s.pointsAwarded} pts`
                            : "—"}
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-4 text-right text-muted-foreground">
                      <span className="inline-flex items-center gap-1.5">
                        <CalendarDays size={15} />
                        {s.finishedAt ? formatDateTime(s.finishedAt) : "—"}
                      </span>
                    </td>
                    <td className="px-4 py-4 text-center">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => router.push(`/sessions/${s.id}/results`)}
                      >
                        Review <ExternalLink size={14} />
                      </Button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          <div className="flex items-center justify-end gap-4 border-t border-border px-4 py-3 text-sm text-muted-foreground">
            <label>
              Rows{" "}
              <select
                className="ml-2 rounded border border-input bg-background px-2 py-1"
                value={rowsPerPage}
                onChange={(e) => {
                  setRowsPerPage(Number(e.target.value));
                  setPage(0);
                }}
              >
                <option value={5}>5</option>
                <option value={10}>10</option>
                <option value={25}>25</option>
                <option value={50}>50</option>
              </select>
            </label>
            <span>
              {total === 0 ? 0 : page * rowsPerPage + 1}–
              {Math.min((page + 1) * rowsPerPage, total)} of {total}
            </span>
            <Button
              size="sm"
              variant="ghost"
              disabled={page === 0}
              onClick={() => setPage(page - 1)}
            >
              Previous
            </Button>
            <Button
              size="sm"
              variant="ghost"
              disabled={(page + 1) * rowsPerPage >= total}
              onClick={() => setPage(page + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </PageShell>
  );
}
