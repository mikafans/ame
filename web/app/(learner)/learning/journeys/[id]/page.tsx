"use client";

import { type ReactNode, useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import {
  ArrowRight,
  BookOpen,
  Check,
  ChevronDown,
  Clock3,
  FileText,
  History,
  LibraryBig,
  TrendingUp,
} from "lucide-react";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";
import { Button } from "@/components/ui/button";
import { ActivityContentRenderer } from "@/components/learning/ActivityContentRenderer";
import { AssessmentResultFeedback } from "@/components/learning/AssessmentResultFeedback";
import { ActivityNotes } from "@/components/learning/ActivityNotes";
import { ActivityVariants } from "@/components/learning/ActivityVariants";

type Journey = components["schemas"]["LearningJourneyResponse"];
type LearningSession = components["schemas"]["LearningSessionResponse"];
type Assessment = components["schemas"]["AssessmentResponse"];
type AssessmentItem = components["schemas"]["AssessmentItemResponse"];
type Attempt = components["schemas"]["AttemptResponse"];
type DeepDive = components["schemas"]["DeepDiveResponse"];
type LearnerView = components["schemas"]["LearnerViewResponse"];
type LearnerCourse = components["schemas"]["LearnerCourseResponse"];
type LearnerProgress = components["schemas"]["LearnerProgressViewResponse"];
type LearnerResources = components["schemas"]["LearnerResourcesResponse"];
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
type ActivityPayload = {
  content?: StarterContent | Record<string, unknown>;
  contentProvenance?: {
    sourceReferences?: string[];
  };
};

function isStarterContent(content: unknown): content is StarterContent {
  if (!content || typeof content !== "object") return false;
  const candidate = content as Partial<StarterContent>;
  return (
    candidate.type === "starter_check" &&
    typeof candidate.instructions === "string" &&
    Array.isArray(candidate.questions)
  );
}

function ActivityProvenance({
  sourceReferences,
}: {
  sourceReferences: string[];
}) {
  if (sourceReferences.length === 0) return null;
  return (
    <div className="mt-5 rounded-xl border border-primary/20 bg-background/70 p-4">
      <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
        Source-backed activity
      </p>
      <p className="mt-2 text-sm text-muted-foreground">
        Grounded in {sourceReferences.length} cited source
        {sourceReferences.length === 1 ? "" : "s"}.
      </p>
    </div>
  );
}

function ActivityRow({
  activity,
  startingActivity,
  activeActivityId,
  onStart,
  onReview,
}: {
  activity: Journey["activities"][number];
  startingActivity: string | null;
  activeActivityId: string | null;
  onStart: (activityId: string) => void;
  onReview: (activityId: string) => void;
}) {
  const isActive = activeActivityId === activity.id;
  const isReady = activity.status === "ready";
  const isCompleted = activity.status === "completed";
  const statusLabel = isActive
    ? "In progress"
    : isCompleted
      ? "Complete"
      : activity.status === "proposed"
        ? "Planned"
        : activity.status.replaceAll("_", " ");

  return (
    <article
      className="scroll-mt-6 flex items-center justify-between gap-4 border-b border-border py-5 last:border-b-0"
      data-testid={"learning-activity-" + activity.id}
      id={"activity-" + activity.id}
    >
      <div className="min-w-0">
        <p className="text-xs uppercase tracking-[0.12em] text-muted-foreground">
          {activity.kind} · {statusLabel}
        </p>
        <h4 className="mt-1 font-medium">{activity.title}</h4>
      </div>
      {isReady && (
        <Button
          type="button"
          disabled={startingActivity !== null}
          onClick={() => onStart(activity.id)}
          className="shrink-0 rounded-full"
        >
          {startingActivity === activity.id
            ? "Starting…"
            : isActive
              ? "Resume"
              : "Begin"}
          <ArrowRight className="size-4" />
        </Button>
      )}
      {isCompleted && (
        <div className="flex shrink-0 items-center gap-3">
          <span className="inline-flex items-center gap-2 text-sm font-medium text-primary">
            <Check className="size-4" /> Done
          </span>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => onReview(activity.id)}
            className="rounded-full"
          >
            Review
          </Button>
        </div>
      )}
      {!isReady && !isCompleted && (
        <span className="shrink-0 text-xs font-medium uppercase tracking-[0.1em] text-muted-foreground">
          {statusLabel}
        </span>
      )}
    </article>
  );
}

function ChapterSection({
  chapter,
  index,
  completedActivities,
  isComplete,
  isCurrent,
  children,
}: {
  chapter: Journey["chapters"][number];
  index: number;
  completedActivities: number;
  isComplete: boolean;
  isCurrent: boolean;
  children: ReactNode;
}) {
  const [expanded, setExpanded] = useState(isCurrent);

  useEffect(() => {
    if (isCurrent) setExpanded(true);
  }, [isCurrent]);

  return (
    <section
      className={
        isCurrent
          ? "rounded-2xl border border-primary/60 bg-card px-5 py-2 shadow-[0_0_0_3px_hsl(var(--primary)/0.08)]"
          : "rounded-2xl border border-border bg-card px-5 py-2"
      }
      data-current={isCurrent ? "true" : "false"}
      data-complete={isComplete ? "true" : "false"}
      data-testid={"learning-chapter-" + chapter.id}
    >
      <div className="flex items-center justify-between gap-3 py-4">
        <div>
          <span className="flex items-center gap-2 text-xs font-bold uppercase tracking-[0.12em] text-primary">
            Chapter {index + 1}
            {isComplete && (
              <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[0.65rem] tracking-[0.08em]">
                Complete
              </span>
            )}
            {isCurrent && (
              <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[0.65rem] tracking-[0.08em]">
                Current
              </span>
            )}
          </span>
          <h3 className="mt-1 text-lg font-semibold">{chapter.title}</h3>
        </div>
        <button
          type="button"
          aria-label={`${expanded ? "Collapse" : "Expand"} Chapter ${index + 1}: ${chapter.title}`}
          aria-controls={"chapter-content-" + chapter.id}
          aria-expanded={expanded}
          className="flex shrink-0 items-center gap-3 text-xs text-muted-foreground"
          onClick={() => setExpanded((current) => !current)}
        >
          <span>
            {completedActivities}/{chapter.activities.length} complete
          </span>
          <ChevronDown
            className={
              expanded
                ? "size-5 rotate-180 transition-transform"
                : "size-5 transition-transform"
            }
          />
        </button>
      </div>
      {expanded && (
        <div id={"chapter-content-" + chapter.id}>
          <div className="border-t border-border py-4">
            <p className="text-sm leading-6 text-muted-foreground">
              {chapter.summary}
            </p>
          </div>
          {children}
        </div>
      )}
    </section>
  );
}

const learnerTabs = [
  { id: "learn", label: "Learn", icon: BookOpen },
  { id: "course", label: "Course", icon: FileText },
  { id: "progress", label: "Progress", icon: TrendingUp },
  { id: "resources", label: "Sources", icon: LibraryBig },
] as const;

function learnerTabHref(journeyId: string, tab: string) {
  return tab === "learn"
    ? `/learning/journeys/${journeyId}`
    : `/learning/journeys/${journeyId}?tab=${tab}`;
}

function CourseTab({ course }: { course: LearnerCourse | null }) {
  if (!course) {
    return (
      <p className="text-sm text-muted-foreground">Loading course outline…</p>
    );
  }
  return (
    <section className="space-y-8" data-testid="learner-course-tab">
      <div className="max-w-3xl">
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Course overview
        </p>
        <h2 className="mt-3 text-3xl font-bold tracking-tight">
          {course.title}
        </h2>
        <p className="mt-3 leading-7 text-muted-foreground">{course.promise}</p>
        <dl className="mt-6 grid gap-4 border-y border-border py-5 text-sm sm:grid-cols-3">
          <div>
            <dt className="text-muted-foreground">Prepared for</dt>
            <dd className="mt-1 font-medium">
              {course.audience ?? "Your stated goal"}
            </dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Estimated effort</dt>
            <dd className="mt-1 font-medium">
              {course.estimatedMinutes
                ? `About ${course.estimatedMinutes} minutes`
                : "Set by your pace"}
            </dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Course status</dt>
            <dd className="mt-1 font-medium">
              {course.publication.reviewed && course.publication.sourceReviewed
                ? "Reviewed and source-grounded"
                : "Published course"}
            </dd>
          </div>
        </dl>
      </div>
      {course.prerequisites.length > 0 && (
        <section>
          <h3 className="text-lg font-semibold">Before you begin</h3>
          <ul className="mt-3 list-disc space-y-2 pl-5 text-sm leading-6 text-muted-foreground">
            {course.prerequisites.map((prerequisite) => (
              <li key={prerequisite}>{prerequisite}</li>
            ))}
          </ul>
        </section>
      )}
      <section>
        <h3 className="text-lg font-semibold">What you will be able to do</h3>
        <ol className="mt-4 grid gap-3 md:grid-cols-2">
          {course.outcomes.map((outcome) => (
            <li className="border border-border bg-card p-4" key={outcome.id}>
              <p className="font-medium">{outcome.statement}</p>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                Evidence: {outcome.successCriteria}
              </p>
            </li>
          ))}
        </ol>
      </section>
      <section>
        <h3 className="text-lg font-semibold">Modules</h3>
        <ol className="mt-4 space-y-3">
          {course.modules.map((module) => (
            <li className="border border-border bg-card p-5" key={module.id}>
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <p className="text-xs font-medium uppercase tracking-[0.12em] text-primary">
                    Module {module.position + 1} ·{" "}
                    {module.state.replaceAll("_", " ")}
                  </p>
                  <h4 className="mt-2 text-lg font-semibold">{module.title}</h4>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    {module.summary}
                  </p>
                </div>
                <span className="text-sm text-muted-foreground">
                  {module.completion.completed}/{module.completion.total}{" "}
                  complete
                </span>
              </div>
              <p className="mt-4 text-xs text-muted-foreground">
                {module.activityKinds
                  .map((kind) => kind.replaceAll("_", " "))
                  .join(" · ")}
              </p>
            </li>
          ))}
        </ol>
      </section>
    </section>
  );
}

function ProgressTab({ progress }: { progress: LearnerProgress | null }) {
  if (!progress)
    return (
      <p className="text-sm text-muted-foreground">Loading your progress…</p>
    );
  return (
    <section className="space-y-8" data-testid="learner-progress-tab">
      <div>
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Evidence, not guesses
        </p>
        <h2 className="mt-3 text-3xl font-bold tracking-tight">
          Your progress
        </h2>
        <p className="mt-3 leading-7 text-muted-foreground">
          Progress appears after you complete work. AME does not turn an
          unattempted outcome into a score.
        </p>
      </div>
      <section>
        <h3 className="text-lg font-semibold">Outcomes</h3>
        <div className="mt-4 grid gap-3 md:grid-cols-2">
          {progress.outcomes.map((outcome) => (
            <article
              className="border border-border bg-card p-4"
              key={outcome.objectiveId}
            >
              <p className="font-medium">{outcome.statement}</p>
              <p className="mt-2 text-sm text-muted-foreground">
                {outcome.evidenceStatus === "not_assessed_yet"
                  ? "Not assessed yet"
                  : `${outcome.evidenceCount} evidence signal${outcome.evidenceCount === 1 ? "" : "s"}`}
              </p>
            </article>
          ))}
        </div>
      </section>
      <section>
        <h3 className="text-lg font-semibold">Attempts</h3>
        {progress.attempts.length === 0 ? (
          <p className="mt-3 text-sm text-muted-foreground">
            Your assessment feedback will appear here after your first attempt.
          </p>
        ) : (
          <ul className="mt-4 space-y-3">
            {progress.attempts.map((attempt) => (
              <li
                className="flex flex-wrap items-center justify-between gap-3 border border-border bg-card p-4"
                key={attempt.id}
              >
                <span className="font-medium">{attempt.activityTitle}</span>
                <span className="text-sm text-muted-foreground">
                  {attempt.score !== null && attempt.maxScore !== null
                    ? `${attempt.score}/${attempt.maxScore}`
                    : attempt.status.replaceAll("_", " ")}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
      {progress.dueReviews.length > 0 && (
        <section>
          <h3 className="text-lg font-semibold">Due to review</h3>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            {progress.dueReviews.length} item
            {progress.dueReviews.length === 1 ? "" : "s"} in this course are
            ready for retrieval practice.
          </p>
        </section>
      )}
    </section>
  );
}

function ResourcesTab({ resources }: { resources: LearnerResources | null }) {
  if (!resources)
    return (
      <p className="text-sm text-muted-foreground">Loading reviewed sources…</p>
    );
  return (
    <section className="space-y-8" data-testid="learner-resources-tab">
      <div>
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Source grounding
        </p>
        <h2 className="mt-3 text-3xl font-bold tracking-tight">
          Sources for this course
        </h2>
        <p className="mt-3 max-w-3xl leading-7 text-muted-foreground">
          These are the reviewed sources that shaped this course. They are shown
          in course context, without exposing raw imported files or internal
          storage details.
        </p>
      </div>
      {resources.sources.length === 0 ? (
        <p className="border border-dashed border-border p-5 text-sm text-muted-foreground">
          No reviewed source has been attached to this published course.
        </p>
      ) : (
        <ul className="space-y-4">
          {resources.sources.map((source) => (
            <li
              className="border border-border bg-card p-5"
              key={`${source.locator}-${source.quote}`}
            >
              <h3 className="font-semibold">{source.title}</h3>
              <a
                className="mt-2 block break-all text-sm text-primary underline underline-offset-4"
                href={source.locator}
                rel="noreferrer"
                target="_blank"
              >
                {source.locator}
              </a>
              <p className="mt-4 border-l-2 border-primary/40 pl-4 text-sm leading-6 text-muted-foreground">
                {source.quote}
              </p>
              <p className="mt-4 text-sm leading-6 text-muted-foreground">
                Why it is here: {source.relevance}
              </p>
              <p className="mt-3 text-xs text-muted-foreground">
                Grounding: {source.groundingStatus.replaceAll("_", " ")} ·
                License: {source.licenseName ?? source.licenseStatus}
              </p>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export default function LearningJourneyPage() {
  const params = useParams<{ id: string }>();
  const searchParams = useSearchParams();
  const [journey, setJourney] = useState<Journey | null>(null);
  const [learnerView, setLearnerView] = useState<LearnerView | null>(null);
  const [learnerViewLoaded, setLearnerViewLoaded] = useState(false);
  const [course, setCourse] = useState<LearnerCourse | null>(null);
  const [progress, setProgress] = useState<LearnerProgress | null>(null);
  const [resources, setResources] = useState<LearnerResources | null>(null);
  const [session, setSession] = useState<LearningSession | null>(null);
  const [assessment, setAssessment] = useState<Assessment | null>(null);
  const [attempt, setAttempt] = useState<Attempt | null>(null);
  const [attemptHistory, setAttemptHistory] = useState<Attempt[]>([]);
  const [deepDive, setDeepDive] = useState<DeepDive | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [startingActivity, setStartingActivity] = useState<string | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [assessmentLoading, setAssessmentLoading] = useState(false);
  const [responses, setResponses] = useState<Record<string, string>>({});
  const [assessmentResponses, setAssessmentResponses] = useState<
    Record<string, string>
  >({});
  const [reviewActivityId, setReviewActivityId] = useState<string | null>(null);
  const requestedTab = searchParams.get("tab");
  const selectedTab =
    requestedTab === "course" ||
    requestedTab === "progress" ||
    requestedTab === "resources"
      ? requestedTab
      : "learn";

  async function loadDeepDive(activityId: string) {
    const result = await api.GET("/api/v1/deep-dives", {
      params: { query: { activityId } },
    });
    if (result.response.status === 404) {
      setDeepDive(null);
      return;
    }
    if (!result.response.ok || !result.data) {
      throw new Error("Could not load the activity explanation");
    }
    setDeepDive(result.data);
  }

  async function loadAssessment(activityId: string, learningSessionId: string) {
    setAssessmentLoading(true);
    try {
      const result = await api.GET("/api/v1/assessments", {
        params: { query: { activityId } },
      });
      if (result.response.status === 404) {
        setAssessment(null);
        setAttempt(null);
        return;
      }
      if (!result.response.ok || !result.data) {
        throw new Error("Could not load the activity assessment");
      }
      setAssessment(result.data);
      const attemptResult = await api.POST(
        "/api/v1/assessments/{assessment_id}/attempts",
        {
          params: { path: { assessment_id: result.data.id } },
          body: { learningSessionId },
        },
      );
      if (!attemptResult.response.ok || !attemptResult.data) {
        throw new Error("Could not start the activity assessment");
      }
      setAttempt(attemptResult.data);
    } finally {
      setAssessmentLoading(false);
    }
  }

  const loadAttemptHistory = useCallback(async () => {
    const result = await api.GET(
      "/api/v1/learning/journeys/{journey_id}/attempts",
      { params: { path: { journey_id: params.id } } },
    );
    if (!result.response.ok || !result.data) {
      throw new Error("Could not load assessment history");
    }
    setAttemptHistory(result.data);
  }, [params.id]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const journeyResult = await api.GET("/api/v1/learning/journeys/{id}", {
          params: { path: { id: params.id } },
        });
        if (!journeyResult.response.ok || !journeyResult.data) {
          throw new Error("Could not load this learning journey");
        }
        if (cancelled) return;
        setJourney(journeyResult.data);
        await loadAttemptHistory();
        const storedSessionId = window.localStorage.getItem(
          `ame-learning-session:${params.id}`,
        );
        if (!storedSessionId) {
          const completedActivity = journeyResult.data.activities
            .filter((activity) => activity.status === "completed")
            .sort((left, right) => right.orderIndex - left.orderIndex)[0];
          if (completedActivity) {
            await loadDeepDive(completedActivity.id);
          }
          return;
        }
        const sessionResult = await api.GET("/api/v1/learning/sessions/{id}", {
          params: { path: { id: storedSessionId } },
        });
        if (
          cancelled ||
          !sessionResult.response.ok ||
          !sessionResult.data ||
          sessionResult.data.journeyId !== params.id
        ) {
          return;
        }
        setSession(sessionResult.data);
        if (sessionResult.data.status === "in_progress") {
          await loadAssessment(sessionResult.data.activityId, storedSessionId);
        } else {
          await loadDeepDive(sessionResult.data.activityId);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Could not load this journey",
          );
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [loadAttemptHistory, params.id]);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      setLearnerViewLoaded(false);
      setLearnerView(null);
      setCourse(null);
      setProgress(null);
      setResources(null);
      const viewResult = await api.GET(
        "/api/v1/learning/journeys/{id}/learner-view",
        { params: { path: { id: params.id } } },
      );
      if (cancelled) return;
      setLearnerViewLoaded(true);
      if (!viewResult.response.ok || !viewResult.data) return;
      setLearnerView(viewResult.data);
      if (selectedTab === "course") {
        const result = await api.GET("/api/v1/learning/journeys/{id}/course", {
          params: { path: { id: params.id } },
        });
        if (!cancelled && result.response.ok && result.data)
          setCourse(result.data);
      }
      if (selectedTab === "progress") {
        const result = await api.GET(
          "/api/v1/learning/journeys/{id}/progress-view",
          { params: { path: { id: params.id } } },
        );
        if (!cancelled && result.response.ok && result.data)
          setProgress(result.data);
      }
      if (selectedTab === "resources") {
        const result = await api.GET(
          "/api/v1/learning/journeys/{id}/resources",
          {
            params: { path: { id: params.id } },
          },
        );
        if (!cancelled && result.response.ok && result.data)
          setResources(result.data);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [params.id, selectedTab]);

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
      setAssessmentResponses({});
      setAssessment(null);
      setAttempt(null);
      setDeepDive(null);
      setReviewActivityId(null);
      window.localStorage.setItem(`ame-learning-session:${params.id}`, data.id);
      await loadAssessment(activityId, data.id);
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

  function responseForAssessmentItem(item: AssessmentItem): unknown {
    const value = assessmentResponses[item.id];
    if (
      item.question.kind === "multiple_choice" ||
      item.question.kind === "true_false"
    ) {
      return { option_id: value };
    }
    return { text: value };
  }

  async function finishAssessment() {
    if (!assessment || !attempt || !session) return;
    const unanswered = assessment.items.find(
      (item) => !assessmentResponses[item.id]?.trim(),
    );
    if (unanswered) {
      throw new Error("Answer every assessment question before submitting");
    }
    for (const item of assessment.items) {
      const result = await api.POST("/api/v1/attempts/{attempt_id}/answers", {
        params: { path: { attempt_id: attempt.id } },
        body: {
          assessmentItemId: item.id,
          questionVersionId: item.questionVersionId,
          response: responseForAssessmentItem(item),
        },
      });
      if (!result.response.ok || !result.data) {
        throw new Error("Could not save the assessment answer");
      }
    }
    const finishedAttempt = await api.POST(
      "/api/v1/attempts/{attempt_id}/finish",
      { params: { path: { attempt_id: attempt.id } } },
    );
    if (!finishedAttempt.response.ok || !finishedAttempt.data) {
      throw new Error("Could not grade the assessment");
    }
    setAttempt(finishedAttempt.data);
    if (finishedAttempt.data.status === "graded") {
      const evidence = await api.POST("/api/v1/progress/evidence", {
        body: {
          journeyId: params.id,
          objectiveId: journey?.objectives[0]?.id ?? "",
          activityId: session.activityId,
          attemptId: attempt.id,
          contentVersion:
            journey?.activities.find(
              (activity) => activity.id === session.activityId,
            )?.contentVersion ?? 1,
          value: finishedAttempt.data.score ?? 0,
          derivationVersion: 1,
        },
      });
      if (!evidence.response.ok) {
        throw new Error("Could not record assessment evidence");
      }
    }
  }

  async function finishSession() {
    if (!session) return;
    setFinishing(true);
    setError(null);
    try {
      if (assessment && attempt?.status === "in_progress") {
        await finishAssessment();
      }
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
      if (refreshed.response.ok && refreshed.data) {
        setJourney(refreshed.data);
        await loadDeepDive(session.activityId);
      }
      await loadAttemptHistory();
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
  const reviewActivity = reviewActivityId
    ? (journey.activities.find(
        (activity) => activity.id === reviewActivityId,
      ) ?? null)
    : null;
  const reviewPayload = reviewActivity?.payload as ActivityPayload | undefined;
  const reviewContent = reviewPayload?.content ?? null;
  const activeActivityId =
    session?.status === "in_progress" ? session.activityId : null;
  const activePayload = activeActivity?.payload as unknown as
    | ActivityPayload
    | undefined;
  const activeContent = activePayload?.content ?? null;
  const activeProvenance = activePayload?.contentProvenance ?? null;
  const ungroupedActivities = journey.activities.filter(
    (activity) =>
      activity.chapterId === null || activity.chapterId === undefined,
  );
  const pathActivities = [
    ...journey.chapters.flatMap((chapter) => chapter.activities),
    ...ungroupedActivities,
  ];
  const completedPathActivities = pathActivities.filter(
    (activity) => activity.status === "completed",
  ).length;
  const pathProgressPercent = pathActivities.length
    ? Math.round((completedPathActivities / pathActivities.length) * 100)
    : 0;
  const continueActivity =
    activeActivityId && activeActivity ? activeActivity : readyActivity;
  const currentChapterId =
    journey.chapters.find((chapter) =>
      chapter.activities.some((activity) => activity.id === activeActivityId),
    )?.id ??
    journey.chapters.find((chapter) =>
      chapter.activities.some((activity) => activity.id === readyActivity?.id),
    )?.id ??
    journey.chapters.find((chapter) =>
      chapter.activities.some((activity) => activity.status !== "completed"),
    )?.id ??
    null;
  const currentChapterIndex = journey.chapters.findIndex(
    (chapter) => chapter.id === currentChapterId,
  );
  const completedChapterBeforeCurrent =
    currentChapterIndex > 0 ? journey.chapters[currentChapterIndex - 1] : null;
  const chapterHandoff =
    completedChapterBeforeCurrent?.activities.every(
      (activity) => activity.status === "completed",
    ) && currentChapterIndex > 0
      ? {
          completed: completedChapterBeforeCurrent,
          next: journey.chapters[currentChapterIndex],
        }
      : null;
  const showLegacyOutline = learnerViewLoaded && learnerView === null;

  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 py-2 sm:py-4">
      <header className="border-b border-border pb-5">
        <Link
          className="text-sm font-medium text-muted-foreground hover:text-foreground"
          href="/learning"
        >
          My learning
        </Link>
        <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
              {journey.status} course
            </p>
            <h1 className="mt-2 text-2xl font-bold tracking-tight sm:text-3xl">
              {learnerView?.title ?? journey.promise}
            </h1>
          </div>
          {learnerView?.next && (
            <p className="text-sm text-muted-foreground">
              Next: {learnerView.next.title}
            </p>
          )}
        </div>
        <nav
          className="mt-5 flex gap-1 overflow-x-auto"
          aria-label="Course navigation"
        >
          {learnerTabs
            .filter((tab) => learnerView || tab.id === "learn")
            .map((tab) => {
              const Icon = tab.icon;
              const active = selectedTab === tab.id;
              return (
                <Link
                  aria-current={active ? "page" : undefined}
                  className={`inline-flex shrink-0 items-center gap-2 border-b-2 px-3 py-2 text-sm font-medium transition ${active ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:border-border hover:text-foreground"}`}
                  href={learnerTabHref(params.id, tab.id)}
                  key={tab.id}
                >
                  <Icon className="size-4" /> {tab.label}
                </Link>
              );
            })}
        </nav>
      </header>

      {selectedTab === "course" && learnerView && <CourseTab course={course} />}
      {selectedTab === "progress" && learnerView && (
        <ProgressTab progress={progress} />
      )}
      {selectedTab === "resources" && learnerView && (
        <ResourcesTab resources={resources} />
      )}
      {(selectedTab === "learn" || !learnerView) && (
        <div className="mx-auto w-full max-w-4xl p-2 sm:p-4">
          <aside className="hidden">
            <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
              {journey.status} journey
            </p>
            <h1 className="text-2xl font-bold tracking-tight">
              {journey.promise}
            </h1>
            <p className="text-sm leading-6 text-muted-foreground">
              {journey.goal.normalizedStatement}
            </p>
            <div className="border-t border-border pt-4">
              <p className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
                Original intent
              </p>
              <p className="mt-2 text-sm leading-6 text-foreground">
                {journey.goal.rawIntent}
              </p>
            </div>
            <div className="border-t border-border pt-5">
              <h2 className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
                Outcomes
              </h2>
              <ul className="mt-4 space-y-3">
                {journey.objectives.map((objective) => (
                  <li
                    key={objective.id}
                    className="flex gap-3 text-sm leading-6"
                  >
                    <Check className="mt-1 size-4 shrink-0 text-primary" />
                    <span>{objective.statement}</span>
                  </li>
                ))}
              </ul>
            </div>
          </aside>

          <main className="space-y-8">
            <section
              className="border border-primary/35 bg-card p-6 sm:p-7"
              data-testid="recommended-next"
            >
              <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
                Your next step
              </p>
              <h2 className="mt-3 text-2xl font-bold tracking-tight">
                {continueActivity?.title ?? "This course is complete"}
              </h2>
              <p className="mt-3 max-w-2xl leading-7 text-muted-foreground">
                {continueActivity && journey.recommendation?.rationale
                  ? journey.recommendation.rationale
                  : continueActivity
                    ? "Complete this focused activity, then AME will make the next course decision visible."
                    : "You have completed the published activities in this course. Review your progress or return to your courses."}
              </p>
              {continueActivity ? (
                <Button
                  type="button"
                  disabled={startingActivity !== null}
                  onClick={() => void startActivity(continueActivity.id)}
                  className="mt-6 rounded-full"
                >
                  {startingActivity === continueActivity.id
                    ? "Opening…"
                    : activeActivityId
                      ? "Resume learning"
                      : "Start next activity"}
                  <ArrowRight className="size-4" />
                </Button>
              ) : (
                <Button asChild className="mt-6 rounded-full" variant="outline">
                  <Link href={`/learning/journeys/${params.id}?tab=progress`}>
                    Review progress <ArrowRight className="size-4" />
                  </Link>
                </Button>
              )}
            </section>
            {journey.recommendation && showLegacyOutline && (
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
            {chapterHandoff && showLegacyOutline && (
              <section
                className="rounded-2xl border border-primary/30 bg-primary/5 p-5"
                data-testid="chapter-handoff"
              >
                <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                  Chapter complete
                </p>
                <h2 className="mt-2 text-xl font-semibold">
                  {chapterHandoff.completed.title} is complete.
                </h2>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  Next up: {chapterHandoff.next.title}. Continue when you are
                  ready to build on this foundation.
                </p>
              </section>
            )}
            <section className={showLegacyOutline ? "space-y-4" : "hidden"}>
              <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
                <div>
                  <h2 className="text-lg font-semibold">Your path</h2>
                  <p className="text-sm text-muted-foreground">
                    Move through each chapter, then use the next recommendation
                    to keep going.
                  </p>
                </div>
                {continueActivity && (
                  <Button
                    type="button"
                    disabled={startingActivity !== null}
                    onClick={() => void startActivity(continueActivity.id)}
                    className="shrink-0 rounded-full"
                  >
                    {startingActivity === continueActivity.id
                      ? "Opening…"
                      : activeActivityId
                        ? "Resume learning"
                        : "Continue learning"}
                    <ArrowRight className="size-4" />
                  </Button>
                )}
              </div>
              <div
                className="space-y-2 rounded-xl border border-border bg-card px-4 py-3"
                data-testid="journey-detail-progress"
              >
                <div className="flex items-center justify-between gap-3 text-xs text-muted-foreground">
                  <span>Course progress</span>
                  <span>
                    {completedPathActivities}/{pathActivities.length} complete ·{" "}
                    {pathProgressPercent}%
                  </span>
                </div>
                <div
                  className="h-2 overflow-hidden rounded-full bg-border"
                  role="progressbar"
                  aria-label="Course progress"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={pathProgressPercent}
                >
                  <div
                    className="h-full rounded-full bg-primary transition-[width]"
                    style={{ width: `${pathProgressPercent}%` }}
                  />
                </div>
              </div>
              <div className="space-y-5">
                {journey.chapters.map((chapter, index) => {
                  const completedActivities = chapter.activities.filter(
                    (activity) => activity.status === "completed",
                  ).length;
                  return (
                    <ChapterSection
                      key={chapter.id}
                      chapter={chapter}
                      index={index}
                      completedActivities={completedActivities}
                      isComplete={
                        completedActivities === chapter.activities.length
                      }
                      isCurrent={chapter.id === currentChapterId}
                    >
                      {chapter.activities.map((activity) => (
                        <ActivityRow
                          key={activity.id}
                          activity={activity}
                          startingActivity={startingActivity}
                          activeActivityId={activeActivityId}
                          onStart={startActivity}
                          onReview={setReviewActivityId}
                        />
                      ))}
                    </ChapterSection>
                  );
                })}
                {ungroupedActivities.length > 0 && (
                  <section className="rounded-2xl border border-border bg-card px-5 py-2">
                    <div className="border-b border-border py-4">
                      <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                        Additional activities
                      </p>
                      <h3 className="mt-1 text-lg font-semibold">
                        Outside the chapter sequence
                      </h3>
                    </div>
                    {ungroupedActivities.map((activity) => (
                      <ActivityRow
                        key={activity.id}
                        activity={activity}
                        startingActivity={startingActivity}
                        activeActivityId={activeActivityId}
                        onStart={startActivity}
                        onReview={setReviewActivityId}
                      />
                    ))}
                  </section>
                )}
              </div>
            </section>

            {readyActivity && !session && (
              <p className="flex items-center gap-2 text-sm text-muted-foreground">
                <Clock3 className="size-4" /> Your first activity is ready when
                you are.
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
                {assessmentLoading && (
                  <p className="mt-5 border-t border-primary/20 pt-5 text-sm text-muted-foreground">
                    Loading the activity assessment…
                  </p>
                )}
                {session.status === "in_progress" && assessment && attempt && (
                  <div className="mt-5 space-y-5 border-t border-primary/20 pt-5">
                    <div>
                      <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                        {attempt.assessmentMode === "graded"
                          ? "Graded assessment"
                          : "Practice assessment"}
                      </p>
                      <h3 className="mt-2 text-lg font-semibold">
                        Check your understanding
                      </h3>
                    </div>
                    {assessment.items.map((item, index) => (
                      <div key={item.id} className="space-y-3">
                        <p className="text-sm font-medium">
                          {index + 1}. {item.question.prompt}
                        </p>
                        {item.question.kind === "multiple_choice" ||
                        item.question.kind === "true_false" ? (
                          <div className="flex flex-wrap gap-2">
                            {item.question.options.map((option) => (
                              <Button
                                key={option.id}
                                type="button"
                                variant={
                                  assessmentResponses[item.id] === option.id
                                    ? "default"
                                    : "outline"
                                }
                                onClick={() =>
                                  setAssessmentResponses((current) => ({
                                    ...current,
                                    [item.id]: option.id,
                                  }))
                                }
                                className="rounded-full"
                              >
                                {option.text}
                              </Button>
                            ))}
                          </div>
                        ) : (
                          <textarea
                            aria-label={`Answer question ${index + 1}`}
                            value={assessmentResponses[item.id] ?? ""}
                            onChange={(event) =>
                              setAssessmentResponses((current) => ({
                                ...current,
                                [item.id]: event.target.value,
                              }))
                            }
                            rows={3}
                            className="w-full rounded-xl border border-input bg-background px-4 py-3 text-foreground outline-none focus:ring-2 focus:ring-ring"
                          />
                        )}
                      </div>
                    ))}
                  </div>
                )}
                {session.status === "in_progress" &&
                  !assessmentLoading &&
                  !assessment &&
                  isStarterContent(activeContent) && (
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
                {session.status === "in_progress" &&
                  !assessmentLoading &&
                  !assessment &&
                  !isStarterContent(activeContent) && (
                    <ActivityContentRenderer
                      content={activeContent}
                      taskId={activeActivity?.id}
                      contentVersion={activeActivity?.contentVersion}
                      rubric={activeActivity?.rubric}
                    />
                  )}
                {activeProvenance?.sourceReferences && (
                  <ActivityProvenance
                    sourceReferences={activeProvenance.sourceReferences}
                  />
                )}
                {activeActivity && (
                  <ActivityNotes
                    activityId={activeActivity.id}
                    contentVersion={activeActivity.contentVersion}
                    journeyId={String(params.id)}
                  />
                )}
                {activeActivity && journey.objectives[0] && (
                  <ActivityVariants
                    activityId={activeActivity.id}
                    objectiveId={journey.objectives[0].id}
                  />
                )}
                {session.status === "in_progress" && assessment && attempt ? (
                  <Button
                    type="button"
                    disabled={finishing || attempt.status !== "in_progress"}
                    onClick={() => {
                      setFinishing(true);
                      setError(null);
                      void finishSession().finally(() => setFinishing(false));
                    }}
                    className="mt-5 rounded-full"
                  >
                    {finishing ? "Grading assessment…" : "Submit assessment"}
                    <Check className="size-4" />
                  </Button>
                ) : session.status === "in_progress" ? (
                  <Button
                    type="button"
                    disabled={finishing}
                    onClick={() => {
                      setFinishing(true);
                      setError(null);
                      void finishSession().finally(() => setFinishing(false));
                    }}
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
                {attempt && <AssessmentResultFeedback attempt={attempt} />}
              </section>
            )}
            {reviewActivity && !session && (
              <section className="border border-border bg-card p-6 shadow-sm">
                <div className="flex flex-wrap items-start justify-between gap-4">
                  <div>
                    <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                      Completed activity
                    </p>
                    <h2 className="mt-2 text-xl font-semibold">
                      {reviewActivity.title}
                    </h2>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => setReviewActivityId(null)}
                    className="rounded-full"
                  >
                    Close review
                  </Button>
                </div>
                <p className="mt-2 text-sm text-muted-foreground">
                  This is the published content you completed. Reviewing it does
                  not start another session or change your progress.
                </p>
                {isStarterContent(reviewContent) ? (
                  <div className="mt-5 space-y-5 border-t border-primary/20 pt-5">
                    {reviewContent.context && (
                      <p className="rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6 text-foreground">
                        {reviewContent.context}
                      </p>
                    )}
                    <p className="text-sm font-medium">
                      {reviewContent.instructions}
                    </p>
                    <ol className="space-y-3">
                      {reviewContent.questions.map((question, index) => (
                        <li key={question.id} className="text-sm leading-6">
                          <span className="font-medium">
                            {index + 1}. {question.prompt}
                          </span>
                          {question.options && (
                            <p className="mt-1 text-muted-foreground">
                              Options: {question.options.join(", ")}
                            </p>
                          )}
                        </li>
                      ))}
                    </ol>
                  </div>
                ) : (
                  <ActivityContentRenderer
                    content={reviewContent}
                    contentVersion={reviewActivity.contentVersion}
                    rubric={reviewActivity.rubric}
                  />
                )}
                {reviewPayload?.contentProvenance?.sourceReferences && (
                  <ActivityProvenance
                    sourceReferences={
                      reviewPayload.contentProvenance.sourceReferences
                    }
                  />
                )}
              </section>
            )}
            {deepDive && (
              <section className="rounded-2xl border border-border bg-card p-6 shadow-sm">
                <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                  Source-backed explanation
                </p>
                <h2 className="mt-2 text-2xl font-bold tracking-tight">
                  {deepDive.title}
                </h2>
                <p className="mt-4 whitespace-pre-wrap leading-7 text-muted-foreground">
                  {deepDive.body}
                </p>
                <div className="mt-5 grid gap-4 border-t border-border pt-5 sm:grid-cols-2">
                  <div>
                    <p className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
                      Example
                    </p>
                    <p className="mt-2 text-sm leading-6">{deepDive.example}</p>
                  </div>
                  <div>
                    <p className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
                      Try this
                    </p>
                    <p className="mt-2 text-sm leading-6">
                      {deepDive.applicationTask}
                    </p>
                  </div>
                </div>
                {deepDive.sourceReferences.length > 0 && (
                  <div className="mt-5 border-t border-border pt-5">
                    <p className="text-xs font-bold uppercase tracking-[0.12em] text-muted-foreground">
                      Sources
                    </p>
                    <ul className="mt-2 space-y-1 text-sm">
                      {deepDive.sourceReferences.map((source) => (
                        <li key={source}>
                          <a
                            href={source}
                            target="_blank"
                            rel="noreferrer"
                            className="text-primary underline underline-offset-4"
                          >
                            {source}
                          </a>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </section>
            )}
            {attemptHistory.length > 0 && (
              <section className="rounded-2xl border border-border bg-card p-6 shadow-sm">
                <div className="flex items-center gap-3">
                  <History className="size-5 text-primary" />
                  <div>
                    <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                      Assessment history
                    </p>
                    <h2 className="mt-1 text-xl font-bold">
                      Your saved attempts
                    </h2>
                  </div>
                </div>
                <ul className="mt-5 space-y-2">
                  {attemptHistory.map((historyAttempt) => (
                    <li
                      key={historyAttempt.id}
                      className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border px-4 py-3 text-sm"
                    >
                      <span className="text-muted-foreground">
                        {historyAttempt.createdAt.slice(0, 10)}
                      </span>
                      <span className="font-medium">
                        {historyAttempt.status === "submitted"
                          ? "Pending review"
                          : historyAttempt.status === "graded" &&
                              historyAttempt.score !== null &&
                              historyAttempt.maxPoints
                            ? `${historyAttempt.score}/${historyAttempt.maxPoints} points`
                            : historyAttempt.status.replaceAll("_", " ")}
                      </span>
                    </li>
                  ))}
                </ul>
              </section>
            )}
            {error && (
              <p role="alert" className="text-sm text-destructive">
                {error}
              </p>
            )}
          </main>
        </div>
      )}
    </div>
  );
}
