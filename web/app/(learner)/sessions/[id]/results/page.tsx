"use client";

import { useState, useEffect, useMemo, use } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { ShareModal } from "@/components/ShareModal";
import { Button, Tag, Card } from "@/components/ui";

interface Answer {
  qid: string;
  correct: boolean;
  points: number;
  max: number;
  type: "mc" | "tf" | "short" | "essay" | "code";
  given: string;
  note: string;
}

interface ResultData {
  id: string;
  quiz_id?: string;
  quiz_title: string;
  course: string;
  attempt_number: number;
  total_attempts: number;
  score: number;
  total: number;
  feedback?: string;
  answers: Answer[];
}

interface CohortHistogramBucket {
  bucket_start: number;
  bucket_end: number;
  count: number;
}

interface CohortStats {
  cohort_avg: number | null;
  percentile: number | null;
  completion_rate: number | null;
  histogram: CohortHistogramBucket[];
}

export default function ResultsPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const [data, setData] = useState<ResultData | null>(null);
  const [cohortStats, setCohortStats] = useState<CohortStats | null>(null);
  const [shareModal, setShareModal] = useState<{
    open: boolean;
    kind: string;
    itemId?: string;
  } | null>(null);
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      .then(({ data: d }: { data?: ResultData }) => {
        if (d) {
          setData(d);
          if (d.quiz_id) {
            fetch(
              `${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080"}/v1/me/cohort-stats?quizId=${d.quiz_id}`,
              { headers: { Authorization: `Bearer ${token}` } },
            )
              .then((r) => (r.ok ? r.json() : null))
              .then((cs: CohortStats | null) => {
                if (cs) setCohortStats(cs);
              })
              .catch(() => {});
          }
        }
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
  const kicker = `${data.course.toUpperCase()} · ATTEMPT ${data.attempt_number} OF ${data.total_attempts}`;

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
          <CohortHistogram
            userBin={Math.floor(pct / 10)}
            histogram={cohortStats?.histogram ?? null}
          />
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
          {/* Stats row */}
          <div
            style={{
              marginTop: 16,
              display: "grid",
              gridTemplateColumns: "1fr 1fr 1fr",
              gap: 8,
              borderTop: "1px solid var(--border)",
              paddingTop: 14,
            }}
          >
            {[
              {
                label: "Cohort avg",
                value:
                  cohortStats?.cohort_avg != null
                    ? `${Math.round(cohortStats.cohort_avg * 100)}%`
                    : "—",
              },
              {
                label: "Percentile",
                value:
                  cohortStats?.percentile != null
                    ? `${cohortStats.percentile}th`
                    : "—",
              },
              {
                label: "Completion",
                value:
                  cohortStats?.completion_rate != null
                    ? `${Math.round(cohortStats.completion_rate * 100)}%`
                    : "—",
              },
            ].map(({ label, value }) => (
              <div key={label} style={{ textAlign: "center" }}>
                <div
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 9,
                    letterSpacing: 1.1,
                    textTransform: "uppercase",
                    color: "var(--muted)",
                    marginBottom: 4,
                  }}
                >
                  {label}
                </div>
                <div
                  style={{
                    fontFamily: "var(--serif)",
                    fontSize: 18,
                    fontWeight: 500,
                    color: "var(--text)",
                  }}
                >
                  {value}
                </div>
              </div>
            ))}
          </div>
        </Card>
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
              {data.answers.filter((a) => !a.correct).length} needs work
            </Tag>
          </div>
        </div>
        <div>
          {data.answers.map((answer, i) => (
            <div
              key={answer.qid}
              style={{
                padding: "18px 22px",
                borderBottom:
                  i < data.answers.length - 1
                    ? "1px solid var(--border)"
                    : "none",
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
                    Q{i + 1} · {answer.type.toUpperCase()}
                  </span>
                </div>
                <div
                  style={{
                    fontFamily: "var(--serif)",
                    fontSize: 15,
                    lineHeight: 1.5,
                    color: "var(--text)",
                    marginBottom: 10,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {answer.given.substring(0, 80)}
                </div>
                <div
                  style={{
                    fontSize: 13,
                    color: "var(--text-2)",
                    fontStyle: "italic",
                  }}
                >
                  {answer.note}
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
                    Explanation would appear here.
                  </div>
                )}
              </div>

              {/* Right: score + actions */}
              <div style={{ textAlign: "right", whiteSpace: "nowrap" }}>
                <div
                  style={{
                    fontFamily: "var(--serif)",
                    fontSize: 22,
                    fontWeight: 500,
                    color: answer.correct ? "var(--text)" : "var(--text-2)",
                    marginBottom: 8,
                  }}
                >
                  {answer.points}
                  <span style={{ color: "var(--muted)", fontSize: 14 }}>
                    {" "}
                    / {answer.max}
                  </span>
                </div>
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
          ))}
        </div>
      </Card>

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

function CohortHistogram({
  userBin,
  histogram,
}: {
  userBin: number;
  histogram: CohortHistogramBucket[] | null;
}) {
  const bins = Array.from({ length: 10 }, (_, i) => i);

  const counts = useMemo(() => {
    if (histogram && histogram.length === 10) {
      return histogram.map((b) => b.count);
    }
    return null;
  }, [histogram]);

  const maxCount = useMemo(
    () => (counts ? Math.max(...counts, 1) : 1),
    [counts],
  );

  return (
    <svg
      width="100%"
      height={170}
      style={{ display: "block", marginBottom: 8 }}
    >
      {bins.map((bin) => {
        const x = (bin / 10) * 100;
        const height = counts ? counts[bin] / maxCount : 0.15;
        const barWidth = 100 / 10 / 1.5;
        const isUserBin = bin === userBin;

        return (
          <g key={bin}>
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
