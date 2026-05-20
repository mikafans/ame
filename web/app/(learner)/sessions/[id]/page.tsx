"use client";

import { useState, useEffect, useCallback, use } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { McqRenderer } from "@/components/question/McqRenderer";
import { ShortRenderer } from "@/components/question/ShortRenderer";
import { EssayRenderer } from "@/components/question/EssayRenderer";
import { ClozeRenderer } from "@/components/question/ClozeRenderer";

interface SessionQuestion {
  questionId: string;
  kind: string;
  prompt: string;
  codeSnippet?: { language: string; body: string };
  options?: Array<{ text: string }>;
  optionOrder?: number[];
}

interface SessionData {
  session: { id: string; status: string; deadline_at?: string };
  questions: SessionQuestion[];
}

type Answer = string | number | null;

export default function ActiveQuizPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const router = useRouter();
  const [session, setSession] = useState<SessionData | null>(null);
  const [idx, setIdx] = useState(0);
  const [answers, setAnswers] = useState<Record<string, Answer>>({});
  const [submitting, setSubmitting] = useState(false);
  const [timeLeft, setTimeLeft] = useState<number | null>(null);
  const [finishing, setFinishing] = useState(false);

  // Load session
  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      .then(({ data }: { data?: SessionData }) => {
        if (data) {
          setSession(data);
          if (data.session.deadline_at) {
            const remaining = Math.max(
              0,
              Math.floor(
                (new Date(data.session.deadline_at).getTime() - Date.now()) /
                  1000,
              ),
            );
            setTimeLeft(remaining);
          }
        }
      })
      .catch(console.error);
  }, [token, id]);

  // Timer countdown
  useEffect(() => {
    if (timeLeft === null || timeLeft <= 0) return;
    const t = setInterval(() => {
      setTimeLeft((prev) => {
        if (prev === null || prev <= 1) {
          clearInterval(t);
          handleFinish();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [timeLeft !== null && timeLeft > 0]);

  const handleFinish = useCallback(async () => {
    if (finishing || !token) return;
    setFinishing(true);
    try {
      await makeClient(token).POST(
        "/v1/sessions/{id}/finish" as never,
        {
          params: { path: { id } },
        } as never,
      );
      router.push(`/sessions/${id}/results`);
    } catch (err) {
      console.error(err);
      setFinishing(false);
    }
  }, [token, id, router, finishing]);

  async function submitAnswer(questionId: string, answer: Answer) {
    if (!token || submitting || answer === null) return;
    setSubmitting(true);
    try {
      const body = buildAnswerBody(cur, answer);
      await makeClient(token).POST(
        "/v1/sessions/{id}/answer" as never,
        {
          params: { path: { id } },
          body: { questionId, ...body } as never,
        } as never,
      );
      setAnswers((prev) => ({ ...prev, [questionId]: answer }));
      if (idx < (session?.questions.length ?? 0) - 1) {
        setIdx((i) => i + 1);
      }
    } catch (err) {
      console.error(err);
    } finally {
      setSubmitting(false);
    }
  }

  if (!session) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Loading session…
      </div>
    );
  }

  const questions = session.questions;
  const cur = questions[idx];
  const answered = Object.keys(answers).length;
  const curAnswer = cur ? (answers[cur.questionId] ?? null) : null;

  const mm = timeLeft !== null ? Math.floor(timeLeft / 60) : null;
  const ss = timeLeft !== null ? String(timeLeft % 60).padStart(2, "0") : null;
  const timeWarning = timeLeft !== null && timeLeft < 180;

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "grid",
        gridTemplateColumns: "1fr 260px",
      }}
    >
      {/* Main */}
      <div
        style={{
          borderRight: "1px solid var(--border)",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {/* Sticky header */}
        <div
          style={{
            position: "sticky",
            top: 0,
            zIndex: 4,
            background: "var(--bg)",
            borderBottom: "1px solid var(--border)",
            padding: "16px 40px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div>
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10,
                letterSpacing: 1.3,
                color: "var(--muted)",
                textTransform: "uppercase",
                marginBottom: 4,
              }}
            >
              Question {idx + 1} of {questions.length}
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
            {timeLeft !== null && mm !== null && ss !== null && (
              <div
                style={{
                  padding: "6px 14px",
                  background: timeWarning
                    ? "var(--red-dim)"
                    : "var(--surface-2)",
                  border: `1px solid ${timeWarning ? "var(--red)" : "var(--border)"}`,
                  borderRadius: 4,
                  fontFamily: "var(--mono)",
                  fontSize: 15,
                  color: timeWarning ? "var(--red)" : "var(--text)",
                  letterSpacing: 1,
                }}
              >
                {mm}:{ss}
              </div>
            )}
            <button
              onClick={handleFinish}
              disabled={finishing}
              style={{
                padding: "8px 18px",
                background: "var(--accent)",
                border: "none",
                borderRadius: 4,
                color: "#000",
                fontWeight: 700,
                fontSize: 13,
                cursor: finishing ? "not-allowed" : "pointer",
                opacity: finishing ? 0.7 : 1,
              }}
            >
              {finishing ? "Finishing…" : "Finish"}
            </button>
          </div>
        </div>

        {/* Question body */}
        <div style={{ flex: 1, padding: "40px 40px 56px", overflowY: "auto" }}>
          {cur && (
            <>
              {/* Kind label */}
              <div
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 10,
                  letterSpacing: 1.3,
                  textTransform: "uppercase",
                  color: "var(--muted)",
                  marginBottom: 16,
                }}
              >
                {kindLabel(cur.kind)}
              </div>

              {/* Code snippet */}
              {cur.codeSnippet && (
                <pre
                  style={{
                    background: "var(--surface-3)",
                    border: "1px solid var(--border)",
                    borderRadius: 6,
                    padding: "16px 18px",
                    fontSize: 13,
                    fontFamily: "var(--mono)",
                    color: "var(--text)",
                    overflowX: "auto",
                    marginBottom: 24,
                  }}
                >
                  <code>{cur.codeSnippet.body}</code>
                </pre>
              )}

              {/* Prompt */}
              <div
                style={{
                  fontSize: 16,
                  fontWeight: 500,
                  color: "var(--text)",
                  lineHeight: 1.6,
                  marginBottom: 28,
                }}
              >
                {cur.prompt}
              </div>

              {/* Answer input */}
              <QuestionInput
                question={cur}
                value={curAnswer}
                onChange={(v) =>
                  setAnswers((prev) => ({ ...prev, [cur.questionId]: v }))
                }
                disabled={submitting || cur.questionId in answers}
              />

              {/* Submit / Next */}
              {!(cur.questionId in answers) && (
                <button
                  onClick={() => submitAnswer(cur.questionId, curAnswer)}
                  disabled={submitting || curAnswer === null}
                  style={{
                    marginTop: 28,
                    padding: "10px 24px",
                    background: "var(--accent)",
                    border: "none",
                    borderRadius: 4,
                    color: "#000",
                    fontWeight: 700,
                    fontSize: 14,
                    cursor:
                      submitting || curAnswer === null
                        ? "not-allowed"
                        : "pointer",
                    opacity: submitting || curAnswer === null ? 0.6 : 1,
                  }}
                >
                  {submitting ? "Submitting…" : "Submit"}
                </button>
              )}

              {cur.questionId in answers && idx < questions.length - 1 && (
                <button
                  onClick={() => setIdx((i) => i + 1)}
                  style={{
                    marginTop: 28,
                    padding: "10px 24px",
                    background: "var(--surface-2)",
                    border: "1px solid var(--border)",
                    borderRadius: 4,
                    color: "var(--text)",
                    fontWeight: 600,
                    fontSize: 14,
                    cursor: "pointer",
                  }}
                >
                  Next →
                </button>
              )}
            </>
          )}
        </div>
      </div>

      {/* Sidebar — question map */}
      <div style={{ padding: "24px 20px", overflowY: "auto" }}>
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.3,
            textTransform: "uppercase",
            color: "var(--muted)",
            marginBottom: 14,
          }}
        >
          Progress · {answered}/{questions.length}
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(5, 1fr)",
            gap: 6,
          }}
        >
          {questions.map((q, i) => {
            const done = q.questionId in answers;
            const current = i === idx;
            return (
              <button
                key={q.questionId}
                onClick={() => setIdx(i)}
                style={{
                  aspectRatio: "1",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  borderRadius: 4,
                  border: `1px solid ${current ? "var(--accent)" : done ? "var(--border)" : "var(--border)"}`,
                  background: current
                    ? "var(--accent-dim)"
                    : done
                      ? "var(--surface-3)"
                      : "var(--surface-2)",
                  color: current
                    ? "var(--accent)"
                    : done
                      ? "var(--muted)"
                      : "var(--text-2)",
                  cursor: "pointer",
                  fontFamily: "var(--mono)",
                  fontSize: 12,
                  fontWeight: current ? 700 : 400,
                }}
              >
                {i + 1}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function kindLabel(kind: string) {
  const map: Record<string, string> = {
    mcq: "Multiple choice",
    free_text: "Free text",
    cloze: "Fill in the blank",
  };
  return map[kind] ?? kind;
}

function buildAnswerBody(q: SessionQuestion, value: Answer) {
  if (q.kind === "mcq") return { selectedPosition: value };
  if (q.kind === "free_text") {
    const text = String(value ?? "");
    const wordCount = text.trim() ? text.trim().split(/\s+/).length : 0;
    if (wordCount > 30) return { body: text, wordCount };
    return { answer: text };
  }
  return { answer: String(value ?? "") };
}

function QuestionInput({
  question,
  value,
  onChange,
  disabled,
}: {
  question: SessionQuestion;
  value: Answer;
  onChange: (v: Answer) => void;
  disabled: boolean;
}) {
  if (question.kind === "mcq" && question.options) {
    const order = question.optionOrder ?? question.options.map((_, i) => i);
    const opts = order.map((pos) => ({
      text: question.options![pos].text,
      index: pos,
    }));
    return (
      <McqRenderer
        options={opts}
        value={typeof value === "number" ? value : null}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  if (question.kind === "cloze") {
    return (
      <ClozeRenderer
        prompt={question.prompt}
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  const isEssay =
    question.kind === "free_text" &&
    String(value ?? "")
      .trim()
      .split(/\s+/).length > 30;
  if (isEssay || question.kind === "free_text") {
    return (
      <EssayRenderer
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  return (
    <ShortRenderer
      value={String(value ?? "")}
      onChange={onChange}
      disabled={disabled}
    />
  );
}
