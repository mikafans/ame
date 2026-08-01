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
import { api, publicApi } from "@/api/client";
import { Button } from "@/components/ui/button";
import { PortabilityPanel } from "@/components/learning/PortabilityPanel";
import { LearnerAnalytics } from "@/components/learning/LearnerAnalytics";

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
type ReviewItem = {
  id: string;
  journeyId: string;
  activityId: string;
  dueAt: string;
  intervalDays: number;
  reviewCount: number;
};
type SourceSnapshot = {
  id: string;
  mediaType: string;
  contentSha256: string;
  byteLength: number;
  content: string;
};
type Citation = {
  id: string;
  snapshotId: string;
  quote: string;
  startByte: number;
  endByte: number;
  groundingStatus: string;
  groundingNote: string;
  licenseStatus: string;
  licenseName?: string | null;
};
type NativeJourney = {
  id: string;
  title: string;
  description: string;
  field: string;
  level: string;
  estimatedMinutes: number;
  outcomes: string[];
  sourceSummary: string;
  reviewStatus: string;
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
  const [dueReviews, setDueReviews] = useState<ReviewItem[]>([]);
  const [ratingReviewId, setRatingReviewId] = useState<string | null>(null);
  const [sources, setSources] = useState<SourceSnapshot[]>([]);
  const [citations, setCitations] = useState<Citation[]>([]);
  const [nativeJourneys, setNativeJourneys] = useState<NativeJourney[]>([]);

  useEffect(() => {
    void (async () => {
      const [
        { data, response },
        reviews,
        sourceSnapshots,
        citationCertificates,
        nativeCatalog,
      ] = await Promise.all([
        api.GET("/api/v1/learning/journeys" as never),
        api.GET("/api/v1/reviews/due"),
        api.GET("/api/v1/source-snapshots"),
        api.GET("/api/v1/citations"),
        publicApi.GET("/public/v1/catalog/journeys"),
      ]);
      if (response.ok && data) {
        const nextJourneys = data as JourneySummary[];
        setJourneys(nextJourneys);
      }
      if (reviews.response.ok && reviews.data) {
        setDueReviews(reviews.data as ReviewItem[]);
      }
      if (sourceSnapshots.response.ok && sourceSnapshots.data) {
        setSources(sourceSnapshots.data as SourceSnapshot[]);
      }
      if (citationCertificates.response.ok && citationCertificates.data) {
        setCitations(citationCertificates.data as Citation[]);
      }
      if (nativeCatalog.response.ok && nativeCatalog.data) {
        setNativeJourneys(nativeCatalog.data as NativeJourney[]);
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

  async function rateReview(reviewItemId: string, rating: string) {
    setRatingReviewId(reviewItemId);
    const result = await api.POST("/api/v1/reviews/{review_item_id}/ratings", {
      params: { path: { review_item_id: reviewItemId } },
      body: { rating: rating as "again" | "hard" | "good" | "easy" },
    });
    if (result.response.ok) {
      setDueReviews((items) =>
        items.filter((item) => item.id !== reviewItemId),
      );
    }
    setRatingReviewId(null);
  }

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
        <Button asChild className="mt-7 rounded-full" variant="outline">
          <Link href="/start">
            Start another journey <ArrowRight className="size-4" />
          </Link>
        </Button>
      </section>

      {!loading && journeys[0] && (
        <section
          className="rounded-3xl border border-primary/35 bg-card p-7 shadow-sm sm:p-9"
          data-testid="recommended-next"
        >
          <div className="grid gap-6 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
            <div>
              <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
                Recommended next
              </p>
              <h2 className="mt-3 text-2xl font-bold tracking-tight sm:text-3xl">
                {journeys[0].nextActivityTitle ?? "Review your journey"}
              </h2>
              <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
                {journeys[0].nextActivityTitle
                  ? "This is the next available activity in your course path."
                  : journeys[0].promise}
              </p>
              <div className="mt-5 flex flex-wrap gap-2 text-xs text-muted-foreground">
                <span className="rounded-full bg-muted px-3 py-1.5">
                  {journeys[0].status.replaceAll("_", " ")}
                </span>
                {streaks[journeys[0].id] !== undefined && (
                  <span className="rounded-full bg-muted px-3 py-1.5">
                    {streaks[journeys[0].id]} qualifying{" "}
                    {streaks[journeys[0].id] === 1 ? "day" : "days"}
                  </span>
                )}
              </div>
            </div>
            <Button asChild className="rounded-full px-6">
              <Link
                href={
                  journeys[0].nextActivityId
                    ? `/learning/journeys/${journeys[0].id}#activity-${journeys[0].nextActivityId}`
                    : `/learning/journeys/${journeys[0].id}`
                }
              >
                {journeys[0].nextActivityId
                  ? "Continue learning"
                  : "Review journey"}
                <ArrowRight className="size-4" />
              </Link>
            </Button>
          </div>
        </section>
      )}

      {!loading && dueReviews.length > 0 && (
        <section
          className="rounded-3xl border border-primary/30 bg-card p-7"
          data-testid="due-review-queue"
        >
          <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
            Due for review
          </p>
          <h2 className="mt-2 text-2xl font-bold">
            Strengthen what you learned
          </h2>
          <p className="mt-2 text-sm text-muted-foreground">
            Rate how recall felt. AME schedules the next review from your own
            review history.
          </p>
          <div className="mt-5 space-y-4">
            {dueReviews.map((review) => (
              <article
                className="rounded-2xl border border-border p-5"
                key={review.id}
              >
                <Link
                  className="font-semibold hover:text-primary"
                  href={`/learning/journeys/${review.journeyId}#activity-${review.activityId}`}
                >
                  Review learning activity
                </Link>
                <p className="mt-1 text-xs text-muted-foreground">
                  {review.reviewCount === 0
                    ? "First scheduled review"
                    : `${review.reviewCount} reviews completed`}
                </p>
                <div className="mt-4 flex flex-wrap gap-2">
                  {(["again", "hard", "good", "easy"] as const).map(
                    (rating) => (
                      <Button
                        disabled={ratingReviewId === review.id}
                        key={rating}
                        onClick={() => void rateReview(review.id, rating)}
                        size="sm"
                        type="button"
                        variant={rating === "good" ? "default" : "outline"}
                      >
                        {rating[0].toUpperCase() + rating.slice(1)}
                      </Button>
                    ),
                  )}
                </div>
              </article>
            ))}
          </div>
        </section>
      )}

      {!loading && sources.length > 0 && (
        <section
          className="rounded-3xl border border-border bg-card p-7"
          data-testid="source-library"
        >
          <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
            Source library
          </p>
          <h2 className="mt-2 text-2xl font-bold">What your learning uses</h2>
          <div className="mt-5 grid gap-3">
            {sources.map((source) => (
              <details
                className="rounded-2xl border border-border p-4"
                key={source.id}
              >
                <summary className="cursor-pointer font-semibold">
                  {source.mediaType} · {source.byteLength} bytes
                </summary>
                <p className="mt-2 break-all font-mono text-xs text-muted-foreground">
                  SHA-256 {source.contentSha256}
                </p>
                <pre className="mt-3 max-h-64 overflow-auto whitespace-pre-wrap rounded-xl bg-muted p-4 text-xs">
                  {source.content}
                </pre>
                {citations
                  .filter((citation) => citation.snapshotId === source.id)
                  .map((citation) => (
                    <blockquote
                      className="mt-3 border-l-2 border-primary pl-4 text-sm"
                      data-testid={`citation-${citation.id}`}
                      key={citation.id}
                    >
                      “{citation.quote}”
                      <span className="mt-1 block text-xs text-muted-foreground">
                        Bytes {citation.startByte}–{citation.endByte} ·{" "}
                        {citation.groundingStatus} · {citation.licenseStatus}
                        {citation.licenseName
                          ? ` (${citation.licenseName})`
                          : ""}
                      </span>
                      <span className="mt-1 block text-xs text-muted-foreground">
                        {citation.groundingNote}
                      </span>
                    </blockquote>
                  ))}
              </details>
            ))}
          </div>
        </section>
      )}

      {!loading && journeys.length > 0 && (
        <PortabilityPanel journeys={journeys} />
      )}

      {!loading && journeys[0] && (
        <LearnerAnalytics journeyId={journeys[0].id} />
      )}

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading journeys…</p>
      ) : journeys.length === 0 ? (
        <section className="space-y-5" data-testid="native-journey-catalog">
          <div className="rounded-2xl border border-dashed border-border p-8 text-center">
            <Compass className="mx-auto size-8 text-primary" />
            <h2 className="mt-4 text-xl font-bold">
              Choose a reviewed starting journey.
            </h2>
            <p className="mt-2 text-muted-foreground">
              Start from a maintained set below, or use a freeform intent when
              your topic is not listed.
            </p>
          </div>
          <div className="grid gap-4 lg:grid-cols-2">
            {nativeJourneys.map((journey) => (
              <article
                className="rounded-2xl border border-border bg-card p-6"
                key={journey.id}
              >
                <p className="font-mono text-xs uppercase tracking-[0.12em] text-primary">
                  {journey.field} · {journey.id}
                </p>
                <h3 className="mt-2 text-xl font-bold">{journey.title}</h3>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  {journey.description}
                </p>
                <p className="mt-3 text-xs text-muted-foreground">
                  {journey.level} · about {journey.estimatedMinutes} minutes
                </p>
                <ul className="mt-4 space-y-2 text-sm">
                  {journey.outcomes.map((outcome) => (
                    <li className="flex gap-2" key={outcome}>
                      <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-primary" />
                      <span>{outcome}</span>
                    </li>
                  ))}
                </ul>
                <p className="mt-4 border-t border-border pt-4 text-xs leading-5 text-muted-foreground">
                  Source and review: {journey.sourceSummary}
                </p>
                <Button asChild className="mt-5 rounded-full" size="sm">
                  <Link
                    href={`/start?catalogId=${encodeURIComponent(journey.id)}`}
                  >
                    Start this journey <ArrowRight className="size-4" />
                  </Link>
                </Button>
              </article>
            ))}
          </div>
        </section>
      ) : (
        <section className="grid gap-4">
          {journeys.map((journey) => {
            const journeyChapters = chapters[journey.id] ?? [];
            const chapterActivities = journeyChapters.flatMap(
              (chapter) => chapter.activities,
            );
            const completedActivities = chapterActivities.filter(
              (activity) => activity.status === "completed",
            ).length;
            const progressPercent = chapterActivities.length
              ? Math.round(
                  (completedActivities / chapterActivities.length) * 100,
                )
              : 0;
            const continueHref = journey.nextActivityId
              ? `/learning/journeys/${journey.id}#activity-${journey.nextActivityId}`
              : `/learning/journeys/${journey.id}`;
            return (
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
                    {journeyChapters.length > 0 && (
                      <div
                        className="mt-4 space-y-2 border-t border-border/70 pt-3"
                        data-testid={"journey-chapters-" + journey.id}
                      >
                        <p className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
                          Course path
                        </p>
                        <div
                          className="space-y-2"
                          data-testid={"journey-progress-" + journey.id}
                        >
                          <div className="flex items-center justify-between gap-3 text-xs text-muted-foreground">
                            <span>Course progress</span>
                            <span>
                              {completedActivities}/{chapterActivities.length}{" "}
                              complete · {progressPercent}%
                            </span>
                          </div>
                          <div
                            className="h-2 overflow-hidden rounded-full bg-border"
                            role="progressbar"
                            aria-label="Course progress"
                            aria-valuemin={0}
                            aria-valuemax={100}
                            aria-valuenow={progressPercent}
                          >
                            <div
                              className="h-full rounded-full bg-primary transition-[width]"
                              style={{ width: `${progressPercent}%` }}
                            />
                          </div>
                        </div>
                        <ul className="space-y-2">
                          {journeyChapters.map((chapter, index) => {
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
                      href={continueHref}
                      className="mt-4 inline-flex items-center gap-2 text-sm font-semibold text-primary underline-offset-4 hover:underline"
                    >
                      {journey.nextActivityId
                        ? "Continue learning"
                        : "Review journey"}{" "}
                      <ArrowRight className="size-4" />
                    </Link>
                  </div>
                  <div className="hidden shrink-0 text-primary transition group-hover:translate-x-1 sm:block">
                    <ArrowRight className="size-5" />
                  </div>
                </div>
              </article>
            );
          })}
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
