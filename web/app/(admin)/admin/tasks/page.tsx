"use client";

import { useEffect, useState } from "react";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";

type Submission = components["schemas"]["TaskSubmissionResponse"];

export default function AdminTasksPage() {
  const [submissions, setSubmissions] = useState<Submission[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [scores, setScores] = useState<Record<string, string>>({});
  const [feedback, setFeedback] = useState<Record<string, string>>({});

  async function load() {
    const result = await api.GET("/api/v1/admin/task-submissions");
    if (!result.response.ok || !result.data) {
      setError("Could not load pending task submissions.");
    } else {
      setSubmissions(result.data);
    }
    setLoading(false);
  }

  useEffect(() => {
    void load();
  }, []);

  async function review(submission: Submission) {
    const score = Number(scores[submission.id]);
    if (!Number.isFinite(score) || score < 0 || score > 1) {
      setError("Enter a score between 0 and 1.");
      return;
    }
    const result = await api.PATCH(
      "/api/v1/task-submissions/{submission_id}/review",
      {
        params: { path: { submission_id: submission.id } },
        body: {
          outcome: "reviewed",
          score,
          feedback: { note: feedback[submission.id] ?? "" },
        },
      },
    );
    if (!result.response.ok) {
      setError("Could not review this submission.");
      return;
    }
    setSubmissions((current) =>
      current.filter((candidate) => candidate.id !== submission.id),
    );
  }

  return (
    <main className="mx-auto w-full max-w-6xl space-y-8 px-6 py-12 sm:px-12">
      <header>
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Learning operations
        </p>
        <h1 className="mt-2 text-3xl font-bold">Task review queue</h1>
        <p className="mt-2 text-muted-foreground">
          Review learner application tasks before they become mastery evidence.
        </p>
      </header>
      {error && <p className="text-sm text-destructive">{error}</p>}
      {loading ? (
        <p className="text-muted-foreground">Loading submissions…</p>
      ) : submissions.length === 0 ? (
        <section className="rounded-2xl border border-dashed border-border p-10 text-center text-muted-foreground">
          No task submissions are waiting for review.
        </section>
      ) : (
        <section className="space-y-4">
          {submissions.map((submission) => (
            <article
              key={submission.id}
              className="rounded-2xl border border-border bg-card p-6"
            >
              <p className="font-mono text-xs text-muted-foreground">
                {submission.id}
              </p>
              <p className="mt-4 text-sm font-medium">Learner response</p>
              <pre className="mt-2 overflow-x-auto rounded-xl bg-muted p-4 text-sm">
                {JSON.stringify(submission.response, null, 2)}
              </pre>
              <div className="mt-4 grid gap-3 sm:grid-cols-[140px_1fr_auto]">
                <input
                  aria-label={`Score ${submission.id}`}
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  placeholder="Score 0–1"
                  value={scores[submission.id] ?? ""}
                  onChange={(event) =>
                    setScores((current) => ({
                      ...current,
                      [submission.id]: event.target.value,
                    }))
                  }
                  className="h-11 rounded-xl border border-input bg-background px-3"
                />
                <input
                  aria-label={`Feedback ${submission.id}`}
                  placeholder="Feedback for the learner"
                  value={feedback[submission.id] ?? ""}
                  onChange={(event) =>
                    setFeedback((current) => ({
                      ...current,
                      [submission.id]: event.target.value,
                    }))
                  }
                  className="h-11 rounded-xl border border-input bg-background px-3"
                />
                <button
                  type="button"
                  onClick={() => void review(submission)}
                  className="rounded-xl bg-primary px-5 text-sm font-semibold text-primary-foreground"
                >
                  Review task
                </button>
              </div>
            </article>
          ))}
        </section>
      )}
    </main>
  );
}
