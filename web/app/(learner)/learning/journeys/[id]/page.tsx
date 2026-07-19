"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { ArrowRight, Check, Clock3, History } from "lucide-react";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";
import { Button } from "@/components/ui/button";

type Journey = components["schemas"]["LearningJourneyResponse"];
type LearningSession = components["schemas"]["LearningSessionResponse"];
type Assessment = components["schemas"]["AssessmentResponse"];
type AssessmentItem = components["schemas"]["AssessmentItemResponse"];
type Attempt = components["schemas"]["AttemptResponse"];
type DeepDive = components["schemas"]["DeepDiveResponse"];
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
type ExplanationContent = {
  type: "explanation";
  heading: string;
  body: string;
  key_points: string[];
};
type WorkedExampleContent = {
  type: "worked_example";
  heading: string;
  prompt: string;
  steps: string[];
  reflection: string;
};
type ActivityContent =
  | StarterContent
  | ExplanationContent
  | WorkedExampleContent;

export default function LearningJourneyPage() {
  const params = useParams<{ id: string }>();
  const [journey, setJourney] = useState<Journey | null>(null);
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

  async function loadAttemptHistory() {
    const result = await api.GET(
      "/api/v1/learning/journeys/{journey_id}/attempts",
      { params: { path: { journey_id: params.id } } },
    );
    if (!result.response.ok || !result.data) {
      throw new Error("Could not load assessment history");
    }
    setAttemptHistory(result.data);
  }

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
          if (completedActivity) await loadDeepDive(completedActivity.id);
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
      setAssessmentResponses({});
      setAssessment(null);
      setAttempt(null);
      setDeepDive(null);
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
      if (refreshed.response.ok && refreshed.data) setJourney(refreshed.data);
      await loadAttemptHistory();
      await loadDeepDive(session.activityId);
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
    ? ((activeActivity.payload as unknown as { content?: ActivityContent })
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
            {session.status === "in_progress" &&
              !assessmentLoading &&
              !assessment &&
              activeContent?.type === "explanation" && (
                <div className="mt-5 space-y-5 border-t border-primary/20 pt-5">
                  <div>
                    <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                      Explanation
                    </p>
                    <h3 className="mt-2 text-lg font-semibold">
                      {activeContent.heading}
                    </h3>
                  </div>
                  <p className="leading-7 text-foreground">
                    {activeContent.body}
                  </p>
                  <ul className="space-y-2 rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6">
                    {activeContent.key_points.map((point) => (
                      <li key={point} className="flex gap-2">
                        <Check className="mt-1 size-4 shrink-0 text-primary" />
                        <span>{point}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            {session.status === "in_progress" &&
              !assessmentLoading &&
              !assessment &&
              activeContent?.type === "worked_example" && (
                <div className="mt-5 space-y-5 border-t border-primary/20 pt-5">
                  <div>
                    <p className="text-xs font-bold uppercase tracking-[0.12em] text-primary">
                      Worked example
                    </p>
                    <h3 className="mt-2 text-lg font-semibold">
                      {activeContent.heading}
                    </h3>
                  </div>
                  <p className="rounded-xl border border-primary/20 bg-background/70 p-4 text-sm leading-6 text-foreground">
                    {activeContent.prompt}
                  </p>
                  <ol className="space-y-2 text-sm leading-6">
                    {activeContent.steps.map((step, index) => (
                      <li key={step} className="flex gap-3">
                        <span className="font-mono text-primary">
                          {index + 1}.
                        </span>
                        <span>{step}</span>
                      </li>
                    ))}
                  </ol>
                  <p className="border-t border-primary/20 pt-4 text-sm leading-6 text-muted-foreground">
                    {activeContent.reflection}
                  </p>
                </div>
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
            {(attempt?.status === "graded" ||
              attempt?.status === "submitted") && (
              <div className="mt-4 space-y-3 border-t border-primary/20 pt-4">
                <p className="text-sm font-medium text-primary">
                  {attempt.status === "submitted"
                    ? "Assessment submitted · pending review"
                    : `Assessment complete · ${Math.round((attempt.score ?? 0) * 100)}%`}
                </p>
                {attempt.items.length > 0 && (
                  <ul className="space-y-2 text-sm">
                    {attempt.items.map((item) => (
                      <li
                        key={item.assessmentItemId}
                        className="flex items-center justify-between gap-4 rounded-lg bg-background/60 px-3 py-2"
                      >
                        <span>Question result</span>
                        <span
                          className={
                            item.evaluationStatus === "correct"
                              ? "font-semibold text-primary"
                              : "font-semibold text-destructive"
                          }
                        >
                          {item.evaluationStatus.replaceAll("_", " ")}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
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
                <h2 className="mt-1 text-xl font-bold">Your saved attempts</h2>
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
  );
}
