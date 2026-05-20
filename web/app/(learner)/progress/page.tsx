"use client";

import { useState, useEffect } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";

interface AttemptItem {
  id: string;
  questionId: string;
  isCorrect: boolean;
  score: number;
  userTagDeltas: Record<string, number>;
  createdAt: string;
}

interface TagStat {
  tag: string;
  total: number;
  correct: number;
  delta: number;
}

export default function ProgressPage() {
  const { token } = useAuth();
  const [attempts, setAttempts] = useState<AttemptItem[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET(
        "/v1/me/attempts" as never,
        {
          params: { query: { limit: 200 } },
        } as never,
      )
      .then(
        ({ data }: { data?: { attempts: AttemptItem[]; total: number } }) => {
          if (data?.attempts) setAttempts(data.attempts);
        },
      )
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [token]);

  const tagStats = computeTagStats(attempts);
  const weakest = [...tagStats]
    .sort((a, b) => a.correct / a.total - b.correct / b.total)
    .slice(0, 5);
  const overall =
    attempts.length > 0
      ? Math.round(
          (attempts.filter((a) => a.isCorrect).length / attempts.length) * 100,
        )
      : null;

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Header */}
      <div style={{ marginBottom: 32 }}>
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
          Your learning
        </div>
        <h1
          style={{
            margin: "0 0 4px",
            fontSize: 26,
            fontWeight: 600,
            color: "var(--text)",
          }}
        >
          Progress
        </h1>
        {overall !== null && (
          <div style={{ color: "var(--text-2)", fontSize: 14 }}>
            Overall accuracy:{" "}
            <span
              style={{
                fontFamily: "var(--mono)",
                fontWeight: 700,
                color:
                  overall >= 80
                    ? "var(--accent)"
                    : overall >= 60
                      ? "var(--amber)"
                      : "var(--red)",
              }}
            >
              {overall}%
            </span>{" "}
            across {attempts.length} attempt{attempts.length !== 1 ? "s" : ""}
          </div>
        )}
      </div>

      {loading ? (
        <div
          style={{
            color: "var(--muted)",
            fontFamily: "var(--mono)",
            fontSize: 13,
          }}
        >
          Loading…
        </div>
      ) : attempts.length === 0 ? (
        <div
          style={{
            padding: "48px 0",
            textAlign: "center",
            color: "var(--muted)",
            fontSize: 14,
          }}
        >
          No attempts yet.{" "}
          <a
            href="/practice"
            style={{ color: "var(--accent)", textDecoration: "none" }}
          >
            Start practicing
          </a>{" "}
          to see your progress here.
        </div>
      ) : (
        <div
          style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}
        >
          {/* Per-tag mastery */}
          <div>
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
              Tag mastery
            </div>
            {tagStats.length === 0 ? (
              <div style={{ color: "var(--muted)", fontSize: 13 }}>
                No tags tracked yet.
              </div>
            ) : (
              <div
                style={{ display: "flex", flexDirection: "column", gap: 10 }}
              >
                {tagStats.map((s) => (
                  <TagMasteryRow key={s.tag} stat={s} />
                ))}
              </div>
            )}
          </div>

          {/* Weakest tags */}
          <div>
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
              Needs work
            </div>
            {weakest.length === 0 ? (
              <div style={{ color: "var(--muted)", fontSize: 13 }}>
                Nothing flagged yet.
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {weakest.map((s) => {
                  const pct = Math.round((s.correct / s.total) * 100);
                  return (
                    <a
                      key={s.tag}
                      href={`/practice?tags=${encodeURIComponent(s.tag)}`}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        padding: "12px 16px",
                        background: "var(--surface)",
                        border: "1px solid var(--border)",
                        borderRadius: 6,
                        textDecoration: "none",
                        gap: 12,
                      }}
                    >
                      <span
                        style={{
                          fontFamily: "var(--mono)",
                          fontSize: 12,
                          color: "var(--text-2)",
                        }}
                      >
                        {s.tag}
                      </span>
                      <span
                        style={{
                          fontFamily: "var(--mono)",
                          fontSize: 13,
                          fontWeight: 700,
                          color: pct >= 60 ? "var(--amber)" : "var(--red)",
                        }}
                      >
                        {pct}%
                      </span>
                    </a>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function TagMasteryRow({ stat }: { stat: TagStat }) {
  const pct = Math.round((stat.correct / stat.total) * 100);
  const barColor =
    pct >= 80 ? "var(--accent)" : pct >= 60 ? "var(--amber)" : "var(--red)";

  return (
    <div
      style={{
        background: "var(--surface)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        padding: "12px 16px",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          marginBottom: 8,
        }}
      >
        <span
          style={{
            fontFamily: "var(--mono)",
            fontSize: 12,
            color: "var(--text-2)",
          }}
        >
          {stat.tag}
        </span>
        <span
          style={{
            fontFamily: "var(--mono)",
            fontSize: 12,
            color: "var(--muted)",
          }}
        >
          {stat.correct}/{stat.total}
        </span>
      </div>
      <div
        style={{
          height: 4,
          background: "var(--surface-3)",
          borderRadius: 2,
          overflow: "hidden",
        }}
      >
        <div
          style={{
            height: "100%",
            width: `${pct}%`,
            background: barColor,
            borderRadius: 2,
            transition: "width 0.4s ease",
          }}
        />
      </div>
    </div>
  );
}

function computeTagStats(attempts: AttemptItem[]): TagStat[] {
  const map = new Map<string, TagStat>();
  for (const a of attempts) {
    for (const [tag, delta] of Object.entries(a.userTagDeltas)) {
      if (!map.has(tag)) {
        map.set(tag, { tag, total: 0, correct: 0, delta: 0 });
      }
      const s = map.get(tag)!;
      s.total++;
      if (a.isCorrect) s.correct++;
      s.delta += delta;
    }
  }
  return [...map.values()].sort((a, b) => b.total - a.total);
}
