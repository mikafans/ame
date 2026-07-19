"use client";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { formatDateTime, formatScore } from "@/utils/format";
import { PageShell } from "@/components/PageShell";
import { Button } from "@/components/ui/button";
interface Stats {
  avg_score: number;
  avg_score_delta: number;
  attempts_total: number;
  attempts_this_week: number;
  hours_spent: number;
  current_streak: number;
  best_streak: number;
  mastered_topics: number;
  mastered_topics_total: number;
}
interface Attempt {
  id: string;
  is_correct: boolean;
  score: number;
  created_at: string;
  question_id: string;
  grader_notes?: string | null;
}
interface Session {
  id: string;
  kind: string;
  status: string;
  assessmentTitle?: string | null;
  pointsAwarded?: number | null;
  maxPoints?: number | null;
  startedAt: string;
  finishedAt?: string | null;
}
interface Agent {
  id: string;
  label: string;
  createdAt: string;
  focusTags: string[];
  currentGoal?: string | null;
  nextTarget?: string | null;
}
export default function ProgressPage() {
  const router = useRouter();
  const [window, setWindow] = useState<"4w" | "all">("4w");
  const [stats, setStats] = useState<Stats | null>(null);
  const [attempts, setAttempts] = useState<Attempt[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    setLoading(true);
    Promise.all([
      (api as any).GET("/v1/me/stats", {
        params: { query: { window: window === "4w" ? "last30d" : "all" } },
      }),
      (api as any).GET("/v1/me/attempts", { params: { query: { limit: 50 } } }),
      (api as any).GET("/v1/sessions", { params: { query: { limit: 10 } } }),
      (api as any).GET("/v1/me/agents"),
    ])
      .then(([statsResult, attemptsResult, sessionsResult, agentsResult]) => {
        setStats(statsResult.data ?? null);
        setAttempts(attemptsResult.data?.attempts ?? []);
        setSessions(sessionsResult.data?.sessions ?? []);
        setAgents(agentsResult.data?.agents ?? []);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [window]);
  const cards = stats
    ? [
        ["Average score", `${Math.round(stats.avg_score * 100)}%`],
        ["Attempts", String(stats.attempts_total)],
        ["Hours spent", stats.hours_spent.toFixed(1)],
        ["Current streak", `${stats.current_streak} days`],
        [
          "Mastered topics",
          `${stats.mastered_topics}/${stats.mastered_topics_total}`,
        ],
      ]
    : [];
  return (
    <PageShell
      kicker="Progress"
      title="Your learning progress"
      subtitle="See how your practice is compounding over time."
    >
      <div className="mb-6 flex gap-2">
        {(["4w", "all"] as const).map((value) => (
          <button
            key={value}
            className={`rounded-md border px-3 py-2 text-xs ${window === value ? "border-primary bg-primary text-primary-foreground" : "border-border text-muted-foreground"}`}
            onClick={() => setWindow(value)}
          >
            {value === "4w" ? "Last 4 weeks" : "All time"}
          </button>
        ))}
      </div>
      {loading ? (
        <div className="p-12 text-muted-foreground">Loading progress…</div>
      ) : (
        <>
          <div className="mb-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
            {cards.map(([label, value]) => (
              <div key={label} className="rounded-lg border border-border p-5">
                <p className="text-xs text-muted-foreground">{label}</p>
                <p className="mt-2 text-2xl font-semibold">{value}</p>
              </div>
            ))}
          </div>
          <div className="grid gap-6 lg:grid-cols-2">
            <section className="rounded-lg border border-border p-5">
              <div className="mb-4 flex items-center justify-between">
                <h2 className="font-semibold">Recent attempts</h2>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => router.push("/results")}
                >
                  View history
                </Button>
              </div>
              <div className="divide-y divide-border">
                {attempts.slice(0, 8).map((attempt) => (
                  <div
                    key={attempt.id}
                    className="flex items-center justify-between py-3 text-sm"
                  >
                    <div>
                      <span
                        className={
                          attempt.is_correct
                            ? "text-emerald-600"
                            : "text-red-600"
                        }
                      >
                        {attempt.is_correct ? "Correct" : "Incorrect"}
                      </span>
                      <p className="text-xs text-muted-foreground">
                        {formatDateTime(attempt.created_at)}
                      </p>
                    </div>
                    <span className="font-mono">
                      {formatScore(attempt.score)}
                    </span>
                  </div>
                ))}
                {attempts.length === 0 && (
                  <p className="py-6 text-sm text-muted-foreground">
                    No attempts yet.
                  </p>
                )}
              </div>
            </section>
            <section className="rounded-lg border border-border p-5">
              <div className="mb-4 flex items-center justify-between">
                <h2 className="font-semibold">Recent sessions</h2>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => router.push("/results")}
                >
                  View all
                </Button>
              </div>
              <div className="divide-y divide-border">
                {sessions.slice(0, 6).map((session) => (
                  <button
                    key={session.id}
                    className="flex w-full items-center justify-between py-3 text-left text-sm hover:text-primary"
                    onClick={() =>
                      router.push(`/sessions/${session.id}/results`)
                    }
                  >
                    <div>
                      <p>{session.assessmentTitle || session.kind}</p>
                      <p className="text-xs text-muted-foreground">
                        {session.finishedAt
                          ? formatDateTime(session.finishedAt)
                          : session.status}
                      </p>
                    </div>
                    <span className="font-mono text-xs">
                      {session.pointsAwarded != null && session.maxPoints
                        ? `${session.pointsAwarded}/${session.maxPoints}`
                        : "—"}
                    </span>
                  </button>
                ))}
              </div>
            </section>
          </div>
          <section className="mt-6 rounded-lg border border-border p-5">
            <h2 className="mb-4 font-semibold">Agent progress</h2>
            <div className="grid gap-3 sm:grid-cols-2">
              {agents.map((agent) => (
                <div
                  key={agent.id}
                  className="rounded border border-border p-4"
                >
                  <p className="font-medium">{agent.label}</p>
                  <p className="mt-1 text-sm text-muted-foreground">
                    {agent.currentGoal ||
                      agent.nextTarget ||
                      "No active target"}
                  </p>
                  <div className="mt-3 flex flex-wrap gap-1">
                    {agent.focusTags.map((tag) => (
                      <span
                        key={tag}
                        className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
              {agents.length === 0 && (
                <p className="text-sm text-muted-foreground">
                  No agents connected.
                </p>
              )}
            </div>
          </section>
        </>
      )}
    </PageShell>
  );
}
