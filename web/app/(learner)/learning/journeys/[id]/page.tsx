"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { ArrowRight, Check, Clock3 } from "lucide-react";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";
import { Button } from "@/components/ui/button";

type Journey = components["schemas"]["LearningJourneyResponse"];
type LearningSession = components["schemas"]["LearningSessionResponse"];

export default function LearningJourneyPage() {
  const params = useParams<{ id: string }>();
  const [journey, setJourney] = useState<Journey | null>(null);
  const [session, setSession] = useState<LearningSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [startingActivity, setStartingActivity] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .GET("/v1/learning/journeys/{id}", {
        params: { path: { id: params.id } },
      })
      .then(({ data, response }) => {
        if (!response.ok || !data) {
          throw new Error("Could not load this learning journey");
        }
        if (!cancelled) setJourney(data);
      })
      .catch((loadError) => {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Could not load this journey",
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [params.id]);

  async function startActivity(activityId: string) {
    setStartingActivity(activityId);
    setError(null);
    try {
      const { data, response } = await api.POST(
        "/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
        {
          params: { path: { journey_id: params.id, activity_id: activityId } },
        },
      );
      if (!response.ok || !data)
        throw new Error("Could not start this activity");
      setSession(data);
    } catch (startError) {
      setError(
        startError instanceof Error
          ? startError.message
          : "Could not start this activity",
      );
    } finally {
      setStartingActivity(null);
    }
  }

  if (error && !journey) {
    return <div className="p-8 text-destructive">{error}</div>;
  }
  if (!journey) {
    return (
      <div className="p-8 text-muted-foreground">Loading your journey…</div>
    );
  }

  const readyActivity = journey.activities.find(
    (activity) => activity.status === "ready",
  );

  return (
    <div className="mx-auto w-full max-w-4xl space-y-8 p-6 sm:p-10">
      <header className="space-y-3">
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          {journey.status} journey
        </p>
        <h1 className="text-3xl font-bold tracking-tight">{journey.promise}</h1>
        <p className="text-muted-foreground">
          {journey.goal.normalizedStatement}
        </p>
      </header>

      <section className="rounded-2xl border border-border bg-card p-6">
        <h2 className="text-lg font-semibold">What you will be able to do</h2>
        <ul className="mt-4 space-y-3">
          {journey.objectives.map((objective) => (
            <li key={objective.id} className="flex gap-3 text-sm leading-6">
              <Check className="mt-1 size-4 shrink-0 text-primary" />
              <span>{objective.statement}</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="space-y-3">
        <div>
          <h2 className="text-lg font-semibold">Your path</h2>
          <p className="text-sm text-muted-foreground">
            One focused step at a time.
          </p>
        </div>
        <div className="space-y-3">
          {journey.activities.map((activity) => (
            <article
              key={activity.id}
              className="flex items-center justify-between gap-4 rounded-xl border border-border p-4"
            >
              <div className="min-w-0">
                <p className="text-xs uppercase tracking-[0.12em] text-muted-foreground">
                  {activity.kind} · {activity.status}
                </p>
                <h3 className="mt-1 font-medium">{activity.title}</h3>
              </div>
              {activity.status === "ready" && (
                <Button
                  type="button"
                  disabled={startingActivity !== null}
                  onClick={() => startActivity(activity.id)}
                  className="shrink-0 rounded-full"
                >
                  {startingActivity === activity.id ? "Starting…" : "Begin"}
                  <ArrowRight className="size-4" />
                </Button>
              )}
            </article>
          ))}
        </div>
      </section>

      {readyActivity && !session && (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <Clock3 className="size-4" /> Your first activity is ready when you
          are.
        </p>
      )}
      {session && (
        <section className="rounded-2xl border border-primary/30 bg-primary/10 p-6">
          <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
            Session started
          </p>
          <h2 className="mt-2 text-xl font-semibold">
            You are ready for the first activity.
          </h2>
          <p className="mt-2 text-sm text-muted-foreground">
            Session {session.id}
          </p>
        </section>
      )}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
