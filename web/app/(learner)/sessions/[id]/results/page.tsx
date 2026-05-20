"use client";

import { useState, useEffect, use } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { ShareModal } from "@/components/ShareModal";

interface AttemptItem {
  id: string;
  questionId: string;
  isCorrect: boolean;
  score: number;
  response: Record<string, unknown>;
}

interface ResultSession {
  id: string;
  status: string;
  result?: {
    correct: number;
    total: number;
    score: number;
    needs_work: string[];
  };
}

interface SessionData {
  session: ResultSession;
  attempts: AttemptItem[];
}

export default function ResultsPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const [data, setData] = useState<SessionData | null>(null);
  const [showShare, setShowShare] = useState(false);

  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      .then(({ data: d }: { data?: SessionData }) => {
        if (d) setData(d);
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

  const result = data.session.result;
  const pct = result ? Math.round(result.score * 100) : null;
  const scoreColor =
    pct === null
      ? "var(--muted)"
      : pct >= 80
        ? "var(--accent)"
        : pct >= 60
          ? "var(--amber)"
          : "var(--red)";

  return (
    <div style={{ padding: "36px 40px 64px", maxWidth: 720 }}>
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
          marginBottom: 32,
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
            Session complete
          </div>
          {result && (
            <div style={{ display: "flex", alignItems: "baseline", gap: 16 }}>
              <span
                style={{
                  fontSize: 52,
                  fontWeight: 700,
                  color: scoreColor,
                  lineHeight: 1,
                }}
              >
                {pct}%
              </span>
              <span style={{ color: "var(--text-2)", fontSize: 16 }}>
                {result.correct}/{result.total} correct
              </span>
            </div>
          )}
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button
            onClick={() => setShowShare(true)}
            style={{
              padding: "8px 14px",
              background: "var(--surface-2)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              color: "var(--text-2)",
              cursor: "pointer",
              fontSize: 13,
              fontFamily: "var(--mono)",
            }}
          >
            Share
          </button>
          <a
            href="/practice"
            style={{
              padding: "8px 16px",
              background: "var(--accent)",
              border: "none",
              borderRadius: 4,
              color: "#000",
              fontWeight: 700,
              fontSize: 13,
              textDecoration: "none",
              display: "inline-block",
            }}
          >
            Practice again
          </a>
        </div>
      </div>

      {/* Needs work tags */}
      {result?.needs_work && result.needs_work.length > 0 && (
        <div
          style={{
            padding: "14px 18px",
            background: "var(--amber-dim)",
            border: "1px solid var(--amber)",
            borderRadius: 6,
            marginBottom: 28,
          }}
        >
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--amber)",
              marginBottom: 8,
            }}
          >
            Needs more practice
          </div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
            {result.needs_work.map((tag) => (
              <span
                key={tag}
                style={{
                  padding: "3px 10px",
                  background: "var(--surface-2)",
                  border: "1px solid var(--border)",
                  borderRadius: 4,
                  fontFamily: "var(--mono)",
                  fontSize: 12,
                  color: "var(--text-2)",
                }}
              >
                {tag}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Per-attempt breakdown */}
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
        Item breakdown · {data.attempts.length} question
        {data.attempts.length !== 1 ? "s" : ""}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {data.attempts.map((attempt, i) => (
          <AttemptRow key={attempt.id} attempt={attempt} index={i} />
        ))}
      </div>

      {showShare && token && (
        <ShareModal
          payload={{ kind: "quiz", id, title: "Session results" }}
          onClose={() => setShowShare(false)}
          bearerToken={token}
        />
      )}
    </div>
  );
}

function AttemptRow({
  attempt,
  index,
}: {
  attempt: AttemptItem;
  index: number;
}) {
  const [expanded, setExpanded] = useState(false);
  const pct = Math.round(attempt.score * 100);

  return (
    <div
      style={{
        background: "var(--surface)",
        border: `1px solid ${attempt.isCorrect ? "var(--border)" : "var(--border-strong)"}`,
        borderLeft: `3px solid ${attempt.isCorrect ? "var(--accent)" : "var(--red)"}`,
        borderRadius: 6,
        overflow: "hidden",
      }}
    >
      <button
        onClick={() => setExpanded((v) => !v)}
        style={{
          width: "100%",
          display: "flex",
          alignItems: "center",
          gap: 14,
          padding: "14px 18px",
          background: "none",
          border: "none",
          cursor: "pointer",
          textAlign: "left",
        }}
      >
        <span
          style={{
            fontFamily: "var(--mono)",
            fontSize: 12,
            color: "var(--muted)",
            flexShrink: 0,
            width: 24,
          }}
        >
          {index + 1}
        </span>
        <span
          style={{
            fontSize: 13,
            color: "var(--text-2)",
            flex: 1,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {attempt.questionId}
        </span>
        <span
          style={{
            fontFamily: "var(--mono)",
            fontSize: 13,
            fontWeight: 600,
            color: attempt.isCorrect ? "var(--accent)" : "var(--red)",
            flexShrink: 0,
          }}
        >
          {attempt.isCorrect ? "✓" : "✗"} {pct}%
        </span>
      </button>
      {expanded && (
        <div
          style={{
            padding: "12px 18px 16px",
            borderTop: "1px solid var(--border)",
            background: "var(--surface-2)",
          }}
        >
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 11,
              color: "var(--muted)",
            }}
          >
            Response: {JSON.stringify(attempt.response)}
          </div>
        </div>
      )}
    </div>
  );
}
