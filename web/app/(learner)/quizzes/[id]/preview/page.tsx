"use client";

import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { Button, Tag, Card } from "@/components/ui";

interface PreviewQuestion {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  orderIndex: number;
}

interface QuizDetail {
  id: string;
  title: string;
  course?: string;
  objectives?: string[];
  questions: PreviewQuestion[];
}

const KIND_LABEL: Record<string, string> = {
  mc: "MC",
  tf: "TF",
  short: "Short",
  essay: "Essay",
  code: "Code",
};

export default function QuizPreviewPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const router = useRouter();
  const [quiz, setQuiz] = useState<QuizDetail | null>(null);
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET("/v1/quizzes/{id}" as never, { params: { path: { id } } } as never)
      .then(({ data: d }: { data?: any }) => {
        if (d) setQuiz(d);
      })
      .catch(console.error);
  }, [token, id]);

  const handleStart = async () => {
    if (!token || !quiz) return;
    setStarting(true);
    try {
      const { data } = await (makeClient(token) as any).POST("/v1/sessions", {
        body: { quizId: id, count: quiz.questions.length },
      });
      const sessionId = data?.sessionId ?? data?.session_id;
      if (sessionId) {
        router.push(`/sessions/${sessionId}`);
      }
    } finally {
      setStarting(false);
    }
  };

  if (!quiz) {
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
        Loading…
      </div>
    );
  }

  const totalPoints = quiz.questions.reduce((s, q) => s + q.points, 0);
  const kindCounts = quiz.questions.reduce(
    (acc, q) => ({ ...acc, [q.kind]: (acc[q.kind] ?? 0) + 1 }),
    {} as Record<string, number>,
  );
  const kindSummary = Object.entries(kindCounts)
    .map(([k, n]) => `${n} ${KIND_LABEL[k] ?? k}`)
    .join(" · ");

  const sorted = [...quiz.questions].sort(
    (a, b) => a.orderIndex - b.orderIndex,
  );

  return (
    <div style={{ padding: "28px 36px 56px", maxWidth: 860 }}>
      {/* Breadcrumb */}
      <div
        style={{
          fontFamily: "var(--mono)",
          fontSize: 11,
          letterSpacing: 1.1,
          textTransform: "uppercase",
          color: "var(--muted)",
          marginBottom: 16,
          display: "flex",
          gap: 8,
          alignItems: "center",
        }}
      >
        <span
          style={{ cursor: "pointer", textDecoration: "underline" }}
          onClick={() => router.push("/library")}
        >
          Library
        </span>
        <span>›</span>
        <span>{quiz.title}</span>
      </div>

      {/* Header */}
      <div style={{ marginBottom: 24 }}>
        {quiz.course && (
          <Tag color="accent" style={{ marginBottom: 10 }}>
            {quiz.course}
          </Tag>
        )}
        <h1
          style={{
            fontFamily: "var(--serif)",
            fontSize: 36,
            fontWeight: 500,
            letterSpacing: -0.7,
            lineHeight: 1.1,
            color: "var(--text)",
            margin: "0 0 16px",
            textWrap: "balance",
          }}
        >
          {quiz.title}
        </h1>
        {quiz.objectives && quiz.objectives.length > 0 && (
          <ul
            style={{
              margin: 0,
              paddingLeft: 18,
              color: "var(--text-2)",
              fontSize: 13,
              lineHeight: 1.7,
            }}
          >
            {quiz.objectives.map((obj, i) => (
              <li key={i}>{obj}</li>
            ))}
          </ul>
        )}
      </div>

      {/* Stat strip */}
      <div
        style={{
          fontFamily: "var(--mono)",
          fontSize: 11,
          letterSpacing: 1.1,
          textTransform: "uppercase",
          color: "var(--muted)",
          marginBottom: 24,
          display: "flex",
          gap: 20,
        }}
      >
        <span>{quiz.questions.length} questions</span>
        <span>{totalPoints} pts</span>
        {kindSummary && <span>{kindSummary}</span>}
      </div>

      {/* Question list */}
      <Card style={{ padding: 0, marginBottom: 32 }}>
        {sorted.map((q, i) => (
          <div
            key={q.id}
            style={{
              padding: "16px 22px",
              borderBottom:
                i < sorted.length - 1 ? "1px solid var(--border)" : "none",
              display: "grid",
              gridTemplateColumns: "28px 1fr auto",
              gap: 14,
              alignItems: "start",
            }}
          >
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--muted)",
                paddingTop: 2,
              }}
            >
              {i + 1}
            </span>
            <div>
              <Tag
                color="muted"
                style={{ marginBottom: 6, fontSize: 9, letterSpacing: 1 }}
              >
                {KIND_LABEL[q.kind] ?? q.kind}
              </Tag>
              <div
                style={{
                  fontSize: 14,
                  color: "var(--text)",
                  lineHeight: 1.5,
                  display: "-webkit-box",
                  WebkitLineClamp: 2,
                  WebkitBoxOrient: "vertical",
                  overflow: "hidden",
                }}
              >
                {q.prompt}
              </div>
            </div>
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--muted)",
                whiteSpace: "nowrap",
                paddingTop: 2,
              }}
            >
              {q.points} pt{q.points !== 1 ? "s" : ""}
            </span>
          </div>
        ))}
      </Card>

      {/* Footer */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          paddingTop: 24,
          borderTop: "1px solid var(--border)",
        }}
      >
        <Button variant="outline" onClick={() => router.push("/library")}>
          Back to library
        </Button>
        <Button variant="primary" onClick={handleStart} disabled={starting}>
          {starting ? "Starting…" : "Start quiz"}
        </Button>
      </div>
    </div>
  );
}
