"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { ArrowRight, Check, Clock3 } from "lucide-react";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";
import { Button } from "@/components/ui/button";

type Journey = components["schemas"]["LearningJourneyResponse"];
type LearningSession = components["schemas"]["LearningSessionResponse"];
type StarterQuestion = {
  id: string;
  kind: "single_choice" | "short_text";
  prompt: string;
  options?: string[];
};
type StarterContent = {
  type: "starter_check";
  context?: string;
  instructions: string;
  questions: StarterQuestion[];
};

export default function LearningJourneyPage() {
  const params = useParams<{ id: string }>();
  const [journey, setJourney] = useState<Journey | null>(null);
  const [session, setSession] = useState<LearningSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [startingActivity, setStartingActivity] = useState<string | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [responses, setResponses] = useState<Record<string, string>>({});

  useEffect(() => {
    let cancelled = false;
    api
      .GET("/api/v1/learning/journeys/{id}", {
        params: { path: { id: params.id } },
      })
      .then(({ data, response }) => {
        if (!response.ok || !data) {
          throw new Error("Could not load this learning journey");
        }
        if (!cancelled) setJourney(data);
        const storedSessionId = window.localStorage.getItem(
          `ame-learning-session:${params.id}`,
        );
        if (storedSessionId) {
          return api.GET("/api/v1/learning/sessions/{id}", {
            params: { path: { id: storedSessionId } },
          });
        }
        return null;
      })
      .then((result) => {
        if (
          result &&
          result.response.ok &&
          result.data &&
          result.data.journeyId === params.id &&
          result.data.status === "in_progress" &&
          !cancelled
        ) {
          setSession(result.data);
        }
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
        "/api/v1/learning/journeys/{journey_id}/activities/{activity_id}/start",
        {
          params: { path: { journey_id: params.id, activity_id: activityId } },
        },
      );
      if (!response.ok || !data)
        throw new Error("Could not start this activity");
      setSession(data);
      setResponses({});
      window.localStorage.setItem(`ame-learning-session:${params.id}`, data.id);
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

  async function finishSession() {
    if (!session) return;
    setFinishing(true);
    setError(null);
    try {
      const { data, response } = await api.POST(
        "/api/v1/learning/sessions/{id}/finish",
        {
          params: { path: { id: session.id } },
          body: {
            completed: true,
            responses: Object.entries(responses).map(([id, value]) => ({
              id,
              value,
            })),
          },
        },
      );
      if (!response.ok || !data)
        throw new Error("Could not finish this activity");
      setSession(data);
      window.localStorage.removeItem(`ame-learning-session:${params.id}`);
      const refreshed = await api.GET("/api/v1/learning/journeys/{id}", {
        params: { path: { id: params.id } },
      });
      if (refreshed.response.ok && refreshed.data) setJourney(refreshed.data);
    } catch (finishError) {
      setError(
        finishError instanceof Error
          ? finishError.message
          : "Could not finish this activity",
      );
    } finally {
      setFinishing(false);
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
  const activeActivity = session
    ? journey.activities.find((activity) => activity.id === session.activityId)
    : null;
  const activeContent = activeActivity?.payload
    ? ((activeActivity.payload as unknown as { content?: StarterContent })
        .content ?? null)
    : null;

  return (
    <div className="mx-auto grid w-full max-w-7xl gap-8 p-6 sm:p-10 lg:grid-cols-[260px_1fr]">
      <aside className="space-y-6 border-b border-border pb-6 lg:border-b-0 lg:border-r lg:pr-8">
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          {journey.status} journey
        </p>
        <h1 className="text-2xl font-bold tracking-tight">{journey.promise}</h1>
        <p className="text-sm leading-6 text-muted-foreground">
          {journey.goal.normalizedStatement}
        </p>
        <div className="border-t border-border pt-5">
          <h2 className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
            Outcomes
          </h2>
          <ul className="mt-4 space-y-3">
            {journey.objectives.map((objective) => (
              <li key={objective.id} className="flex gap-3 text-sm leading-6">
                <Check className="mt-1 size-4 shrink-0 text-primary" />
                <span>{objective.statement}</span>
              </li>
            ))}
          </ul>
        </div>
      </aside>

      <main className="space-y-8">
        {journey.recommendation && (
          <section className="border-l-4 border-primary bg-primary/5 p-5">
            <p className="font-mono text-xs uppercase tracking-[0.12em] text-primary">
              Recommended next
            </p>
            <h2 className="mt-2 text-xl font-semibold">
              {journey.recommendation.title}
            </h2>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              {journey.recommendation.rationale}
            </p>
          </section>
        )}
        <section className="space-y-4">
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
                className="flex items-center justify-between gap-4 border-b border-border py-5 first:border-t"
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
          <section className="border border-primary/30 bg-primary/10 p-6 shadow-[8px_8px_0_hsl(var(--primary)/0.12)]">
            <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
              Session started
            </p>
            <h2 className="mt-2 text-xl font-semibold">
              {activeActivity?.title ?? "Your first activity"}
            </h2>
            <p className="mt-2 text-sm text-muted-foreground">
              {activeActivity?.payload &&
              typeof activeActivity.payload === "object" &&
              "purpose" in activeActivity.payload
                ? String(activeActivity.payload.purpose)
                : "Take this focused first step, then AME will open the next one."}
            </p>
            {session.status === "in_progress" &&
              activeContent?.type === "starter_check" && (
                <div className="mt-5 space-y-5 border-t border-primary/20 pt-5">
                  {activeContent.context && (
                    <p className="rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6 text-foreground">
                      {activeContent.context}
                    </p>
                  )}
                  <p className="text-sm font-medium">
                    {activeContent.instructions}
                  </p>
                  {activeContent.questions.map((question) => (
                    <div key={question.id} className="space-y-2">
                      <label
                        htmlFor={`learning-${question.id}`}
                        className="block text-sm font-medium"
                      >
                        {question.prompt}
                      </label>
                      {question.kind === "single_choice" ? (
                        <div className="flex flex-wrap gap-2">
                          {question.options?.map((option) => (
                            <Button
                              key={option}
                              type="button"
                              variant={
                                responses[question.id] === option
                                  ? "default"
                                  : "outline"
                              }
                              onClick={() =>
                                setResponses((current) => ({
                                  ...current,
                                  [question.id]: option,
                                }))
                              }
                              className="rounded-full"
                            >
                              {option.replaceAll("_", " ")}
                            </Button>
                          ))}
                        </div>
                      ) : (
                        <input
                          id={`learning-${question.id}`}
                          value={responses[question.id] ?? ""}
                          onChange={(event) =>
                            setResponses((current) => ({
                              ...current,
                              [question.id]: event.target.value,
                            }))
                          }
                          className="h-11 w-full rounded-xl border border-input bg-background px-4 text-foreground outline-none focus:ring-2 focus:ring-ring"
                        />
                      )}
                    </div>
                  ))}
                </div>
              )}
            {session.status === "in_progress" ? (
              <Button
                type="button"
                disabled={finishing}
                onClick={finishSession}
                className="mt-5 rounded-full"
              >
                {finishing ? "Saving progress…" : "Mark activity complete"}
                <Check className="size-4" />
              </Button>
            ) : (
              <p className="mt-5 text-sm font-medium text-primary">
                Complete. Your next activity is now ready.
              </p>
            )}
          </section>
        )}
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}
      </main>
    </div>
  );
}
