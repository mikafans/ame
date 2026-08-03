"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import {
  ArrowRight,
  BookOpenCheck,
  Clock3,
  Compass,
  LibraryBig,
} from "lucide-react";
import { api, publicApi } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";
import { Button } from "@/components/ui/button";

type Course = components["schemas"]["LearnerCourseSummaryResponse"];
type NativeJourney = components["schemas"]["NativeJourneyCatalogResponse"];

function progressLabel(course: Course) {
  return `${course.progress.completed}/${course.progress.total} complete`;
}

function courseHref(course: Course) {
  return `/learning/journeys/${course.journeyId}`;
}

export default function LearningHomePage() {
  const [courses, setCourses] = useState<Course[]>([]);
  const [nativeJourneys, setNativeJourneys] = useState<NativeJourney[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedCollection, setSelectedCollection] = useState<
    "in-progress" | "completed"
  >("in-progress");

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      api.GET("/api/v1/learning/courses"),
      publicApi.GET("/public/v1/catalog/journeys"),
    ]).then(([courseResult, catalogResult]) => {
      if (cancelled) return;
      if (courseResult.response.ok && courseResult.data) {
        setCourses(courseResult.data);
      }
      if (catalogResult.response.ok && catalogResult.data) {
        setNativeJourneys(catalogResult.data);
      }
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <section className="mx-auto max-w-4xl py-12" aria-live="polite">
        <p className="text-sm text-muted-foreground">Opening your learning…</p>
      </section>
    );
  }

  if (courses.length > 0) {
    return (
      <CourseLibrary
        courses={courses}
        nativeJourneys={nativeJourneys}
        selectedCollection={selectedCollection}
        setSelectedCollection={setSelectedCollection}
      />
    );
  }

  return <EmptyCourseLibrary nativeJourneys={nativeJourneys} />;
}

function CourseLibrary({
  courses,
  nativeJourneys,
  selectedCollection,
  setSelectedCollection,
}: {
  courses: Course[];
  nativeJourneys: NativeJourney[];
  selectedCollection: "in-progress" | "completed";
  setSelectedCollection: (collection: "in-progress" | "completed") => void;
}) {
  const active = courses.filter((course) => course.status !== "completed");
  const completed = courses.filter((course) => course.status === "completed");
  const visibleCourses =
    selectedCollection === "in-progress" ? active : completed;
  return (
    <main
      className="mx-auto max-w-5xl space-y-10 py-4"
      data-testid="course-library"
    >
      <header className="max-w-3xl">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
              My learning
            </p>
            <h1 className="mt-3 text-3xl font-bold tracking-tight">
              Continue where you left off.
            </h1>
          </div>
          <Button asChild className="rounded-full" variant="outline">
            <Link href="/agent">Start a new course</Link>
          </Button>
        </div>
        <p className="mt-3 leading-7 text-muted-foreground">
          Each course has its own next step, progress, and reviewed sources.
        </p>
      </header>
      <div
        className="border-b border-border"
        role="tablist"
        aria-label="Course collections"
      >
        <CollectionTab
          active={selectedCollection === "in-progress"}
          count={active.length}
          id="in-progress"
          label="In progress"
          onSelect={() => setSelectedCollection("in-progress")}
        />
        <CollectionTab
          active={selectedCollection === "completed"}
          count={completed.length}
          id="completed"
          label="Completed"
          onSelect={() => setSelectedCollection("completed")}
        />
      </div>
      <CourseGroup
        title={
          selectedCollection === "in-progress" ? "In progress" : "Completed"
        }
        courses={visibleCourses}
        empty={
          selectedCollection === "in-progress"
            ? "No courses are in progress."
            : "No completed courses yet."
        }
      />
      {nativeJourneys.length > 0 && (
        <ReviewedStartingPaths nativeJourneys={nativeJourneys} />
      )}
    </main>
  );
}

function CollectionTab({
  active,
  count,
  id,
  label,
  onSelect,
}: {
  active: boolean;
  count: number;
  id: string;
  label: string;
  onSelect: () => void;
}) {
  return (
    <button
      aria-controls={`course-group-${id}`}
      aria-selected={active}
      className={`border-b-2 px-4 py-3 text-sm font-medium transition ${active ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:border-border hover:text-foreground"}`}
      id={`course-tab-${id}`}
      onClick={onSelect}
      role="tab"
      type="button"
    >
      {label} <span className="ml-1 text-xs">{count}</span>
    </button>
  );
}

function CourseGroup({
  title,
  courses,
  empty,
}: {
  title: string;
  courses: Course[];
  empty: string;
}) {
  return (
    <section aria-labelledby={`course-group-${title.toLowerCase()}`}>
      <h2
        id={`course-group-${title === "In progress" ? "in-progress" : "completed"}`}
        className="text-lg font-semibold"
      >
        {title}
      </h2>
      {courses.length === 0 ? (
        <p className="mt-3 text-sm text-muted-foreground">{empty}</p>
      ) : (
        <div className="mt-4 grid gap-4 md:grid-cols-2">
          {courses.map((course) => (
            <article
              className="flex min-h-56 flex-col border border-border bg-card p-5"
              key={course.journeyId}
            >
              <div className="flex items-start justify-between gap-4">
                <div>
                  <p className="text-xs font-medium uppercase tracking-[0.12em] text-primary">
                    {course.status === "completed"
                      ? "Completed"
                      : "In progress"}
                  </p>
                  <h3 className="mt-2 text-xl font-semibold">{course.title}</h3>
                </div>
                <BookOpenCheck className="size-5 shrink-0 text-primary" />
              </div>
              <p className="mt-3 text-sm leading-6 text-muted-foreground">
                {course.goal}
              </p>
              <div className="mt-5 space-y-2 text-sm">
                <p>{progressLabel(course)}</p>
                {course.currentModule && (
                  <p className="text-muted-foreground">
                    Module {course.currentModule.position + 1}:{" "}
                    {course.currentModule.title}
                  </p>
                )}
                {course.review.dueCount > 0 && (
                  <p className="text-primary">
                    {course.review.dueCount} review
                    {course.review.dueCount === 1 ? "" : "s"} due
                  </p>
                )}
              </div>
              <Button asChild className="mt-auto w-fit" size="sm">
                <Link href={courseHref(course)}>
                  {course.next ? "Continue" : "View course"}
                  <ArrowRight className="size-4" />
                </Link>
              </Button>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function EmptyCourseLibrary({
  nativeJourneys,
}: {
  nativeJourneys: NativeJourney[];
}) {
  return (
    <main
      className="mx-auto max-w-5xl space-y-10 py-4"
      data-testid="native-journey-catalog"
    >
      <section className="border border-dashed border-border bg-card p-7 sm:p-9">
        <Compass className="size-7 text-primary" />
        <p className="mt-5 font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Your private learning space
        </p>
        <h1 className="mt-3 text-3xl font-bold tracking-tight">
          No journeys in this account yet.
        </h1>
        <p className="mt-3 max-w-2xl leading-7 text-muted-foreground">
          This is your private course library. Ask an agent to prepare a course
          for your goal, or begin one of the reviewed starting paths below.
        </p>
        <Button asChild className="mt-6 rounded-full">
          <Link href="/agent">How an agent sets up a course</Link>
        </Button>
      </section>
      {nativeJourneys.length > 0 && (
        <ReviewedStartingPaths nativeJourneys={nativeJourneys} />
      )}
    </main>
  );
}

function ReviewedStartingPaths({
  nativeJourneys,
}: {
  nativeJourneys: NativeJourney[];
}) {
  return (
    <section>
      <div className="flex items-center gap-3">
        <LibraryBig className="size-5 text-primary" />
        <div>
          <h2 className="text-xl font-semibold">Reviewed starting paths</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            These are optional starting points, not another learner's course.
          </p>
        </div>
      </div>
      <div className="mt-5 grid gap-4 md:grid-cols-2">
        {nativeJourneys.map((journey) => (
          <article
            className="border border-border bg-card p-5"
            key={journey.id}
          >
            <p className="font-mono text-xs uppercase tracking-[0.12em] text-primary">
              {journey.id}
            </p>
            <h3 className="mt-2 text-lg font-semibold">{journey.title}</h3>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">
              {journey.description}
            </p>
            <p className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
              <Clock3 className="size-3.5" /> About {journey.estimatedMinutes}{" "}
              minutes
            </p>
            <Button
              asChild
              className="mt-5 rounded-full"
              size="sm"
              variant="outline"
            >
              <Link href={`/start?catalogId=${encodeURIComponent(journey.id)}`}>
                Start this path <ArrowRight className="size-4" />
              </Link>
            </Button>
          </article>
        ))}
      </div>
    </section>
  );
}
