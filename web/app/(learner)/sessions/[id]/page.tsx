"use client";

import { use, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import {
  ArrowLeft,
  ArrowRight,
  Flag,
  LoaderCircle,
  LogOut,
  X,
} from "lucide-react";
import { api } from "@/api/client";
import { McqRenderer } from "@/components/question/McqRenderer";
import { ShortRenderer } from "@/components/question/ShortRenderer";
import { EssayRenderer } from "@/components/question/EssayRenderer";
import { ClozeRenderer } from "@/components/question/ClozeRenderer";
import { TfRenderer } from "@/components/question/TfRenderer";
import { CodeRenderer } from "@/components/question/CodeRenderer";
import { Button } from "@/components/ui/button";

interface SessionQuestion {
  questionId: string;
  kind: string;
  prompt: string;
  points: number;
  codeSnippet?: { language: string; body: string };
  language?: string;
  options?: Array<{ text: string }>;
  optionOrder?: number[];
}
interface SessionData {
  session: {
    id: string;
    status: string;
    deadline_at?: string;
    assessment_title?: string;
  };
  questions: SessionQuestion[];
}
type Answer = string | number | boolean | null;

const QUESTION_TYPES: Record<string, string> = {
  mc: "Multiple choice",
  mcq: "Multiple choice",
  tf: "True / false",
  short: "Short answer",
  free_text: "Short answer",
  essay: "Essay",
  code: "Code",
  cloze: "Fill in the blank",
};

export default function ActiveQuizPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const router = useRouter();
  const [session, setSession] = useState<SessionData | null>(null);
  const [idx, setIdx] = useState(0);
  const [answers, setAnswers] = useState<Record<string, Answer>>({});
  const [flagged, setFlagged] = useState<Record<string, boolean>>({});
  const [timeLeft, setTimeLeft] = useState<number | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [dialog, setDialog] = useState<"quit" | "submit" | null>(null);
  const [timesSpent, setTimesSpent] = useState<Record<string, number>>({});
  const [questionStart, setQuestionStart] = useState(Date.now());
  const [lastIdx, setLastIdx] = useState(0);

  useEffect(() => {
    api
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      .then(({ data }: { data?: SessionData }) => {
        if (!data) return;
        setSession(data);
        if (data.session.deadline_at) {
          setTimeLeft(
            Math.max(
              0,
              Math.floor(
                (new Date(data.session.deadline_at).getTime() - Date.now()) /
                  1000,
              ),
            ),
          );
        } else if (data.questions.length) {
          setTimeLeft(Math.max(20, data.questions.length * 2) * 60);
        }
      })
      .catch(console.error);
  }, [id]);

  const questions = useMemo(() => session?.questions ?? [], [session]);
  const answered = Object.keys(answers).filter(
    (key) => answers[key] !== null && answers[key] !== "",
  ).length;
  const current = questions[idx];

  useEffect(() => {
    const previous = questions[lastIdx];
    if (previous) {
      setTimesSpent((value) => ({
        ...value,
        [previous.questionId]:
          (value[previous.questionId] ?? 0) + Date.now() - questionStart,
      }));
    }
    setQuestionStart(Date.now());
    setLastIdx(idx);
    // The timer intentionally starts when the question index changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [idx, questions.length]);

  const handleFinish = useCallback(async () => {
    if (finishing) return;
    setFinishing(true);
    try {
      const finalTimes = { ...timesSpent };
      if (current) {
        finalTimes[current.questionId] =
          (finalTimes[current.questionId] ?? 0) + Date.now() - questionStart;
      }
      for (const [questionId, value] of Object.entries(answers)) {
        const question = session?.questions.find(
          (item) => item.questionId === questionId,
        );
        if (!question) continue;
        let response: Record<string, unknown>;
        if (question.kind === "mc" || question.kind === "mcq") {
          const order =
            question.optionOrder ?? question.options?.map((_, i) => i) ?? [];
          response = {
            selected_position:
              order.indexOf(value as number) >= 0
                ? order.indexOf(value as number)
                : value,
          };
        } else if (question.kind === "tf") {
          response = { answer: value as boolean };
        } else if (question.kind === "essay") {
          const body = String(value ?? "");
          response = {
            body,
            word_count: body.trim().split(/\s+/).filter(Boolean).length,
          };
        } else if (question.kind === "code") {
          response = {
            source: String(value ?? ""),
            language:
              question.language ?? question.codeSnippet?.language ?? "python",
          };
        } else {
          response = { answer: String(value ?? "") };
        }
        await (api as any).POST("/v1/sessions/{id}/answer", {
          params: { path: { id } },
          body: {
            questionId,
            response,
            time_to_answer_ms: Math.round(finalTimes[questionId] ?? 0),
          },
        });
      }
      await (api as any).POST("/v1/sessions/{id}/finish", {
        params: { path: { id } },
      });
      router.push(`/sessions/${id}/results`);
    } catch (error) {
      console.error(error);
      setFinishing(false);
    }
  }, [
    answers,
    current,
    finishing,
    id,
    questionStart,
    router,
    session,
    timesSpent,
  ]);

  const finishRef = useRef(handleFinish);
  useEffect(() => {
    finishRef.current = handleFinish;
  }, [handleFinish]);

  useEffect(() => {
    if (timeLeft === null || timeLeft <= 0) return;
    const timer = setInterval(() => {
      setTimeLeft((value) => {
        if (value === null || value <= 1) {
          clearInterval(timer);
          finishRef.current();
          return 0;
        }
        return value - 1;
      });
    }, 1000);
    return () => clearInterval(timer);
  }, [timeLeft !== null && timeLeft > 0]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (!session) return;
      const target = event.target as HTMLElement | null;
      const typing =
        ["INPUT", "TEXTAREA"].includes(target?.tagName ?? "") ||
        target?.isContentEditable;
      if (
        event.key === "Enter" &&
        !event.shiftKey &&
        !(target?.tagName === "TEXTAREA" && !event.ctrlKey && !event.metaKey)
      ) {
        event.preventDefault();
        if (idx < questions.length - 1) setIdx((value) => value + 1);
        else setDialog("submit");
      } else if (!typing && event.key.toLowerCase() === "f" && current) {
        setFlagged((value) => ({
          ...value,
          [current.questionId]: !value[current.questionId],
        }));
      } else if (!typing && /^[1-9]$/.test(event.key) && current) {
        const option = Number(event.key) - 1;
        if (current.kind === "tf" && option < 2) {
          setAnswers((value) => ({
            ...value,
            [current.questionId]: option === 0,
          }));
        } else if (current.options && option < current.options.length) {
          const order = current.optionOrder ?? current.options.map((_, i) => i);
          setAnswers((value) => ({
            ...value,
            [current.questionId]: order[option],
          }));
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, idx, questions.length, session]);

  if (!session) {
    return (
      <div className="grid min-h-screen place-items-center">
        <LoaderCircle className="size-7 animate-spin text-primary" />
      </div>
    );
  }

  const minutes = timeLeft === null ? null : Math.floor(timeLeft / 60);
  const seconds =
    timeLeft === null ? null : String(timeLeft % 60).padStart(2, "0");
  const setAnswer = (value: Answer) => {
    if (current)
      setAnswers((previous) => ({ ...previous, [current.questionId]: value }));
  };

  return (
    <div className="flex min-h-screen flex-col bg-background">
      <header className="sticky top-0 z-20 flex flex-wrap items-center justify-between gap-3 border-b border-border bg-background/95 px-4 py-3 backdrop-blur sm:px-8">
        <div className="min-w-0">
          <p className="truncate font-medium">
            {session.session.assessment_title || "Quiz"}
          </p>
          <p className="text-xs text-muted-foreground">
            {answered}/{questions.length} answered
            {minutes !== null && ` · ${minutes}:${seconds}`}
          </p>
        </div>
        <div className="flex gap-2">
          <Button onClick={() => setDialog("submit")} disabled={finishing}>
            {finishing ? "Submitting…" : "Submit"}
          </Button>
          <Button variant="outline" onClick={() => setDialog("quit")}>
            <LogOut /> Quit
          </Button>
        </div>
      </header>
      <div className="h-1 bg-muted">
        <div
          className="h-full bg-primary transition-all"
          style={{
            width: `${questions.length ? (answered / questions.length) * 100 : 0}%`,
          }}
        />
      </div>

      <main className="flex min-h-0 flex-1">
        <section className="min-w-0 flex-1 overflow-y-auto p-5 sm:p-10">
          <div className="mx-auto max-w-3xl">
            {current && (
              <>
                <div className="mb-3 flex items-center justify-between gap-3 text-xs text-muted-foreground">
                  <span>
                    Question {idx + 1} of {questions.length} ·{" "}
                    {QUESTION_TYPES[current.kind] ?? current.kind} ·{" "}
                    {current.points} pt{current.points === 1 ? "" : "s"}
                  </span>
                  <button
                    type="button"
                    className={`inline-flex items-center gap-1 rounded-md px-2 py-1 transition hover:bg-muted ${flagged[current.questionId] ? "text-amber-600 dark:text-amber-300" : ""}`}
                    onClick={() =>
                      setFlagged((value) => ({
                        ...value,
                        [current.questionId]: !value[current.questionId],
                      }))
                    }
                  >
                    <Flag className="size-3.5" />{" "}
                    {flagged[current.questionId]
                      ? "Flagged"
                      : "Flag for review"}
                  </button>
                </div>
                <h1 className="mb-8 text-2xl font-semibold tracking-tight">
                  {current.prompt}
                </h1>
                <div data-testid="question-input">
                  <QuestionInput
                    question={current}
                    value={answers[current.questionId] ?? null}
                    onChange={setAnswer}
                    disabled={false}
                  />
                </div>
              </>
            )}
          </div>
        </section>

        <aside className="hidden w-72 shrink-0 border-l border-border bg-card/30 p-6 md:block">
          <h2 className="mb-4 text-sm font-semibold">Questions</h2>
          <div className="grid grid-cols-5 gap-2">
            {questions.map((question, questionIndex) => {
              const isAnswered =
                answers[question.questionId] !== undefined &&
                answers[question.questionId] !== null &&
                answers[question.questionId] !== "";
              return (
                <button
                  key={question.questionId}
                  type="button"
                  onClick={() => setIdx(questionIndex)}
                  className={`relative aspect-square rounded-lg border-2 text-sm font-medium transition hover:bg-muted ${questionIndex === idx ? "border-primary bg-primary/10 text-primary" : isAnswered ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300" : "border-border"}`}
                >
                  {questionIndex + 1}
                  {flagged[question.questionId] && (
                    <Flag className="absolute -right-1 -top-1 size-3.5 rounded-full bg-background text-amber-500" />
                  )}
                </button>
              );
            })}
          </div>
        </aside>
      </main>

      <footer className="flex items-center justify-between border-t border-border bg-background px-4 py-3 sm:px-8">
        <Button
          variant="outline"
          disabled={idx === 0}
          onClick={() => setIdx((value) => Math.max(0, value - 1))}
        >
          <ArrowLeft /> Previous
        </Button>
        <span className="hidden text-xs text-muted-foreground sm:block">
          Press Enter to continue
        </span>
        {idx < questions.length - 1 ? (
          <Button
            onClick={() =>
              setIdx((value) => Math.min(questions.length - 1, value + 1))
            }
          >
            Next <ArrowRight />
          </Button>
        ) : (
          <Button onClick={() => setDialog("submit")} disabled={finishing}>
            Submit
          </Button>
        )}
      </footer>

      {dialog && (
        <ConfirmDialog
          kind={dialog}
          unanswered={questions.length - answered}
          flagged={Object.values(flagged).filter(Boolean).length}
          onClose={() => setDialog(null)}
          onConfirm={
            dialog === "quit"
              ? async () => {
                  await (api as any).PATCH("/v1/sessions/{id}", {
                    params: { path: { id } },
                    body: { status: "abandoned" },
                  });
                  router.push("/explore");
                }
              : handleFinish
          }
        />
      )}
    </div>
  );
}

function ConfirmDialog({
  kind,
  unanswered,
  flagged,
  onClose,
  onConfirm,
}: {
  kind: "quit" | "submit";
  unanswered: number;
  flagged: number;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/50 p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
    >
      <div className="w-full max-w-md rounded-xl border border-border bg-background p-6 shadow-xl">
        <div className="flex items-start justify-between gap-4">
          <h2 id="confirm-title" className="text-lg font-semibold">
            {kind === "quit" ? "Quit Assessment?" : "Submit Assessment?"}
          </h2>
          <button
            type="button"
            aria-label="Close"
            onClick={onClose}
            className="rounded-md p-1 text-muted-foreground hover:bg-muted"
          >
            <X className="size-4" />
          </button>
        </div>
        <p className="mt-3 text-sm leading-6 text-muted-foreground">
          {kind === "quit" ? (
            "Are you sure you want to quit? Your progress will not be saved and you cannot resume this assessment later."
          ) : unanswered || flagged ? (
            <>
              You have {unanswered} unanswered question
              {unanswered === 1 ? "" : "s"}
              {flagged ? ` and ${flagged} flagged for review` : ""}. Are you
              sure you want to submit?
            </>
          ) : (
            "Are you sure you want to submit and finish this assessment?"
          )}
        </p>
        <div className="mt-6 flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant={kind === "quit" ? "destructive" : "default"}
            onClick={onConfirm}
          >
            {kind === "quit" ? "Quit" : "Submit"}
          </Button>
        </div>
      </div>
    </div>
  );
}

function QuestionInput({
  question,
  value,
  onChange,
  disabled,
}: {
  question: SessionQuestion;
  value: Answer;
  onChange: (value: Answer) => void;
  disabled: boolean;
}) {
  if ((question.kind === "mc" || question.kind === "mcq") && question.options) {
    const order =
      question.optionOrder ?? question.options.map((_, index) => index);
    return (
      <McqRenderer
        options={order.map((index) => ({
          text: question.options![index].text,
          index,
        }))}
        value={typeof value === "number" ? value : null}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  if (question.kind === "tf")
    return (
      <TfRenderer
        value={typeof value === "boolean" ? value : null}
        onChange={onChange}
        disabled={disabled}
      />
    );
  if (question.kind === "cloze")
    return (
      <ClozeRenderer
        prompt={question.prompt}
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  if (question.kind === "code")
    return (
      <CodeRenderer
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
        language={question.language ?? question.codeSnippet?.language}
        starter={question.codeSnippet?.body}
      />
    );
  if (question.kind === "essay")
    return (
      <EssayRenderer
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  return (
    <ShortRenderer
      value={String(value ?? "")}
      onChange={onChange}
      disabled={disabled}
    />
  );
}
