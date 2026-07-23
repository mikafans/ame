"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import {
  ArrowRight,
  BarChart3,
  CheckCircle2,
  Compass,
  Flame,
  History,
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

type JourneyChapter = {
  id: string;
  title: string;
  summary: string;
  activities: {
    id: string;
    title: string;
    status: string;
  }[];
};

type Snapshot = { mastery: number; confidence: number; evidenceCount: number };
type Objective = { id: string; statement: string };
type ObjectiveProgress = { objective: Objective; snapshot: Snapshot | null };
type StreakEvent = { qualifyingDay: string };
type TimelineEvent = {
  kind: string;
  title: string;
  occurredAt: string;
};

function activityStatusLabel(status: string) {
  if (status === "completed") return "Complete";
  if (status === "ready") return "Ready";
  if (status === "proposed") return "Planned";
  return status.replaceAll("_", " ");
}

export default function LearningHomePage() {
  const [journeys, setJourneys] = useState<JourneySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [chapters, setChapters] = useState<Record<string, JourneyChapter[]>>(
    {},
  );
  const [snapshots, setSnapshots] = useState<Record<string, Snapshot>>({});
  const [objectiveProgress, setObjectiveProgress] = useState<
    Record<string, ObjectiveProgress[]>
  >({});
  const [streaks, setStreaks] = useState<Record<string, number>>({});
  const [timelines, setTimelines] = useState<Record<string, TimelineEvent[]>>(
    {},
  );

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
          const streakResponse = await api.GET(
            "/api/v1/progress/{journey_id}/streaks",
            { params: { path: { journey_id: journey.id } } },
          );
          const timelineResponse = await api.GET(
            "/api/v1/progress/{journey_id}/timeline",
            { params: { path: { journey_id: journey.id } } },
          );
          const streakDays = new Set(
            streakResponse.response.ok && streakResponse.data
              ? (streakResponse.data as StreakEvent[]).map(
                  (event) => event.qualifyingDay,
                )
              : [],
          );
          if (!response.ok) return null;
          const journeyDetail = data as { chapters?: JourneyChapter[] };
          const objectives =
            (data as { objectives?: Objective[] }).objectives ?? [];
          const objectiveProgress = await Promise.all(
            objectives.map(async (objective) => {
              const snapshot = await api.GET(
                "/api/v1/progress/{journey_id}/objectives/{objective_id}",
                {
                  params: {
                    path: {
                      journey_id: journey.id,
                      objective_id: objective.id,
                    },
                  },
                },
              );
              return {
                objective,
                snapshot:
                  snapshot.response.ok && snapshot.data
                    ? (snapshot.data as Snapshot)
                    : null,
              };
            }),
          );
          return {
            journeyId: journey.id,
            chapters: journeyDetail.chapters ?? [],
            snapshot: objectiveProgress[0]?.snapshot ?? null,
            objectiveProgress,
            streakDays,
            timeline:
              timelineResponse.response.ok && timelineResponse.data
                ? (timelineResponse.data as TimelineEvent[])
                : [],
          };
        }),
      );
      setChapters(
        Object.fromEntries(
          results
            .filter(
              (value): value is NonNullable<typeof value> => value !== null,
            )
            .map((value) => [value.journeyId, value.chapters]),
        ),
      );
      setSnapshots(
        Object.fromEntries(
          results
            .filter(
              (value): value is NonNullable<typeof value> => value !== null,
            )
            .filter(
              (value): value is typeof value & { snapshot: Snapshot } =>
                value.snapshot !== null,
            )
            .map((value) => [value.journeyId, value.snapshot]),
        ),
      );
      setObjectiveProgress(
        Object.fromEntries(
          results
            .filter(
              (value): value is NonNullable<typeof value> => value !== null,
            )
            .map((value) => [value.journeyId, value.objectiveProgress]),
        ),
      );
      setStreaks(
        Object.fromEntries(
          results
            .filter(
              (value): value is NonNullable<typeof value> => value !== null,
            )
            .map((value) => [value.journeyId, value.streakDays.size]),
        ),
      );
      setTimelines(
        Object.fromEntries(
          results
            .filter(
              (value): value is NonNullable<typeof value> => value !== null,
            )
            .map((value) => [value.journeyId, value.timeline]),
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
            <article
              key={journey.id}
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
                  {(chapters[journey.id] ?? []).length > 0 && (
                    <div
                      className="mt-4 space-y-2 border-t border-border/70 pt-3"
                      data-testid={"journey-chapters-" + journey.id}
                    >
                      <p className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
                        Course path
                      </p>
                      <ul className="space-y-2">
                        {chapters[journey.id].map((chapter, index) => {
                          const completed = chapter.activities.filter(
                            (activity) => activity.status === "completed",
                          ).length;
                          const nextActivity = chapter.activities.find(
                            (activity) => activity.status === "ready",
                          );
                          return (
                            <li
                              key={chapter.id}
                              className="rounded-xl bg-muted/40 px-3 py-2 text-xs"
                            >
                              <div className="flex items-center justify-between gap-3">
                                <span className="font-medium">
                                  {index + 1}. {chapter.title}
                                </span>
                                <span className="shrink-0 text-muted-foreground">
                                  {completed}/{chapter.activities.length}
                                </span>
                              </div>
                              <p className="mt-1 text-muted-foreground">
                                {nextActivity
                                  ? "Next: " + nextActivity.title
                                  : completed === chapter.activities.length
                                    ? "Complete"
                                    : chapter.summary}
                              </p>
                              <ul className="mt-3 space-y-1 border-t border-border/60 pt-2">
                                {chapter.activities.map((activity) => (
                                  <li key={activity.id}>
                                    <Link
                                      href={`/learning/journeys/${journey.id}#activity-${activity.id}`}
                                      className="flex items-center justify-between gap-3 rounded-lg px-2 py-1.5 text-muted-foreground transition hover:bg-background hover:text-foreground"
                                      data-testid={
                                        "desk-activity-" + activity.id
                                      }
                                    >
                                      <span className="truncate">
                                        {activity.title}
                                      </span>
                                      <span className="shrink-0 text-[0.68rem] uppercase tracking-[0.08em]">
                                        {activityStatusLabel(activity.status)}
                                      </span>
                                    </Link>
                                  </li>
                                ))}
                              </ul>
                            </li>
                          );
                        })}
                      </ul>
                    </div>
                  )}
                  {(snapshots[journey.id] ||
                    streaks[journey.id] !== undefined) && (
                    <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-muted-foreground">
                      {snapshots[journey.id] && (
                        <span className="inline-flex items-center gap-2">
                          <BarChart3 className="size-4 text-primary" />
                          {Math.round(snapshots[journey.id].mastery * 100)}%
                          signal · {snapshots[journey.id].evidenceCount}{" "}
                          evidence
                        </span>
                      )}
                      {streaks[journey.id] !== undefined && (
                        <span className="inline-flex items-center gap-2">
                          <Flame className="size-4 text-primary" />
                          {streaks[journey.id]} qualifying{" "}
                          {streaks[journey.id] === 1 ? "day" : "days"}
                        </span>
                      )}
                    </div>
                  )}
                  {timelines[journey.id]?.length > 0 && (
                    <div className="mt-4 flex items-start gap-2 border-t border-border/70 pt-3 text-xs text-muted-foreground">
                      <History className="mt-0.5 size-4 shrink-0 text-primary" />
                      <span>
                        Recent:{" "}
                        {timelines[journey.id]
                          .slice(-3)
                          .map((event) => event.title)
                          .join(" · ")}
                      </span>
                    </div>
                  )}
                  <Link
                    href={`/learning/journeys/${journey.id}`}
                    className="mt-4 inline-flex items-center gap-2 text-sm font-semibold text-primary underline-offset-4 hover:underline"
                  >
                    Open journey <ArrowRight className="size-4" />
                  </Link>
                </div>
                <div className="hidden shrink-0 text-primary transition group-hover:translate-x-1 sm:block">
                  <ArrowRight className="size-5" />
                </div>
              </div>
            </article>
          ))}
        </section>
      )}

      <section className="space-y-4">
        <div>
          <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
            Signals, not guesses
          </p>
          <h2 className="mt-2 text-2xl font-bold">Objective progress</h2>
          <p className="mt-2 text-sm text-muted-foreground">
            Each objective keeps its own evidence-backed signal as you learn.
          </p>
        </div>
        <div className="grid gap-3 sm:grid-cols-2">
          {journeys.flatMap((journey) =>
            (objectiveProgress[journey.id] ?? []).map(
              ({ objective, snapshot }) => (
                <div
                  key={objective.id}
                  data-testid={`objective-progress-${objective.id}`}
                  className="rounded-2xl border border-border bg-card p-5"
                >
                  <p className="font-medium leading-6">{objective.statement}</p>
                  <p className="mt-3 text-sm text-muted-foreground">
                    {snapshot
                      ? `${Math.round(snapshot.mastery * 100)}% signal · ${snapshot.evidenceCount} evidence`
                      : "No signal yet"}
                  </p>
                </div>
              ),
            ),
          )}
        </div>
      </section>

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
