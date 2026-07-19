"use client";

import { useState, useEffect } from "react";
import { formatDate } from "@/utils/format";
import { Button } from "@/components/ui/button";

interface PendingAttempt {
  attempt_id: string;
  session_id: string;
  user_id: string;
  user_email: string;
  user_display_name: string;
  question_id: string;
  question_prompt: string;
  response_body: string;
  response_word_count: number;
  created_at: string;
}
interface GradeState {
  score: string;
  notes: string;
  submitting: boolean;
  done: boolean;
}

export default function GradingPage() {
  const [attempts, setAttempts] = useState<PendingAttempt[]>([]);
  const [loading, setLoading] = useState(true);
  const [grades, setGrades] = useState<Record<string, GradeState>>({});
  useEffect(() => {
    const apiUrl =
      process.env.NEXT_PUBLIC_API_URL ??
      (typeof window !== "undefined"
        ? `http://${window.location.hostname}:28080`
        : "http://localhost:28080");
    fetch(`${apiUrl}/v1/attempts/pending`, { credentials: "include" })
      .then((r) => {
        if (!r.ok) throw new Error("Failed to fetch pending attempts");
        return r.json();
      })
      .then((data: PendingAttempt[]) => {
        const list = Array.isArray(data) ? data : [];
        setAttempts(list);
        setGrades(
          Object.fromEntries(
            list.map((a) => [
              a.attempt_id,
              { score: "", notes: "", submitting: false, done: false },
            ]),
          ),
        );
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);
  const handleGrade = async (attemptId: string) => {
    const g = grades[attemptId];
    if (!g) return;
    const scoreNum = parseFloat(g.score);
    if (isNaN(scoreNum) || scoreNum < 0 || scoreNum > 100) return;
    setGrades((prev) => ({
      ...prev,
      [attemptId]: { ...prev[attemptId], submitting: true },
    }));
    const apiUrl =
      process.env.NEXT_PUBLIC_API_URL ??
      (typeof window !== "undefined"
        ? `http://${window.location.hostname}:28080`
        : "http://localhost:28080");
    try {
      const res = await fetch(`${apiUrl}/v1/attempts/${attemptId}/grade`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ score: scoreNum / 100, notes: g.notes || null }),
      });
      if (res.ok) {
        setAttempts((prev) => prev.filter((a) => a.attempt_id !== attemptId));
      }
      setGrades((prev) => ({
        ...prev,
        [attemptId]: { ...prev[attemptId], submitting: false, done: res.ok },
      }));
    } catch {
      setGrades((prev) => ({
        ...prev,
        [attemptId]: { ...prev[attemptId], submitting: false },
      }));
    }
  };
  if (loading)
    return (
      <div className="flex min-h-screen items-center justify-center">
        <span className="size-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      </div>
    );
  return (
    <div className="px-4 pb-16 pt-10 sm:px-12">
      <div className="mb-7">
        <p className="mb-2 text-xs uppercase tracking-[0.13em] text-muted-foreground">
          Manual grading
        </p>
        <h1 className="text-3xl font-medium">Essay Grading</h1>
      </div>
      {attempts.length === 0 ? (
        <div className="rounded-lg border border-border p-16 text-center text-sm text-muted-foreground">
          No essays pending manual review.
        </div>
      ) : (
        <div className="space-y-5">
          <p className="text-xs text-muted-foreground">
            {attempts.length} essay{attempts.length !== 1 ? "s" : ""} awaiting
            review
          </p>
          {attempts.map((attempt) => {
            const g = grades[attempt.attempt_id] ?? {
              score: "",
              notes: "",
              submitting: false,
              done: false,
            };
            const scoreNum = parseFloat(g.score);
            const scoreValid =
              !isNaN(scoreNum) && scoreNum >= 0 && scoreNum <= 100;
            return (
              <article
                key={attempt.attempt_id}
                className="overflow-hidden rounded-lg border border-border"
              >
                <div className="flex items-start justify-between border-b border-border px-5 py-4">
                  <div>
                    <p className="mb-1 text-xs uppercase tracking-[0.12em] text-muted-foreground">
                      Essay · {attempt.response_word_count} words
                    </p>
                    <h2 className="text-base font-medium leading-snug">
                      {attempt.question_prompt}
                    </h2>
                  </div>
                  <div className="ml-6 shrink-0 text-right text-xs text-muted-foreground">
                    <p>{attempt.user_display_name}</p>
                    <p className="text-muted-foreground/70">
                      {attempt.user_email}
                    </p>
                    <p className="text-muted-foreground/70">
                      {formatDate(attempt.created_at)}
                    </p>
                  </div>
                </div>
                <div className="border-b border-border bg-muted/40 px-5 py-4">
                  <p className="mb-2 text-xs uppercase tracking-[0.12em] text-muted-foreground">
                    Response
                  </p>
                  <p className="whitespace-pre-wrap text-sm leading-7">
                    {attempt.response_body || "(empty)"}
                  </p>
                </div>
                <div className="grid gap-4 px-5 py-4 sm:grid-cols-[120px_1fr_auto] sm:items-end">
                  <label className="grid gap-1.5 text-xs text-muted-foreground">
                    Score (0–100)
                    <input
                      className="h-9 rounded-md border border-input bg-background px-3 text-sm text-foreground outline-none focus:ring-2 focus:ring-ring"
                      type="number"
                      min="0"
                      max="100"
                      step="1"
                      value={g.score}
                      onChange={(e) =>
                        setGrades((prev) => ({
                          ...prev,
                          [attempt.attempt_id]: {
                            ...prev[attempt.attempt_id],
                            score: e.target.value,
                          },
                        }))
                      }
                      placeholder="e.g. 75"
                    />
                  </label>
                  <label className="grid gap-1.5 text-xs text-muted-foreground">
                    Notes (optional)
                    <textarea
                      className="min-h-[72px] rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-2 focus:ring-ring"
                      value={g.notes}
                      onChange={(e) =>
                        setGrades((prev) => ({
                          ...prev,
                          [attempt.attempt_id]: {
                            ...prev[attempt.attempt_id],
                            notes: e.target.value,
                          },
                        }))
                      }
                      placeholder="Good structure but missing examples…"
                    />
                  </label>
                  <Button
                    disabled={!scoreValid || g.submitting}
                    onClick={() => handleGrade(attempt.attempt_id)}
                  >
                    {g.submitting ? "Saving…" : "Grade"}
                  </Button>
                </div>
              </article>
            );
          })}
        </div>
      )}
    </div>
  );
}
