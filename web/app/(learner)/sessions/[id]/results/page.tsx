"use client";

import { useState, useEffect, useMemo, use } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { ShareModal } from "@/components/ShareModal";
import { Button, Tag, Card } from "@/components/ui";

interface Answer {
  qid: string;
  correct: boolean;
  pendingReview: boolean;
  points: number;
  max: number;
  type: "mc" | "tf" | "short" | "essay" | "code";
  prompt: string;
  given: string;
  note: string;
}

interface ResultData {
  id: string;
  quiz_title: string;
  course: string;
  attempt_number: number;
  total_attempts: number;
  score: number;
  total: number;
  duration_min: number | null;
  feedback?: string;
  answers: Answer[];
}

function resolveAnswer(response: any, kind: string, q: any): string {
  if (!response) return "—";
  if (kind === "mc") {
    const pos: number = response.selected_position ?? response.answer ?? 0;
    const options: { text: string }[] = q.options ?? [];
    const order: number[] =
      q.option_order ?? q.optionOrder ?? options.map((_: any, i: number) => i);
    const origIdx = order[pos];
    return options[origIdx]?.text ?? `Option ${pos + 1}`;
  }
  if (kind === "tf")
    return response.answer === true
      ? "True"
      : response.answer === false
        ? "False"
        : "—";
  if (kind === "essay") return response.body ?? "—";
  if (kind === "short") return response.answer ?? "—";
  if (kind === "code") return response.code ?? response.body ?? "—";
  if (typeof response === "object") return JSON.stringify(response);
  return String(response);
}

export default function ResultsPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const [data, setData] = useState<ResultData | null>(null);
  const [shareModal, setShareModal] = useState<{
    open: boolean;
    kind: string;
    itemId?: string;
  } | null>(null);
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set());

  useEffect(() => {
    try {
      localStorage.setItem("ame.lastSessionId", id);
    } catch {
      /* ignore */
    }
  }, [id]);

  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .then(({ data: d }: { data?: any }) => {
        if (!d) return;
        const session = d.session ?? {};
        const attempts: any[] = d.attempts ?? [];
        const questions: any[] = d.questions ?? [];

        // Build a lookup from the hydrated question plan
        const questionById: Record<string, any> = {};
        for (const q of questions) {
          questionById[q.questionId ?? q.id] = q;
        }

        const answers: Answer[] = attempts.map((a: any) => {
          const qid = a.question_id ?? a.questionId ?? "";
          const q = questionById[qid] ?? {};
          const max = q.points ?? 1;
          const kind: string = q.kind ?? a.kind ?? "mc";
          const pendingReview = kind === "essay" || kind === "code";
          return {
            qid,
            correct: a.is_correct ?? false,
            pendingReview,
            points: a.score != null ? Math.round(a.score * max) : 0,
            max,
            type: kind as Answer["type"],
            prompt: q.prompt ?? "",
            given: resolveAnswer(a.response, kind, q),
            note: q.explanation ?? "",
          };
        });

        const score = answers.reduce((s, a) => s + a.points, 0);
        const total = answers.reduce((s, a) => s + a.max, 0) || 1;

        // Use stored result if available (set by /finish)
        const stored = session.result;
        const durationMin =
          session.started_at && session.finished_at
            ? Math.round(
                (new Date(session.finished_at).getTime() -
                  new Date(session.started_at).getTime()) /
                  60000,
              )
            : null;
        setData({
          id: session.id ?? id,
          quiz_title: session.quiz_title ?? "Quiz Results",
          course: session.course ?? session.kind ?? "quiz",
          attempt_number: 1,
          total_attempts: 1,
          score: stored?.points_awarded ?? score,
          total: stored?.max_points ?? total,
          duration_min: durationMin,
          feedback: stored?.feedback,
          answers,
        });
      })
      .catch(console.error);
  }, [token, id]);

  if (!data) {
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
        Loading results…
      </div>
    );
  }

  const pct = Math.round((data.score / data.total) * 100);
  const passed = pct >= 70;
  const kicker = `${(data.course ?? "quiz").toUpperCase()} · ATTEMPT ${data.attempt_number} OF ${data.total_attempts}`;

  const toggleExpanded = (qid: string) => {
    const next = new Set(expandedItems);
    if (next.has(qid)) {
      next.delete(qid);
    } else {
      next.add(qid);
    }
    setExpandedItems(next);
  };

  const handleShareQuiz = () => {
    setShareModal({ open: true, kind: "quiz" });
  };

  const handleShareItem = (qid: string) => {
    setShareModal({ open: true, kind: "item", itemId: qid });
  };

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Page header */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
          marginBottom: 28,
        }}
      >
        <div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 8,
            }}
          >
            {kicker}
          </div>
          <h1
            style={{
              fontFamily: "var(--serif)",
              fontSize: 40,
              fontWeight: 500,
              letterSpacing: -0.8,
              lineHeight: 1.1,
              color: "var(--text)",
              margin: 0,
            }}
          >
            {data.quiz_title} — Results
          </h1>
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          <Button variant="outline">Export PDF</Button>
          <Button variant="primary" onClick={handleShareQuiz}>
            Share quiz
          </Button>
        </div>
      </div>

      {/* Top row: two cards */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1.3fr 1fr",
          gap: 18,
          marginBottom: 24,
        }}
      >
        {/* Score card */}
        <Card style={{ padding: 28 }}>
          <div style={{ display: "flex", gap: 28, alignItems: "center" }}>
            {/* Donut chart */}
            <DonutChart correct={data.score} total={data.total} size={140} />

            <div style={{ flex: 1 }}>
              {/* Badges */}
              <div
                style={{
                  display: "flex",
                  gap: 10,
                  alignItems: "center",
                  marginBottom: 10,
                }}
              >
                <Tag color={passed ? "accent" : "red"}>
                  {passed ? "PASS" : "FAIL"}
                </Tag>
              </div>

              {/* Score fraction */}
              <div
                style={{
                  fontFamily: "var(--serif)",
                  fontSize: 26,
                  fontWeight: 500,
                  letterSpacing: -0.5,
                  lineHeight: 1,
                }}
              >
                {data.score}{" "}
                <span style={{ color: "var(--muted)", fontSize: 16 }}>
                  / {data.total} pts
                </span>
              </div>

              {/* AI feedback */}
              <div
                style={{
                  marginTop: 14,
                  color: "var(--text-2)",
                  fontSize: 13,
                  lineHeight: 1.6,
                  maxWidth: 460,
                  fontStyle: "italic",
                }}
              >
                {data.feedback || "No feedback provided for this quiz."}
              </div>
            </div>
          </div>
        </Card>

        {/* Cohort distribution */}
        <Card style={{ padding: 24 }}>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 8,
            }}
          >
            Cohort distribution
          </div>
          <CohortHistogram userBin={Math.min(Math.floor(pct / 10), 9)} />
          <div
            style={{
              marginTop: 12,
              fontSize: 12,
              color: "var(--muted)",
              display: "flex",
              gap: 16,
              fontFamily: "var(--mono)",
              letterSpacing: 0.5,
            }}
          >
            <span>
              <span
                style={{
                  display: "inline-block",
                  width: 8,
                  height: 8,
                  background: "var(--accent)",
                  marginRight: 4,
                }}
              />{" "}
              Your bin
            </span>
            <span>
              <span
                style={{
                  display: "inline-block",
                  width: 8,
                  height: 8,
                  background: "var(--surface-3)",
                  marginRight: 4,
                }}
              />{" "}
              Cohort
            </span>
          </div>
        </Card>
      </div>

      {/* Stats row */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          gap: 12,
          marginBottom: 24,
        }}
      >
        {[
          {
            label: "Duration",
            value:
              data.duration_min != null
                ? data.duration_min >= 60
                  ? `${Math.floor(data.duration_min / 60)}h ${data.duration_min % 60}m`
                  : `${data.duration_min}m`
                : "—",
          },
          { label: "Cohort avg", value: "—" },
          { label: "Percentile", value: "—" },
          { label: "Topics mastered", value: "—" },
        ].map(({ label, value }) => (
          <Card key={label} style={{ padding: "16px 20px" }}>
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10,
                letterSpacing: 1.3,
                textTransform: "uppercase",
                color: "var(--muted)",
                marginBottom: 6,
              }}
            >
              {label}
            </div>
            <div
              style={{
                fontFamily: "var(--serif)",
                fontSize: 22,
                fontWeight: 500,
                letterSpacing: -0.3,
              }}
            >
              {value}
            </div>
          </Card>
        ))}
      </div>

      {/* Per-item review */}
      <Card style={{ padding: 0 }}>
        <div
          style={{
            padding: "16px 22px",
            borderBottom: "1px solid var(--border)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div>
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10,
                letterSpacing: 1.3,
                textTransform: "uppercase",
                color: "var(--muted)",
              }}
            >
              Per-item
            </div>
            <div
              style={{
                fontFamily: "var(--serif)",
                fontSize: 18,
                fontWeight: 500,
                marginTop: 2,
              }}
            >
              Answer review
            </div>
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <Tag color="accent">
              {data.answers.filter((a) => a.correct).length} correct
            </Tag>
            <Tag color="red">
              {
                data.answers.filter((a) => !a.correct && !a.pendingReview)
                  .length
              }{" "}
              needs work
            </Tag>
            {data.answers.some((a) => a.pendingReview) && (
              <Tag color="muted">
                {data.answers.filter((a) => a.pendingReview).length} pending
                review
              </Tag>
            )}
          </div>
        </div>
        <div>
          {data.answers.map((answer, i) => {
            const borderColor = answer.pendingReview
              ? "var(--surface-3)"
              : answer.correct
                ? "var(--accent)"
                : "var(--red, #ef4444)";
            const scoreColor = answer.pendingReview
              ? "var(--muted)"
              : answer.correct
                ? "var(--accent)"
                : "var(--red, #ef4444)";
            const iconSymbol = answer.pendingReview
              ? "○"
              : answer.correct
                ? "✓"
                : "✗";
            const iconColor = answer.pendingReview
              ? "var(--muted)"
              : answer.correct
                ? "var(--accent)"
                : "var(--red, #ef4444)";

            return (
              <div
                key={answer.qid}
                style={{
                  padding: "18px 22px",
                  borderBottom:
                    i < data.answers.length - 1
                      ? "1px solid var(--border)"
                      : "none",
                  borderLeft: `3px solid ${borderColor}`,
                  display: "grid",
                  gridTemplateColumns: "1fr auto",
                  gap: 18,
                  alignItems: "start",
                }}
              >
                {/* Left: question info */}
                <div style={{ minWidth: 0 }}>
                  <div
                    style={{
                      display: "flex",
                      gap: 8,
                      alignItems: "center",
                      marginBottom: 6,
                    }}
                  >
                    <span
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 11,
                        color: "var(--muted)",
                        letterSpacing: 1.1,
                        textTransform: "uppercase",
                      }}
                    >
                      <span style={{ color: iconColor, marginRight: 6 }}>
                        {iconSymbol}
                      </span>
                      Q{i + 1} · {answer.type.toUpperCase()}
                    </span>
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--serif)",
                      fontSize: 15,
                      lineHeight: 1.5,
                      color: "var(--text)",
                      marginBottom: 8,
                    }}
                  >
                    {answer.prompt}
                  </div>
                  <div
                    style={{
                      display:
                        answer.type === "essay" ? "block" : "inline-block",
                      fontSize: 13,
                      color:
                        answer.type === "essay"
                          ? "var(--muted)"
                          : "var(--text-2)",
                      lineHeight: 1.5,
                    }}
                  >
                    {answer.type === "essay" ? `"${answer.given}"` : null}
                    {answer.type !== "essay" && (
                      <span
                        style={{
                          fontFamily: "var(--mono)",
                          fontSize: 12,
                          background: "var(--surface-2)",
                          border: "1px solid var(--border)",
                          borderRadius: 4,
                          padding: "4px 8px",
                          marginLeft: 4,
                          color: "var(--text)",
                        }}
                      >
                        {answer.given}
                      </span>
                    )}
                  </div>

                  {/* Expanded explanation */}
                  {expandedItems.has(answer.qid) && (
                    <div
                      style={{
                        marginTop: 12,
                        paddingTop: 12,
                        borderTop: "1px solid var(--border)",
                        fontSize: 13,
                        color: "var(--text-2)",
                        lineHeight: 1.6,
                      }}
                    >
                      {answer.note ||
                        "No explanation available for this question."}
                    </div>
                  )}
                </div>

                {/* Right: score + actions */}
                <div style={{ textAlign: "right", whiteSpace: "nowrap" }}>
                  {answer.pendingReview ? (
                    <div
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 11,
                        letterSpacing: 1.1,
                        textTransform: "uppercase",
                        color: "var(--muted)",
                        marginBottom: 8,
                        paddingTop: 4,
                      }}
                    >
                      Pending review
                      <div
                        style={{
                          color: "var(--muted)",
                          fontSize: 13,
                          fontFamily: "var(--serif)",
                          fontWeight: 400,
                          letterSpacing: 0,
                          textTransform: "none",
                          marginTop: 2,
                        }}
                      >
                        — / {answer.max}
                      </div>
                    </div>
                  ) : (
                    <div
                      style={{
                        fontFamily: "var(--serif)",
                        fontSize: 22,
                        fontWeight: 500,
                        color: scoreColor,
                        marginBottom: 8,
                      }}
                    >
                      {answer.points}
                      <span style={{ color: "var(--muted)", fontSize: 14 }}>
                        {" "}
                        / {answer.max}
                      </span>
                    </div>
                  )}
                  <div
                    style={{
                      display: "flex",
                      gap: 10,
                      justifyContent: "flex-end",
                    }}
                  >
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => handleShareItem(answer.qid)}
                    >
                      Share
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => toggleExpanded(answer.qid)}
                    >
                      {expandedItems.has(answer.qid) ? "Hide" : "See"} solution
                    </Button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </Card>

      {/* Footer */}
      <div
        style={{
          display: "flex",
          justifyContent: "flex-end",
          gap: 12,
          marginTop: 32,
          paddingTop: 24,
          borderTop: "1px solid var(--border)",
        }}
      >
        <Button
          variant="outline"
          onClick={() => (window.location.href = "/library")}
        >
          Back to library
        </Button>
        <Button
          variant="primary"
          onClick={() => (window.location.href = "/progress")}
        >
          View 6-week plan
        </Button>
      </div>

      {/* Share modal */}
      {shareModal?.open && token && (
        <ShareModal
          payload={{
            kind: shareModal.kind as "quiz" | "item",
            id: shareModal.itemId || id,
            title: shareModal.kind === "item" ? "Question" : data.quiz_title,
          }}
          onClose={() => setShareModal(null)}
          bearerToken={token}
        />
      )}
    </div>
  );
}

function DonutChart({
  correct,
  total,
  size = 140,
}: {
  correct: number;
  total: number;
  size: number;
}) {
  const pct = (correct / total) * 100;
  const circumference = 2 * Math.PI * (size / 2 - 12);
  const strokeDashoffset = circumference * (1 - pct / 100);

  return (
    <div style={{ position: "relative", width: size, height: size }}>
      <svg width={size} height={size} style={{ transform: "rotate(-90deg)" }}>
        {/* Background circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={size / 2 - 12}
          fill="none"
          stroke="var(--surface-2)"
          strokeWidth={8}
        />
        {/* Progress circle */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={size / 2 - 12}
          fill="none"
          stroke="var(--accent)"
          strokeWidth={8}
          strokeDasharray={circumference}
          strokeDashoffset={strokeDashoffset}
          strokeLinecap="round"
          style={{ transition: "stroke-dashoffset 300ms" }}
        />
      </svg>
      {/* Percentage text */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexDirection: "column",
        }}
      >
        <div
          style={{
            fontFamily: "var(--serif)",
            fontSize: 36,
            fontWeight: 600,
            lineHeight: 1,
            letterSpacing: -0.5,
          }}
        >
          {Math.round(pct)}%
        </div>
      </div>
    </div>
  );
}

function CohortHistogram({ userBin }: { userBin: number }) {
  const bins = Array.from({ length: 10 }, (_, i) => i);
  const heights = useMemo(() => bins.map(() => Math.random() * 0.8 + 0.2), []);

  return (
    <svg
      width="100%"
      height={170}
      style={{ display: "block", marginBottom: 8 }}
    >
      {bins.map((bin, i) => {
        const x = (bin / 10) * 100;
        const height = heights[i];
        const barWidth = 100 / 10 / 1.5;
        const isUserBin = bin === userBin;

        return (
          <g key={bin}>
            {/* Bar */}
            <rect
              x={`${x + (10 - barWidth) / 2}%`}
              y={`${100 - height * 100}%`}
              width={`${barWidth}%`}
              height={`${height * 100}%`}
              fill={isUserBin ? "var(--accent)" : "var(--surface-3)"}
              rx={2}
            />
          </g>
        );
      })}
    </svg>
  );
}
