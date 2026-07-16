"use client";

import { use, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { CheckCircle2, ChevronLeft, X, XCircle } from "lucide-react";
import { formatDate } from "@/utils/format";
import { api } from "@/api/client";
import { MarkdownView } from "@/components/MarkdownView";
import { HighlightedCode } from "@/components/HighlightedCode";
import { Button } from "@/components/ui/button";

type Answer = {
  qid: string;
  correct: boolean;
  points: number;
  max: number;
  type: string;
  prompt: string;
  given: string;
  correctAnswer?: string;
  gradeStatus: string;
  explanation?: string;
};
type Result = {
  id: string;
  assessment_title: string | null;
  score: number;
  total: number;
  feedback?: string;
  answers: Answer[];
  attempt_number?: number | null;
  total_attempts?: number | null;
};
type History = {
  id: string;
  pointsAwarded?: number;
  maxPoints?: number;
  startedAt: string;
  finishedAt?: string;
  attemptNumber: number;
};
type Deepen = {
  question: {
    id: string;
    prompt: string;
    kind: string;
    explanation?: string | null;
    deepDive?: string | null;
    source?: string | null;
    tags: string[];
  };
  related: Array<{ id: string; prompt: string; kind: string; tags: string[] }>;
};

export default function ResultsPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const router = useRouter();
  const [data, setData] = useState<Result | null>(null);
  const [history, setHistory] = useState<History[]>([]);
  const [assessmentId, setAssessmentId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [retaking, setRetaking] = useState(false);
  const [deepening, setDeepening] = useState<string | null>(null);
  const [deep, setDeep] = useState<Deepen | null>(null);
  const [deepPrevious, setDeepPrevious] = useState<Deepen | null>(null);
  const [requested, setRequested] = useState<Record<string, boolean>>({});

  useEffect(() => {
    (async () => {
      const { data: response } = await (api as any).GET("/v1/sessions/{id}", {
        params: { path: { id } },
      });
      const session = response?.session;
      if (!session) return;
      setAssessmentId(session.assessment_id ?? null);
      const attempts = new Map(
        (response.attempts ?? []).map((attempt: any) => [
          attempt.question_id,
          attempt,
        ]),
      );
      const answers = (response.questions ?? []).map((question: any) => {
        const attempt = attempts.get(question.questionId) as any;
        const responseBody = attempt?.response ?? {};
        const correct = attempt?.correct_answer ?? {};
        const presentation = attempt?.presentation ?? {};
        let given = "";
        if ("selected_position" in responseBody) {
          const position = Number(responseBody.selected_position);
          const canonical = presentation.option_order?.[position] ?? position;
          given = `${String.fromCharCode(65 + position)}: ${question.options?.[canonical]?.text ?? ""}`;
        } else if ("answer" in responseBody)
          given =
            typeof responseBody.answer === "boolean"
              ? responseBody.answer
                ? "True"
                : "False"
              : String(responseBody.answer ?? "");
        else if ("body" in responseBody)
          given = String(responseBody.body ?? "");
        else if ("source" in responseBody)
          given = String(responseBody.source ?? "");
        let correctAnswer = "";
        if (!attempt?.is_correct) {
          if ("correct_index" in correct)
            correctAnswer = String.fromCharCode(
              65 + Number(correct.correct_index),
            );
          else if ("correct" in correct)
            correctAnswer = correct.correct ? "True" : "False";
          else if ("accepted" in correct)
            correctAnswer = correct.accepted.join(", ");
          else if ("exemplar" in correct)
            correctAnswer = String(correct.exemplar);
        }
        return {
          qid: question.questionId,
          correct: attempt?.is_correct ?? false,
          points: Math.round((attempt?.score ?? 0) * question.points),
          max: question.points,
          type: question.kind,
          prompt: question.prompt,
          given,
          correctAnswer,
          gradeStatus: attempt?.grade_status ?? "ungraded",
          explanation: question.explanation ?? undefined,
        };
      });
      setData({
        id: session.id,
        assessment_title: session.assessment_title ?? null,
        score: session.result?.points_awarded ?? 0,
        total: session.result?.max_points ?? 0,
        feedback: session.result?.feedback,
        answers,
      });
      const apiUrl =
        process.env.NEXT_PUBLIC_API_URL ??
        `http://${window.location.hostname}:28080`;
      if (session.assessment_id) {
        const response = await fetch(
          `${apiUrl}/v1/sessions?assessmentId=${session.assessment_id}`,
          { credentials: "include" },
        );
        const historyResponse = response.ok ? await response.json() : null;
        setHistory(historyResponse?.sessions ?? []);
        const current = (historyResponse?.sessions ?? []).find(
          (item: History) => item.id === id,
        );
        if (current)
          setData((previous) =>
            previous
              ? {
                  ...previous,
                  attempt_number: current.attemptNumber,
                  total_attempts: current.totalAttempts,
                }
              : previous,
          );
      }
    })()
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [id]);

  async function openDeepen(questionId: string) {
    setDeepening(questionId);
    try {
      const { data: result } = await (api as any).GET(
        "/v1/questions/{id}/deepen",
        {
          params: { path: { id: questionId }, query: { exclude: questionId } },
        },
      );
      setDeep(result ?? null);
    } catch (error) {
      console.error(error);
    } finally {
      setDeepening(null);
    }
  }
  async function requestDeepDive(questionId: string) {
    await (api as any).POST("/v1/deep-dives", {
      body: {
        questionId,
        sourceSessionId: id,
        reason: "Marked from results review",
      },
    });
    setRequested((value) => ({ ...value, [questionId]: true }));
  }
  async function retake() {
    if (!assessmentId) return;
    setRetaking(true);
    const { data: next } = await api.POST("/v1/sessions", {
      body: { assessmentId },
    });
    if ((next as any)?.sessionId)
      router.push(`/sessions/${(next as any).sessionId}`);
    else setRetaking(false);
  }

  if (loading)
    return (
      <div className="grid min-h-[60vh] place-items-center text-sm text-muted-foreground">
        Loading results…
      </div>
    );
  if (!data)
    return (
      <div className="mx-auto max-w-3xl px-6 py-12 text-sm text-muted-foreground">
        Results not found.
      </div>
    );
  const percent = data.total ? Math.round((data.score / data.total) * 100) : 0;
  const passed = percent >= 70;
  const deepTags = deep
    ? [
        ...new Set([
          ...(deep.question.tags ?? []),
          ...(deep.related ?? []).flatMap((item) => item.tags ?? []),
        ]),
      ]
    : [];
  return (
    <main className="mx-auto max-w-4xl px-6 py-10 sm:px-10">
      <button
        type="button"
        onClick={() => router.push("/explore")}
        className="mb-6 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
      >
        <ChevronLeft className="size-4" /> Back to Explore
      </button>
      <header className="mb-8">
        <p className="mb-1 font-mono text-xs uppercase tracking-widest text-muted-foreground">
          Assessment results
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">
          Quiz Results — {data.assessment_title || "Results"}
        </h1>
        {data.attempt_number && (
          <p className="mt-1 text-sm text-muted-foreground">
            Attempt {data.attempt_number} of {data.total_attempts}
          </p>
        )}
      </header>
      <section className="mb-8 flex flex-wrap items-center justify-between gap-6 rounded-xl border border-border bg-card p-6">
        <div>
          <p className="text-5xl font-semibold tracking-tight">{percent}%</p>
          <p className="mt-1 text-sm text-muted-foreground">
            {data.score} / {data.total} points
          </p>
          {data.feedback && (
            <p className="mt-3 text-sm italic text-muted-foreground">
              {data.feedback}
            </p>
          )}
        </div>
        <div
          className={`inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-sm font-medium ${passed ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300" : "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300"}`}
        >
          {passed ? (
            <CheckCircle2 className="size-4" />
          ) : (
            <XCircle className="size-4" />
          )}
          {passed ? "Passed" : "Not passed"}
        </div>
      </section>
      <h2 className="mb-3 text-lg font-semibold">Answer review</h2>
      <div className="space-y-3">
        {data.answers.map((answer, index) => (
          <article
            key={answer.qid}
            data-testid="answer-card"
            className={`rounded-xl border bg-card p-5 ${answer.correct ? "border-emerald-500/30" : answer.gradeStatus === "pending_manual" ? "border-amber-500/30" : "border-destructive/30"}`}
          >
            <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-start">
              <h3 className="font-medium">
                Q{index + 1}. {answer.prompt}
              </h3>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => openDeepen(answer.qid)}
                  disabled={deepening === answer.qid}
                >
                  Dive deeper
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => requestDeepDive(answer.qid)}
                  disabled={requested[answer.qid]}
                >
                  {requested[answer.qid] ? "Requested" : "Request deep dive"}
                </Button>
                <span className="rounded-full border px-2 py-1 text-xs">
                  {answer.gradeStatus === "pending_manual"
                    ? "Pending review"
                    : `${answer.points}/${answer.max} pts`}
                </span>
              </div>
            </div>
            {answer.gradeStatus === "pending_manual" ? (
              <p className="mt-4 text-sm text-muted-foreground">
                Your submission: {answer.given || "No answer submitted"}.
                Awaiting manual review.
              </p>
            ) : answer.correct ? (
              <p className="mt-4 border-l-2 border-emerald-500 pl-3 text-sm text-emerald-700 dark:text-emerald-300">
                Correct: {answer.given || "—"}
              </p>
            ) : (
              <div className="mt-4 grid gap-4 rounded-lg border border-dashed border-destructive/30 bg-destructive/5 p-4 sm:grid-cols-2">
                <div>
                  <p className="mb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    Your answer
                  </p>
                  {answer.type === "code" && answer.given ? (
                    <HighlightedCode code={answer.given} language="python" />
                  ) : (
                    <p className="text-sm font-medium text-destructive">
                      {answer.given || "—"}
                    </p>
                  )}
                </div>
                {answer.correctAnswer && (
                  <div className="sm:border-l sm:border-border sm:pl-4">
                    <p className="mb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                      Correct solution
                    </p>
                    <p className="text-sm font-medium text-emerald-700 dark:text-emerald-300">
                      {answer.correctAnswer}
                    </p>
                  </div>
                )}
              </div>
            )}
            {answer.explanation && (
              <div className="mt-4 border-t border-border pt-3 text-sm text-muted-foreground">
                <span className="font-semibold text-primary">
                  Explanation:{" "}
                </span>
                {answer.explanation}
              </div>
            )}
          </article>
        ))}
      </div>
      {history.length > 1 && (
        <section className="mt-10">
          <h2 className="mb-3 text-lg font-semibold">All attempts</h2>
          <div className="space-y-2">
            {history.map((attempt) => (
              <button
                key={attempt.id}
                type="button"
                onClick={() =>
                  attempt.id !== id &&
                  router.push(`/sessions/${attempt.id}/results`)
                }
                className={`flex w-full items-center justify-between rounded-lg border p-4 text-left ${attempt.id === id ? "border-primary bg-primary/5" : "border-border hover:bg-muted/40"}`}
              >
                <span>
                  <span className="block text-sm font-medium">
                    Attempt {attempt.attemptNumber}
                    {attempt.id === id && " (this one)"}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {formatDate(attempt.finishedAt ?? attempt.startedAt)}
                  </span>
                </span>
                <span className="rounded-full border px-2 py-1 text-xs">
                  {attempt.maxPoints
                    ? `${Math.round(((attempt.pointsAwarded ?? 0) / attempt.maxPoints) * 100)}%`
                    : "—"}
                </span>
              </button>
            ))}
          </div>
        </section>
      )}
      <div className="mt-8 flex justify-center">
        <Button onClick={retake} disabled={retaking}>
          {retaking ? "Starting…" : "Take again"}
        </Button>
      </div>
      {deep && (
        <div
          className="fixed inset-0 z-50 flex justify-end bg-black/50"
          role="dialog"
          aria-modal="true"
        >
          <aside className="h-full w-full max-w-xl overflow-y-auto border-l border-border bg-background p-6 shadow-xl">
            <div className="mb-6 flex items-center justify-between">
              <div className="flex items-center gap-2">
                {deepPrevious && (
                  <button
                    type="button"
                    onClick={() => {
                      setDeep(deepPrevious);
                      setDeepPrevious(null);
                    }}
                    className="rounded-md px-2 py-1 text-sm text-muted-foreground hover:bg-muted hover:text-foreground"
                  >
                    Back
                  </button>
                )}
                <h2 className="text-lg font-semibold">Dive Deeper</h2>
              </div>
              <button
                type="button"
                aria-label="Close"
                onClick={() => {
                  setDeep(null);
                  setDeepPrevious(null);
                }}
                className="rounded-md p-1 hover:bg-muted"
              >
                <X data-testid="CloseIcon" className="size-4" />
              </button>
            </div>
            <h3 className="text-base font-medium">{deep.question.prompt}</h3>
            {deepTags.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-2">
                {deepTags.map((tag) => (
                  <span
                    key={tag}
                    className="rounded-full border border-sky-500/30 bg-sky-500/10 px-2 py-1 text-xs text-sky-700 dark:text-sky-300"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            )}
            {deep.question.deepDive ? (
              <div className="mt-6">
                <h3 className="mb-2 text-xs font-semibold uppercase tracking-widest text-muted-foreground">
                  Deep Dive Study Notes
                </h3>
                <div className="prose prose-sm max-w-none dark:prose-invert">
                  <MarkdownView content={deep.question.deepDive} />
                </div>
              </div>
            ) : (
              <p className="mt-6 text-sm text-muted-foreground">
                No study notes have been published for this question yet.
              </p>
            )}
            {deep.question.source && (
              <a
                className="mt-6 block text-sm text-primary underline"
                href={deep.question.source}
                target="_blank"
                rel="noreferrer"
              >
                Visit External Source
              </a>
            )}
            {deep.related?.length > 0 && (
              <div className="mt-8 border-t border-border pt-6">
                <h3 className="mb-3 text-sm font-semibold">
                  Related questions
                </h3>
                <div className="space-y-2">
                  {deep.related.map((item) => (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => {
                        setDeepPrevious(deep);
                        openDeepen(item.id);
                      }}
                      className="block w-full rounded-lg border border-border p-3 text-left text-sm hover:bg-muted/40"
                    >
                      {item.prompt}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </aside>
        </div>
      )}
    </main>
  );
}
