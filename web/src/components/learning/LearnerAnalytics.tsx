"use client";

import { useEffect, useState } from "react";
import { api } from "@/api/client";

type Ratio = { value?: number | null; numerator: number; denominator: number };
type Analytics = {
  timezone: string;
  privacyBoundary: string;
  streak: { currentDays: number; bestDays: number; qualifyingDays: number };
  completionRate: Ratio;
  averageScore: Ratio;
  attempts: number;
  timeSpent: { seconds: number; finishedSessions: number };
  masteryTrend: { objectiveId: string; value: number; occurredAt: string }[];
  reviewHistory: {
    reviewItemId: string;
    rating: string;
    reviewedAt: string;
    intervalDays: number;
  }[];
  definitions: string[];
};

export function LearnerAnalytics({ journeyId }: { journeyId: string }) {
  const [analytics, setAnalytics] = useState<Analytics | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
    void api
      .GET("/api/v1/learning/journeys/{id}/analytics", {
        params: { path: { id: journeyId }, query: { timezone } },
      })
      .then((result) => {
        if (result.response.ok && result.data) {
          setAnalytics(result.data as Analytics);
        } else {
          setError(true);
        }
      });
  }, [journeyId]);

  if (error) {
    return (
      <p className="text-sm text-destructive">Analytics could not be loaded.</p>
    );
  }
  if (!analytics) {
    return <p className="text-sm text-muted-foreground">Loading analytics…</p>;
  }
  const percent = (metric: Ratio) =>
    metric.value == null
      ? "No scored data"
      : `${Math.round(metric.value * 100)}%`;
  return (
    <section
      className="rounded-3xl border border-border bg-card p-7"
      data-testid="learner-analytics"
    >
      <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
        Retention and progress
      </p>
      <h2 className="mt-2 text-2xl font-bold">Your event-backed trends</h2>
      <p className="mt-1 text-xs text-muted-foreground">
        {analytics.timezone} · {analytics.privacyBoundary.replaceAll("_", " ")}
      </p>
      <div className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Metric
          detail={`${analytics.streak.qualifyingDays} distinct qualifying days`}
          label="Current / best streak"
          value={`${analytics.streak.currentDays} / ${analytics.streak.bestDays} days`}
        />
        <Metric
          detail={`${analytics.completionRate.numerator} completed / ${analytics.completionRate.denominator} curriculum activities`}
          label="Completion"
          value={percent(analytics.completionRate)}
        />
        <Metric
          detail={`${analytics.averageScore.denominator} scored outcomes · ${analytics.attempts} assessment attempts`}
          label="Average score"
          value={percent(analytics.averageScore)}
        />
        <Metric
          detail={`${analytics.timeSpent.finishedSessions} finished sessions`}
          label="Time spent"
          value={`${Math.round(analytics.timeSpent.seconds / 60)} min`}
        />
      </div>
      {analytics.masteryTrend.length === 0 &&
      analytics.reviewHistory.length === 0 ? (
        <p className="mt-5 text-sm text-muted-foreground">
          Complete reviewed work and retrieval sessions to start these trends.
        </p>
      ) : (
        <div className="mt-5 grid gap-4 sm:grid-cols-2">
          <div>
            <h3 className="font-semibold">Mastery evidence</h3>
            <ul className="mt-2 space-y-1 text-sm text-muted-foreground">
              {analytics.masteryTrend.slice(-5).map((point) => (
                <li key={`${point.objectiveId}-${point.occurredAt}`}>
                  {Math.round(point.value * 100)}% ·{" "}
                  {new Date(point.occurredAt).toLocaleDateString()}
                </li>
              ))}
            </ul>
          </div>
          <div>
            <h3 className="font-semibold">Review history</h3>
            <ul className="mt-2 space-y-1 text-sm text-muted-foreground">
              {analytics.reviewHistory.slice(-5).map((review) => (
                <li key={`${review.reviewItemId}-${review.reviewedAt}`}>
                  {review.rating} · next interval {review.intervalDays} days
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
      <details className="mt-5 text-xs text-muted-foreground">
        <summary className="cursor-pointer font-medium">
          Metric definitions
        </summary>
        <ul className="mt-2 list-disc space-y-1 pl-5">
          {analytics.definitions.map((definition) => (
            <li key={definition}>{definition}</li>
          ))}
        </ul>
      </details>
    </section>
  );
}

function Metric({
  label,
  value,
  detail,
}: {
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <div className="rounded-2xl bg-muted p-4">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-xl font-bold">{value}</p>
      <p className="mt-1 text-xs text-muted-foreground">{detail}</p>
    </div>
  );
}
