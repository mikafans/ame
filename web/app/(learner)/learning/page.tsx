"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import {
  ArrowRight,
  BarChart3,
  CheckCircle2,
  Compass,
  Sparkles,
} from "lucide-react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";

type JourneySummary = {
  id: string;
  promise: string;
  status: string;
  rawIntent: string;
  nextActivityId: string | null;
  nextActivityTitle: string | null;
  createdAt: string;
};

type Snapshot = { mastery: number; confidence: number; evidenceCount: number };

export default function LearningHomePage() {
  const [journeys, setJourneys] = useState<JourneySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [snapshots, setSnapshots] = useState<Record<string, Snapshot>>({});

  useEffect(() => {
    void (async () => {
      const { data, response } = await api.GET(
        "/api/v1/learning/journeys" as never,
      );
      if (response.ok && data) {
        const nextJourneys = data as JourneySummary[];
        setJourneys(nextJourneys);
      }
      setLoading(false);
    })();
  }, []);

  useEffect(() => {
    if (journeys.length === 0) return;
    void (async () => {
      const results = await Promise.all(
        journeys.slice(0, 3).map(async (journey) => {
          const { data, response } = await api.GET(
            "/api/v1/learning/journeys/{id}",
            { params: { path: { id: journey.id } } },
          );
          const objective = (
            data as { objectives?: { id: string }[] } | undefined
          )?.objectives?.[0];
          if (!response.ok || !objective) return null;
          const snapshot = await api.GET(
            "/api/v1/progress/{journey_id}/objectives/{objective_id}",
            {
              params: {
                path: { journey_id: journey.id, objective_id: objective.id },
              },
            },
          );
          if (!snapshot.response.ok || !snapshot.data) return null;
          return [journey.id, snapshot.data as Snapshot] as const;
        }),
      );
      setSnapshots(
        Object.fromEntries(
          results.filter(
            (value): value is readonly [string, Snapshot] => value !== null,
          ),
        ),
      );
    })();
  }, [journeys]);

  return (
    <main className="mx-auto w-full max-w-5xl space-y-8 px-4 py-8 sm:px-8 sm:py-12">
      <section className="rounded-3xl border border-border bg-card p-7 shadow-sm sm:p-10">
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Learning desk
        </p>
        <h1 className="mt-3 max-w-2xl text-4xl font-extrabold tracking-tight sm:text-5xl">
          Your learning, with a next move.
        </h1>
        <p className="mt-4 max-w-2xl leading-7 text-muted-foreground">
          An agent can shape the plan, but you always see what it means: the
          intent, the next activity, and the evidence behind your progress.
        </p>
        <Button asChild className="mt-7 rounded-full">
          <Link href="/start">
            Start another journey <ArrowRight className="size-4" />
          </Link>
        </Button>
      </section>

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading journeys…</p>
      ) : journeys.length === 0 ? (
        <section className="rounded-2xl border border-dashed border-border p-10 text-center">
          <Compass className="mx-auto size-8 text-primary" />
          <h2 className="mt-4 text-xl font-bold">
            Your first journey is one prompt away.
          </h2>
          <p className="mt-2 text-muted-foreground">
            Tell AME what you want to learn and it will shape a grounded first
            step.
          </p>
        </section>
      ) : (
        <section className="grid gap-4">
          {journeys.map((journey) => (
            <Link
              key={journey.id}
              href={`/learning/journeys/${journey.id}`}
              className="group rounded-2xl border border-border bg-card p-6 transition hover:border-primary hover:shadow-sm"
            >
              <div className="flex flex-col justify-between gap-5 sm:flex-row sm:items-center">
                <div>
                  <p className="font-mono text-xs uppercase tracking-[0.12em] text-primary">
                    {journey.status}
                  </p>
                  <h2 className="mt-2 text-xl font-bold">
                    {journey.rawIntent}
                  </h2>
                  <p className="mt-2 text-sm text-muted-foreground">
                    {journey.nextActivityTitle
                      ? `Next: ${journey.nextActivityTitle}`
                      : journey.promise}
                  </p>
                  {snapshots[journey.id] && (
                    <div className="mt-4 flex items-center gap-3 text-xs text-muted-foreground">
                      <BarChart3 className="size-4 text-primary" />
                      <span>
                        {Math.round(snapshots[journey.id].mastery * 100)}%
                        signal · {snapshots[journey.id].evidenceCount} evidence
                      </span>
                    </div>
                  )}
                </div>
                <ArrowRight className="size-5 shrink-0 text-primary transition group-hover:translate-x-1" />
              </div>
            </Link>
          ))}
        </section>
      )}

      {!loading && journeys.length > 0 && (
        <section className="grid gap-4 sm:grid-cols-3">
          <div className="rounded-2xl border border-border bg-card p-5">
            <Sparkles className="size-5 text-primary" />
            <h2 className="mt-4 font-semibold">Agent-shaped plan</h2>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              Your prompt became a journey with visible objectives.
            </p>
          </div>
          <div className="rounded-2xl border border-border bg-card p-5">
            <CheckCircle2 className="size-5 text-primary" />
            <h2 className="mt-4 font-semibold">Evidence over guesses</h2>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              Practice and assessments create progress signals you can inspect.
            </p>
          </div>
          <div className="rounded-2xl border border-border bg-card p-5">
            <Compass className="size-5 text-primary" />
            <h2 className="mt-4 font-semibold">Go deeper when needed</h2>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              Weak signals can lead to a source-backed explanation and another
              try.
            </p>
          </div>
        </section>
      )}
    </main>
  );
}
