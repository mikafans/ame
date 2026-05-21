"use client";

import { useState, useEffect } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { Card, Stat, Button } from "@/components/ui";

type WindowType = "4w" | "12w" | "all";

interface StatsResponse {
  avg_score: number;
  avg_score_delta: number;
  attempts_total: number;
  attempts_this_week: number;
  hours_spent: number;
  hours_spent_delta: number;
  current_streak: number;
  best_streak: number;
  mastered_topics: number;
  mastered_topics_total: number;
  mastered_topics_delta_since: string;
}

interface Attempt {
  id: string;
  is_correct: boolean;
  score: number;
  user_tag_deltas: Record<string, number>;
  created_at: string;
}

interface WeeklyAvg {
  week: number;
  score: number;
}

interface TagAvg {
  tag: string;
  score: number;
  count: number;
}

export default function ProgressPage() {
  const { token } = useAuth();
  const [window, setWindow] = useState<WindowType>("12w");
  const [stats, setStats] = useState<StatsResponse | null>(null);
  const [attempts, setAttempts] = useState<Attempt[]>([]);
  const [statsLoading, setStatsLoading] = useState(true);
  const [attemptsLoading, setAttemptsLoading] = useState(true);

  useEffect(() => {
    if (!token) return;

    setStatsLoading(true);
    makeClient(token)
      .GET(
        "/v1/me/stats" as never,
        {
          params: { query: { window } },
        } as never,
      )
      .then(({ data }: { data?: StatsResponse }) => {
        if (data) setStats(data);
      })
      .catch(console.error)
      .finally(() => setStatsLoading(false));
  }, [token, window]);

  useEffect(() => {
    if (!token) return;

    setAttemptsLoading(true);
    makeClient(token)
      .GET(
        "/v1/me/attempts" as never,
        {
          params: { query: { limit: 200 } },
        } as never,
      )
      .then(({ data }: { data?: { attempts: Attempt[]; total: number } }) => {
        if (data?.attempts) setAttempts(data.attempts);
      })
      .catch(console.error)
      .finally(() => setAttemptsLoading(false));
  }, [token]);

  const weeklyAvgs = computeWeeklyAverages(attempts);
  const tagAvgs = computeTagAverages(attempts);
  const topTags = tagAvgs.sort((a, b) => b.count - a.count).slice(0, 6);

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Page header */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          marginBottom: 24,
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
              marginBottom: 6,
            }}
          >
            ALL COURSES
          </div>
          <h2
            style={{
              margin: 0,
              fontFamily: "var(--serif)",
              fontSize: 28,
              fontWeight: 500,
              letterSpacing: -0.5,
              color: "var(--text)",
            }}
          >
            Progress dashboard
          </h2>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <div
            style={{
              display: "flex",
              gap: 2,
              background: "var(--surface-2)",
              borderRadius: 6,
              padding: 2,
            }}
          >
            {(["4w", "12w", "all"] as WindowType[]).map((w) => (
              <button
                key={w}
                onClick={() => setWindow(w)}
                style={{
                  padding: "8px 12px",
                  fontSize: 13,
                  fontWeight: 500,
                  border: "1px solid transparent",
                  borderRadius: 4,
                  background: window === w ? "var(--accent)" : "transparent",
                  color: window === w ? "#0b1410" : "var(--text-2)",
                  cursor: "pointer",
                  transition: "background 120ms, color 120ms",
                }}
              >
                {w === "4w" ? "4w" : w === "12w" ? "12w" : "All"}
              </button>
            ))}
          </div>
          <Button variant="ghost" size="md" disabled>
            Export
          </Button>
        </div>
      </div>

      {/* Stats strip */}
      {statsLoading ? (
        <div
          style={{
            color: "var(--muted)",
            fontFamily: "var(--mono)",
            fontSize: 13,
            marginBottom: 24,
          }}
        >
          Loading…
        </div>
      ) : stats ? (
        <Card style={{ marginBottom: 24, padding: 0 }}>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(5, 1fr)",
              gap: 0,
            }}
          >
            {[
              {
                label: "AVG SCORE",
                value: `${(stats.avg_score * 100).toFixed(1)}%`,
                delta: `${stats.avg_score_delta >= 0 ? "+" : ""}${stats.avg_score_delta.toFixed(1)} vs prior`,
                positive: stats.avg_score_delta >= 0,
              },
              {
                label: "ATTEMPTS",
                value: stats.attempts_total,
                delta: `${stats.attempts_this_week} this week`,
              },
              {
                label: "HOURS SPENT",
                value: stats.hours_spent.toFixed(1),
                delta: `${stats.hours_spent_delta >= 0 ? "+" : ""}${stats.hours_spent_delta.toFixed(1)} vs avg`,
                positive: stats.hours_spent_delta >= 0,
              },
              {
                label: "CURRENT STREAK",
                value: `${stats.current_streak} d`,
                delta: `best: ${stats.best_streak} d`,
              },
              {
                label: "MASTERED TOPICS",
                value: `${stats.mastered_topics} / ${stats.mastered_topics_total}`,
                delta: `+${stats.mastered_topics_delta_since}`,
              },
            ].map((s, i) => (
              <div
                key={s.label}
                style={{
                  padding: "22px 24px",
                  borderRight: i < 4 ? "1px solid var(--border)" : "none",
                }}
              >
                <Stat
                  label={s.label}
                  value={s.value}
                  delta={s.delta}
                  deltaPositive={"positive" in s ? s.positive : undefined}
                />
              </div>
            ))}
          </div>
        </Card>
      ) : (
        <div
          style={{
            color: "var(--muted)",
            fontSize: 13,
            marginBottom: 24,
          }}
        >
          No data yet.
        </div>
      )}

      {/* Charts row */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1.5fr 1fr",
          gap: 18,
        }}
      >
        {/* Score trend chart */}
        <Card style={{ padding: 24 }}>
          <div style={{ marginBottom: 14 }}>
            <div
              style={{
                fontFamily: "var(--serif)",
                fontSize: 18,
                fontWeight: 500,
                letterSpacing: -0.2,
              }}
            >
              Score trend
            </div>
            <div
              style={{
                fontSize: 12,
                color: "var(--muted)",
                marginTop: 2,
              }}
            >
              Weekly rolling average across all subjects
            </div>
          </div>

          {attemptsLoading ? (
            <div
              style={{
                height: 220,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--muted)",
                fontSize: 13,
              }}
            >
              Loading…
            </div>
          ) : weeklyAvgs.length === 0 ? (
            <div
              style={{
                height: 220,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--muted)",
                fontSize: 13,
              }}
            >
              No data yet
            </div>
          ) : (
            <ScoreLineChart data={weeklyAvgs} />
          )}
        </Card>

        {/* By subject chart */}
        <Card style={{ padding: 24 }}>
          <div style={{ marginBottom: 14 }}>
            <div
              style={{
                fontFamily: "var(--serif)",
                fontSize: 18,
                fontWeight: 500,
                letterSpacing: -0.2,
              }}
            >
              By subject
            </div>
            <div
              style={{
                fontSize: 12,
                color: "var(--muted)",
                marginTop: 2,
              }}
            >
              Average score, last N weeks
            </div>
          </div>

          {attemptsLoading ? (
            <div
              style={{
                height: 232,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--muted)",
                fontSize: 13,
              }}
            >
              Loading…
            </div>
          ) : topTags.length === 0 ? (
            <div
              style={{
                height: 232,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--muted)",
                fontSize: 13,
              }}
            >
              No data yet
            </div>
          ) : (
            <BarChart data={topTags} />
          )}
        </Card>
      </div>
    </div>
  );
}

function ScoreLineChart({ data }: { data: WeeklyAvg[] }) {
  const width = 600;
  const height = 220;
  const padding = 40;
  const innerWidth = width - padding * 2;
  const innerHeight = height - padding * 2;

  const maxScore = 1;
  const minScore = 0;

  const points = data.map((d, i) => {
    const x = (i / (data.length - 1 || 1)) * innerWidth + padding;
    const y =
      padding +
      innerHeight -
      ((d.score - minScore) / (maxScore - minScore)) * innerHeight;
    return { x, y, score: d.score };
  });

  return (
    <svg width={width} height={height} style={{ overflow: "visible" }}>
      <polyline
        points={points.map((p) => `${p.x},${p.y}`).join(" ")}
        fill="none"
        stroke="var(--accent)"
        strokeWidth={2}
      />
      {points.map((p, i) => (
        <circle
          key={i}
          cx={p.x}
          cy={p.y}
          r={3}
          fill="var(--accent)"
          opacity="0.8"
        />
      ))}
      <line
        x1={padding}
        y1={padding + innerHeight}
        x2={width - padding}
        y2={padding + innerHeight}
        stroke="var(--border)"
        strokeWidth={1}
      />
      <line
        x1={padding}
        y1={padding}
        x2={padding}
        y2={padding + innerHeight}
        stroke="var(--border)"
        strokeWidth={1}
      />
    </svg>
  );
}

function BarChart({ data }: { data: TagAvg[] }) {
  const barHeight = 24;
  const gap = 8;
  const labelWidth = 120;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap }}>
      {data.map((item) => {
        const pct = Math.round(item.score * 100);
        return (
          <div
            key={item.tag}
            style={{ display: "flex", gap: 12, alignItems: "center" }}
          >
            <div
              style={{
                width: labelWidth,
                fontSize: 12,
                color: "var(--text-2)",
                fontFamily: "var(--mono)",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {item.tag}
            </div>
            <div
              style={{
                flex: 1,
                height: barHeight,
                background: "var(--surface-2)",
                borderRadius: 4,
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  height: "100%",
                  width: `${pct}%`,
                  background: "var(--accent)",
                  borderRadius: 4,
                  transition: "width 0.4s ease",
                }}
              />
            </div>
            <div
              style={{
                width: 36,
                textAlign: "right",
                fontSize: 12,
                fontFamily: "var(--mono)",
                color: "var(--text-2)",
              }}
            >
              {pct}%
            </div>
          </div>
        );
      })}
    </div>
  );
}

function computeWeeklyAverages(attempts: Attempt[]): WeeklyAvg[] {
  if (attempts.length === 0) return [];

  const weekMap = new Map<number, { sum: number; count: number }>();

  for (const a of attempts) {
    const date = new Date(a.created_at);
    const week = getISO8601Week(date);
    if (!weekMap.has(week)) {
      weekMap.set(week, { sum: 0, count: 0 });
    }
    const entry = weekMap.get(week)!;
    entry.sum += a.score;
    entry.count++;
  }

  const sorted = Array.from(weekMap.entries())
    .sort((a, b) => a[0] - b[0])
    .slice(-12);

  return sorted.map(([week, data]) => ({
    week,
    score: data.sum / data.count,
  }));
}

function computeTagAverages(attempts: Attempt[]): TagAvg[] {
  const tagMap = new Map<string, { sum: number; count: number }>();

  for (const a of attempts) {
    for (const tag of Object.keys(a.user_tag_deltas)) {
      if (!tagMap.has(tag)) {
        tagMap.set(tag, { sum: 0, count: 0 });
      }
      const entry = tagMap.get(tag)!;
      entry.sum += a.score;
      entry.count++;
    }
  }

  return Array.from(tagMap.entries()).map(([tag, data]) => ({
    tag,
    score: data.sum / data.count,
    count: data.count,
  }));
}

function getISO8601Week(date: Date): number {
  const d = new Date(
    Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()),
  );
  const dayNum = d.getUTCDay() || 7;
  d.setUTCDate(d.getUTCDate() + 4 - dayNum);
  const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
  return Math.ceil(((d.getTime() - yearStart.getTime()) / 86400000 + 1) / 7);
}
