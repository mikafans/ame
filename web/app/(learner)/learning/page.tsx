"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { ArrowRight, CheckCircle2, Compass } from "lucide-react";
import { api, publicApi } from "@/api/client";
import { Button } from "@/components/ui/button";

type JourneySummary = {
  id: string;
  promise: string;
  status: string;
  rawIntent: string;
  nextActivityId: string | null;
  nextActivityTitle: string | null;
};

type JourneyChapter = {
  id: string;
  title: string;
  summary: string;
  activities: { id: string; title: string; status: string }[];
};

type ReviewItem = {
  id: string;
  journeyId: string;
  activityId: string;
  reviewCount: number;
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
  sources: {
    title: string;
    url: string;
    locator?: string | null;
    license: string;
  }[];
  contentReview: { status: string; reviewedAt: string; reviewer: string };
};

export default function LearningHomePage() {
  const [journeys, setJourneys] = useState<JourneySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [chapters, setChapters] = useState<JourneyChapter[]>([]);
  const [dueReviews, setDueReviews] = useState<ReviewItem[]>([]);
  const [ratingReviewId, setRatingReviewId] = useState<string | null>(null);
  const [nativeJourneys, setNativeJourneys] = useState<NativeJourney[]>([]);

  const activeJourney = useMemo(
    () =>
      journeys.find((journey) => journey.nextActivityId !== null) ??
      journeys.find((journey) => journey.status === "active") ??
      journeys[0] ??
      null,
    [journeys],
  );

  useEffect(() => {
    void (async () => {
      const [{ data, response }, reviews, nativeCatalog] = await Promise.all([
        api.GET("/api/v1/learning/journeys"),
        api.GET("/api/v1/reviews/due"),
        publicApi.GET("/public/v1/catalog/journeys"),
      ]);
      if (response.ok && data) setJourneys(data as JourneySummary[]);
      if (reviews.response.ok && reviews.data) {
        setDueReviews(reviews.data as ReviewItem[]);
      }
      if (nativeCatalog.response.ok && nativeCatalog.data) {
        setNativeJourneys(nativeCatalog.data as NativeJourney[]);
      }
      setLoading(false);
    })();
  }, []);

  useEffect(() => {
    if (!activeJourney) {
      setChapters([]);
      return;
    }
    void (async () => {
      const { data, response } = await api.GET(
        "/api/v1/learning/journeys/{id}",
        { params: { path: { id: activeJourney.id } } },
      );
      if (response.ok && data) {
        setChapters((data as { chapters?: JourneyChapter[] }).chapters ?? []);
      }
    })();
  }, [activeJourney]);

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

  if (loading) {
    return (
      <main className="mx-auto w-full max-w-4xl px-4 py-8 sm:px-8 sm:py-12">
        <p className="text-sm text-muted-foreground">Loading your course…</p>
      </main>
    );
  }

  if (!activeJourney) {
    return (
      <main className="mx-auto w-full max-w-5xl space-y-5 px-4 py-8 sm:px-8 sm:py-12">
        <section className="rounded-2xl border border-dashed border-border p-8 text-center">
          <Compass className="mx-auto size-8 text-primary" />
          <h1 className="mt-4 text-xl font-bold">Choose your first course</h1>
          <p className="mt-2 text-muted-foreground">
            Start with a reviewed path, describe a goal, or ask your agent to
            set up a course for you.
          </p>
          <Button asChild className="mt-5 rounded-full" variant="outline">
            <Link href="/agent">How an agent sets up a course</Link>
          </Button>
        </section>
        <section data-testid="native-journey-catalog">
          <h2 className="text-lg font-bold">Reviewed starting paths</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
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
                <ul className="mt-2 space-y-1 text-xs text-muted-foreground">
                  {journey.sources.map((source) => (
                    <li key={source.url}>
                      <a
                        className="underline underline-offset-2 hover:text-foreground"
                        href={source.url}
                        rel="noreferrer"
                        target="_blank"
                      >
                        {source.title}
                      </a>
                      {source.locator ? ` · ${source.locator}` : ""} ·{" "}
                      {source.license}
                    </li>
                  ))}
                </ul>
                <p className="mt-2 text-xs text-muted-foreground">
                  {journey.contentReview.status} by{" "}
                  {journey.contentReview.reviewer} ·{" "}
                  {journey.contentReview.reviewedAt}
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
      </main>
    );
  }

  const activities = chapters.flatMap((chapter) => chapter.activities);
  const completedActivities = activities.filter(
    (activity) => activity.status === "completed",
  ).length;
  const progressPercent = activities.length
    ? Math.round((completedActivities / activities.length) * 100)
    : 0;
  const continueHref = activeJourney.nextActivityId
    ? `/learning/journeys/${activeJourney.id}#activity-${activeJourney.nextActivityId}`
    : `/learning/journeys/${activeJourney.id}`;
  const activeReviews = dueReviews.filter(
    (review) => review.journeyId === activeJourney.id,
  );

  return (
    <main
      className="mx-auto w-full max-w-4xl space-y-6 px-4 py-8 sm:px-8 sm:py-12"
      data-testid="active-learning-desk"
    >
      <section
        className="rounded-3xl border border-primary/35 bg-card p-7 shadow-sm sm:p-9"
        data-testid="recommended-next"
      >
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Your active course
        </p>
        <div className="mt-3 flex flex-col gap-5 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h1 className="max-w-2xl text-3xl font-extrabold tracking-tight sm:text-4xl">
              {activeJourney.rawIntent}
            </h1>
            <p className="mt-3 max-w-2xl leading-7 text-muted-foreground">
              {activeJourney.promise}
            </p>
          </div>
          <Button asChild className="shrink-0 rounded-full px-6">
            <Link href={continueHref}>
              {activeJourney.nextActivityId
                ? "Continue learning"
                : "Review course"}{" "}
              <ArrowRight className="size-4" />
            </Link>
          </Button>
        </div>
        <div className="mt-7 border-t border-border pt-5">
          <p className="text-sm text-muted-foreground">Next step</p>
          <h2 className="mt-1 text-xl font-bold">
            {activeJourney.nextActivityTitle ?? "Review your course"}
          </h2>
        </div>
      </section>

      <section
        className="rounded-2xl border border-border bg-card p-6"
        data-testid={`journey-chapters-${activeJourney.id}`}
      >
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.12em] text-primary">
              Course path
            </p>
            <h2 className="mt-2 text-xl font-bold">Your progress</h2>
          </div>
          <span className="text-sm text-muted-foreground">
            {completedActivities}/{activities.length} complete
          </span>
        </div>
        <div
          className="mt-5"
          data-testid={`journey-progress-${activeJourney.id}`}
        >
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
          <p className="mt-2 text-xs text-muted-foreground">
            {progressPercent}% complete
          </p>
        </div>
        <ol className="mt-5 space-y-3">
          {chapters.map((chapter, index) => {
            const complete = chapter.activities.filter(
              (activity) => activity.status === "completed",
            ).length;
            const isCurrent = chapter.activities.some(
              (activity) => activity.id === activeJourney.nextActivityId,
            );
            return (
              <li className="flex gap-3" key={chapter.id}>
                <span className="flex size-6 shrink-0 items-center justify-center rounded-full border border-border text-xs font-semibold">
                  {index + 1}
                </span>
                <div className="min-w-0">
                  <p className="font-semibold">{chapter.title}</p>
                  <p className="mt-0.5 text-sm text-muted-foreground">
                    {isCurrent
                      ? `Current: ${activeJourney.nextActivityTitle}`
                      : complete === chapter.activities.length
                        ? "Complete"
                        : chapter.summary}
                  </p>
                </div>
                <span className="ml-auto shrink-0 text-sm text-muted-foreground">
                  {complete}/{chapter.activities.length}
                </span>
              </li>
            );
          })}
        </ol>
      </section>

      {activeReviews.length > 0 && (
        <section
          className="rounded-2xl border border-border bg-card p-6"
          data-testid="due-review-queue"
        >
          <p className="font-mono text-xs uppercase tracking-[0.12em] text-primary">
            Due in this course
          </p>
          <h2 className="mt-2 text-xl font-bold">
            Strengthen what you learned
          </h2>
          <div className="mt-5 space-y-4">
            {activeReviews.map((review) => (
              <article
                className="rounded-xl border border-border p-4"
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
    </main>
  );
}
