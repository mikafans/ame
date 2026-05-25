"use client";

import { useState, useEffect, useCallback, useRef, use } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { McqRenderer } from "@/components/question/McqRenderer";
import { ShortRenderer } from "@/components/question/ShortRenderer";
import { EssayRenderer } from "@/components/question/EssayRenderer";
import { ClozeRenderer } from "@/components/question/ClozeRenderer";
import { TfRenderer } from "@/components/question/TfRenderer";
import { CodeRenderer } from "@/components/question/CodeRenderer";

interface SessionQuestion {
  questionId: string;
  kind: string;
  prompt: string;
  points: number;
  codeSnippet?: { language: string; body: string };
  options?: Array<{ text: string }>;
  optionOrder?: number[];
}

interface SessionData {
  session: {
    id: string;
    status: string;
    deadline_at?: string;
    duration?: number;
    allowed_materials?: string[];
    course_title?: string;
    quiz_title?: string;
  };
  questions: SessionQuestion[];
}

type Answer = string | number | boolean | null;

const QUESTION_TYPES: Record<string, string> = {
  mc: "Multiple choice",
  tf: "True / false",
  short: "Short answer",
  essay: "Essay",
  code: "Code",
  mcq: "Multiple choice",
  free_text: "Free text",
  cloze: "Fill in the blank",
};

const DEFAULT_ALLOWED_MATERIALS = [
  "One sheet of notes (any)",
  "Class textbook (printed)",
  "Standard calculator",
];

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
  const [flagged, setFlagged] = useState<Record<string, boolean>>({});
  const [timeLeft, setTimeLeft] = useState<number | null>(null);
  const [finishing, setFinishing] = useState(false);
  const autosaveScheduled = useRef(false);

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
          } else if (data.questions.length > 0) {
            setTimeLeft(Math.max(20, data.questions.length * 2) * 60);
          }
        }
      })
      .catch(console.error);
  }, [token, id]);

  const handleFinish = useCallback(async () => {
    if (finishing || !token) return;
    setFinishing(true);
    try {
      const client = makeClient(token);
      // Submit each answered question in the correct AttemptResponse shape
      for (const [qid, val] of Object.entries(answers)) {
        const q = session?.questions.find((x) => x.questionId === qid);
        if (!q) continue;
        let response: Record<string, unknown>;
        if (q.kind === "mc") {
          // answers store the original index; API expects display position
          const displayPos = q.optionOrder
            ? q.optionOrder.indexOf(val as number)
            : (val as number);
          response = {
            selected_position: displayPos >= 0 ? displayPos : (val as number),
          };
        } else if (q.kind === "tf") {
          response = { answer: val as boolean };
        } else if (q.kind === "essay") {
          const body = String(val ?? "");
          response = {
            body,
            word_count: body.trim().split(/\s+/).filter(Boolean).length,
          };
        } else {
          response = { answer: String(val ?? "") };
        }
        await (
          client as never as {
            POST: (p: string, o: unknown) => Promise<unknown>;
          }
        ).POST("/v1/sessions/{id}/answer", {
          params: { path: { id } },
          body: { questionId: qid, response },
        });
      }
      // Finish the session — sets session.result
      await (
        client as never as { POST: (p: string, o: unknown) => Promise<unknown> }
      ).POST("/v1/sessions/{id}/finish", { params: { path: { id } } });
      try {
        localStorage.setItem("ame.lastSessionId", id);
      } catch {
        /* ignore */
      }
      router.push(`/sessions/${id}/results`);
    } catch (err) {
      console.error(err);
      setFinishing(false);
    }
  }, [token, id, router, finishing, answers, session]);

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

  // Autosave every 8 seconds
  useEffect(() => {
    if (
      !token ||
      Object.keys(answers).length === 0 ||
      autosaveScheduled.current
    )
      return;
    autosaveScheduled.current = true;
    const timer = setTimeout(() => {
      const answersList = Object.entries(answers).map(([qid, val]) => ({
        questionId: qid,
        answer: val,
      }));
      makeClient(token)
        .PATCH(
          "/v1/sessions/{id}/answers" as never,
          {
            params: { path: { id } },
            body: { answers: answersList } as never,
          } as never,
        )
        .catch(console.error)
        .finally(() => {
          autosaveScheduled.current = false;
        });
    }, 8000);
    return () => {
      clearTimeout(timer);
      autosaveScheduled.current = false;
    };
  }, [answers, token, id]);

  const handleSaveExit = useCallback(async () => {
    if (!token) return;
    try {
      await makeClient(token).PATCH(
        "/v1/sessions/{id}" as never,
        {
          params: { path: { id } },
          body: { status: "abandoned" },
        } as never,
      );
      router.push("/sessions");
    } catch (err) {
      console.error(err);
    }
  }, [token, id, router]);

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
  const flaggedCount = Object.values(flagged).filter(Boolean).length;
  const curAnswer = cur ? (answers[cur.questionId] ?? null) : null;

  const mm = timeLeft !== null ? Math.floor(timeLeft / 60) : null;
  const ss = timeLeft !== null ? String(timeLeft % 60).padStart(2, "0") : null;
  const timeWarning = timeLeft !== null && timeLeft < 300;

  const attemptNum = 1; // TODO: get from session data
  const totalAttempts = 2; // TODO: get from session data
  const allowedMaterials =
    session.session.allowed_materials &&
    session.session.allowed_materials.length > 0
      ? session.session.allowed_materials
      : DEFAULT_ALLOWED_MATERIALS;

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "grid",
        gridTemplateColumns: "1fr 240px",
      }}
    >
      {/* Main quiz area */}
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          borderRight: "1px solid var(--border)",
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
            paddingBottom: "19px",
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
              Attempt {attemptNum} of {totalAttempts} ·{" "}
              {session.session.course_title || "COURSE"}
            </div>
            <div
              style={{
                fontFamily: "var(--serif)",
                fontSize: 18,
                fontWeight: 500,
                letterSpacing: -0.2,
                color: "var(--text)",
              }}
            >
              {session.session.quiz_title || "Quiz"}
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
            {timeLeft !== null && mm !== null && ss !== null && (
              <div
                style={{
                  padding: "8px 14px",
                  border: `1px solid ${timeWarning ? "var(--red)" : "var(--border)"}`,
                  borderRadius: 6,
                  background: timeWarning ? "var(--red-dim)" : "var(--surface)",
                  fontFamily: "var(--mono)",
                  fontSize: 14,
                  fontWeight: 600,
                  color: timeWarning ? "var(--red)" : "var(--text)",
                  display: "flex",
                  gap: 8,
                  alignItems: "center",
                  letterSpacing: 0.5,
                }}
              >
                <Icon
                  name="bell"
                  size={14}
                  color={timeWarning ? "var(--red)" : "var(--text)"}
                />
                {mm}:{ss}
              </div>
            )}
            <Button variant="ghost" onClick={handleSaveExit}>
              Save & exit
            </Button>
          </div>

          {/* Progress bar — inside sticky header so it stays pinned while scrolling */}
          <div
            style={{
              position: "absolute",
              bottom: 0,
              left: 0,
              right: 0,
              height: 3,
              background: "var(--surface-2)",
            }}
          >
            <div
              style={{
                height: "100%",
                width: `${questions.length > 0 ? (answered / questions.length) * 100 : 0}%`,
                background: "var(--accent)",
                transition: "width 200ms",
              }}
            />
          </div>
        </div>

        {/* Question content */}
        <div
          style={{
            flex: 1,
            padding: "44px 56px",
            overflowY: "auto",
            paddingBottom: 56,
          }}
        >
          {cur && (
            <>
              {/* Question sub-header */}
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "baseline",
                  marginBottom: 10,
                }}
              >
                <div
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 11,
                    letterSpacing: 1.3,
                    color: "var(--muted)",
                    textTransform: "uppercase",
                  }}
                >
                  Question {idx + 1} of {questions.length} ·{" "}
                  {QUESTION_TYPES[cur.kind] || cur.kind} · {cur.points}{" "}
                  {cur.points === 1 ? "pt" : "pts"}
                </div>
                <button
                  onClick={() =>
                    setFlagged((f) => ({
                      ...f,
                      [cur.questionId]: !f[cur.questionId],
                    }))
                  }
                  style={{
                    background: "transparent",
                    border: "none",
                    cursor: "pointer",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                    fontFamily: "var(--mono)",
                    fontSize: 11,
                    letterSpacing: 1,
                    color: flagged[cur.questionId]
                      ? "var(--amber)"
                      : "var(--muted)",
                    padding: 0,
                  }}
                >
                  <Icon
                    name="flag"
                    size={12}
                    color={
                      flagged[cur.questionId] ? "var(--amber)" : "var(--muted)"
                    }
                  />
                  {flagged[cur.questionId] ? "Flagged" : "Flag for review"}
                </button>
              </div>

              {/* Prompt */}
              <h2
                style={{
                  fontFamily: "var(--serif)",
                  fontSize: 26,
                  lineHeight: 1.35,
                  fontWeight: 500,
                  letterSpacing: -0.2,
                  margin: 0,
                  color: "var(--text)",
                  marginTop: 20,
                }}
              >
                {cur.prompt}
              </h2>

              {/* Question input */}
              <div data-testid="question-input" style={{ marginTop: 32 }}>
                <QuestionInput
                  question={cur}
                  value={curAnswer}
                  onChange={(v) =>
                    setAnswers((prev) => ({ ...prev, [cur.questionId]: v }))
                  }
                  disabled={false}
                />
              </div>

              {/* Navigation */}
              <div
                style={{
                  marginTop: 48,
                  paddingTop: 24,
                  borderTop: "1px solid var(--border)",
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <Button
                  variant="ghost"
                  onClick={() => setIdx((i) => Math.max(0, i - 1))}
                  disabled={idx === 0}
                >
                  Previous
                </Button>
                {idx < questions.length - 1 ? (
                  <Button
                    variant="primary"
                    onClick={() =>
                      setIdx((i) => Math.min(questions.length - 1, i + 1))
                    }
                  >
                    Next question
                  </Button>
                ) : (
                  <Button
                    variant="primary"
                    onClick={handleFinish}
                    disabled={finishing}
                  >
                    {finishing ? "Submitting…" : "Submit attempt"}
                  </Button>
                )}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Right rail */}
      <aside
        style={{
          padding: "20px 22px",
          background: "var(--surface)",
          overflowY: "auto",
        }}
      >
        {/* Question palette */}
        <div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 10,
            }}
          >
            Question palette
          </div>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(5, 1fr)",
              gap: 6,
              marginBottom: 18,
            }}
          >
            {questions.map((q, i) => {
              const status =
                q.questionId in answers ? "answered" : "unanswered";
              const isCurrent = i === idx;
              const isFlagged = flagged[q.questionId];
              return (
                <button
                  key={q.questionId}
                  onClick={() => setIdx(i)}
                  style={{
                    height: 36,
                    position: "relative",
                    background:
                      status === "answered"
                        ? "var(--accent-dim)"
                        : "var(--surface-2)",
                    color:
                      status === "answered" ? "var(--accent)" : "var(--text-2)",
                    border: `1px solid ${
                      isCurrent
                        ? "var(--accent)"
                        : status === "answered"
                          ? "var(--accent-line)"
                          : "var(--border)"
                    }`,
                    fontFamily: "var(--mono)",
                    fontSize: 12,
                    fontWeight: 600,
                    borderRadius: 4,
                    cursor: "pointer",
                  }}
                >
                  {i + 1}
                  {isFlagged ? (
                    <span
                      style={{
                        position: "absolute",
                        top: 2,
                        right: 3,
                        width: 5,
                        height: 5,
                        borderRadius: "50%",
                        background: "var(--amber)",
                      }}
                    />
                  ) : null}
                </button>
              );
            })}
          </div>
        </div>

        {/* Divider */}
        <div
          style={{
            height: "1px",
            background: "var(--border)",
            margin: "12px 0",
          }}
        />

        {/* Legend */}
        <div
          style={{
            marginTop: 18,
            display: "flex",
            flexDirection: "column",
            gap: 8,
          }}
        >
          <LegendItem
            swatch="var(--accent-dim)"
            border="var(--accent-line)"
            label={`Answered · ${answered}`}
          />
          <LegendItem
            swatch="var(--surface-2)"
            border="var(--border)"
            label={`Unanswered · ${questions.length - answered}`}
          />
          <LegendItem dot="var(--amber)" label={`Flagged · ${flaggedCount}`} />
        </div>

        {/* Divider */}
        <div
          style={{
            height: "1px",
            background: "var(--border)",
            margin: "18px 0",
          }}
        />

        {/* Integrity */}
        <div style={{ marginTop: 18 }}>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 10,
            }}
          >
            Integrity
          </div>
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 8,
              fontSize: 12,
              color: "var(--text-2)",
            }}
          >
            <IntegrityRow icon="check" text="Browser locked" />
            <IntegrityRow icon="check" text="Single tab session" />
            <IntegrityRow icon="check" text="Autosave every 8 s" />
          </div>
        </div>

        {/* Allowed materials */}
        <div
          style={{
            marginTop: 26,
            padding: 14,
            background: "var(--surface-2)",
            border: "1px solid var(--border)",
            borderRadius: 6,
          }}
        >
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.2,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 6,
            }}
          >
            Allowed
          </div>
          <ul
            style={{
              margin: 0,
              paddingLeft: 16,
              color: "var(--text-2)",
              fontSize: 12,
              lineHeight: 1.7,
            }}
          >
            {allowedMaterials.map((item, i) => (
              <li key={i}>{item}</li>
            ))}
          </ul>
        </div>
      </aside>
    </div>
  );
}

function LegendItem({
  swatch,
  border,
  dot,
  label,
}: {
  swatch?: string;
  border?: string;
  dot?: string;
  label: string;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        color: "var(--text-2)",
        fontSize: 12,
      }}
    >
      {swatch ? (
        <div
          style={{
            width: 14,
            height: 14,
            background: swatch,
            border: `1px solid ${border}`,
            borderRadius: 3,
          }}
        />
      ) : (
        <div style={{ width: 14, display: "flex", justifyContent: "center" }}>
          <div
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: dot,
            }}
          />
        </div>
      )}
      <span>{label}</span>
    </div>
  );
}

function IntegrityRow({ icon, text }: { icon: string; text: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <Icon name={icon} size={13} color="var(--accent)" />
      <span>{text}</span>
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
  onChange: (v: Answer) => void;
  disabled: boolean;
}) {
  // Multiple choice
  if ((question.kind === "mc" || question.kind === "mcq") && question.options) {
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

  // True/False
  if (question.kind === "tf") {
    return (
      <TfRenderer
        value={typeof value === "boolean" ? value : null}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }

  // Cloze
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

  // Code
  if (question.kind === "code") {
    return (
      <CodeRenderer
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }

  // Essay (free text with >30 words)
  const isEssay =
    question.kind === "free_text" &&
    String(value ?? "")
      .trim()
      .split(/\s+/).length > 30;
  if (isEssay || question.kind === "essay") {
    return (
      <EssayRenderer
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }

  // Short answer / free text
  return (
    <ShortRenderer
      value={String(value ?? "")}
      onChange={onChange}
      disabled={disabled}
    />
  );
}
