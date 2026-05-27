"use client";

import { useState, useEffect } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import CircularProgress from "@mui/material/CircularProgress";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import ToggleButton from "@mui/material/ToggleButton";

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
  const [tagNames, setTagNames] = useState<Record<string, string>>({});
  const [statsLoading, setStatsLoading] = useState(true);
  const [attemptsLoading, setAttemptsLoading] = useState(true);

  useEffect(() => {
    if (!token) return;
    setStatsLoading(true);
    makeClient(token)
      .GET("/v1/me/stats" as never, { params: { query: { window } } } as never)
      .then(({ data }: { data?: StatsResponse }) => {
        if (data) setStats(data);
      })
      .catch(console.error)
      .finally(() => setStatsLoading(false));
  }, [token, window]);

  useEffect(() => {
    if (!token) return;
    setAttemptsLoading(true);
    Promise.all([
      makeClient(token)
        .GET(
          "/v1/me/attempts" as never,
          { params: { query: { limit: 200 } } } as never,
        )
        .then(
          ({ data }: { data?: { attempts: Attempt[]; total: number } }) =>
            data?.attempts ?? [],
        ),
      makeClient(token)
        .GET("/v1/tags" as never, {} as never)
        .then(({ data }: { data?: { id: string; name: string }[] }) =>
          Array.isArray(data) ? data : [],
        ),
    ])
      .then(([fetchedAttempts, tags]) => {
        setAttempts(fetchedAttempts);
        const lookup: Record<string, string> = {};
        for (const t of tags) lookup[t.id] = t.name;
        setTagNames(lookup);
      })
      .catch(console.error)
      .finally(() => setAttemptsLoading(false));
  }, [token]);

  const weeklyAvgs = computeWeeklyAverages(attempts);
  const tagAvgs = computeTagAverages(attempts, tagNames);
  const topTags = tagAvgs.sort((a, b) => b.count - a.count).slice(0, 6);

  return (
    <Box sx={{ p: 4 }}>
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          mb: 3,
        }}
      >
        <Box>
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              letterSpacing: 1.4,
              textTransform: "uppercase",
              display: "block",
            }}
          >
            All courses
          </Typography>
          <Typography variant="h4" sx={{ fontWeight: 500 }}>
            Progress dashboard
          </Typography>
        </Box>
        <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
          <ToggleButtonGroup
            value={window}
            exclusive
            onChange={(_, v) => v && setWindow(v)}
            size="small"
          >
            <ToggleButton value="4w">4w</ToggleButton>
            <ToggleButton value="12w">12w</ToggleButton>
            <ToggleButton value="all">All</ToggleButton>
          </ToggleButtonGroup>
          <Button variant="outlined" size="small">
            Export
          </Button>
        </Stack>
      </Box>

      {statsLoading ? (
        <Typography color="text.secondary" sx={{ mb: 3 }}>
          Loading…
        </Typography>
      ) : stats ? (
        <Stack direction="row" spacing={2} sx={{ mb: 3 }}>
          {[
            {
              label: "Avg score",
              value: `${Math.round(stats.avg_score * 100)}%`,
            },
            { label: "Attempts", value: String(stats.attempts_total ?? 0) },
            { label: "Hours", value: `${stats.hours_spent ?? 0}h` },
            { label: "Streak", value: `${stats.current_streak ?? 0}d` },
          ].map(({ label, value }) => (
            <Card key={label} variant="outlined" sx={{ flex: 1 }}>
              <CardContent>
                <Typography variant="h5" sx={{ fontWeight: 500 }}>
                  {value}
                </Typography>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ textTransform: "uppercase", letterSpacing: 1 }}
                >
                  {label}
                </Typography>
              </CardContent>
            </Card>
          ))}
        </Stack>
      ) : null}

      <Stack direction="row" spacing={2}>
        <Card variant="outlined" sx={{ flex: 1 }}>
          <CardContent>
            <Typography variant="subtitle1" sx={{ fontWeight: 500 }}>
              Score trend
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Weekly rolling average across all subjects
            </Typography>
            <Box
              sx={{
                height: 200,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {attemptsLoading ? (
                <CircularProgress size={24} />
              ) : weeklyAvgs.length > 0 ? (
                <ScoreLineChart data={weeklyAvgs} />
              ) : (
                <Typography color="text.secondary">No data</Typography>
              )}
            </Box>
          </CardContent>
        </Card>
        <Card variant="outlined" sx={{ flex: 1 }}>
          <CardContent>
            <Typography variant="subtitle1" sx={{ fontWeight: 500 }}>
              By subject
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Average score, last N weeks
            </Typography>
            <Box
              sx={{
                height: 200,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {attemptsLoading ? (
                <CircularProgress size={24} />
              ) : topTags.length > 0 ? (
                <BarChart data={topTags} />
              ) : (
                <Typography color="text.secondary">No data</Typography>
              )}
            </Box>
          </CardContent>
        </Card>
      </Stack>
    </Box>
  );
}

function ScoreLineChart({ data }: { data: WeeklyAvg[] }) {
  const width = 600;
  const height = 180;
  const padding = 40;
  const innerWidth = width - padding * 2;
  const innerHeight = height - padding * 2;

  const points = data.map((d, i) => {
    const x = (i / (data.length - 1 || 1)) * innerWidth + padding;
    const y = padding + innerHeight - d.score * innerHeight;
    return { x, y };
  });

  return (
    <svg
      width="100%"
      viewBox={`0 0 ${width} ${height}`}
      style={{ overflow: "visible" }}
    >
      <polyline
        points={points.map((p) => `${p.x},${p.y}`).join(" ")}
        fill="none"
        stroke="#1976d2"
        strokeWidth={2}
      />
      {points.map((p, i) => (
        <circle key={i} cx={p.x} cy={p.y} r={3} fill="#1976d2" opacity="0.8" />
      ))}
      <line
        x1={padding}
        y1={padding + innerHeight}
        x2={width - padding}
        y2={padding + innerHeight}
        stroke="#e0e0e0"
        strokeWidth={1}
      />
    </svg>
  );
}

function BarChart({ data }: { data: TagAvg[] }) {
  return (
    <Box
      sx={{ display: "flex", flexDirection: "column", gap: 1, width: "100%" }}
    >
      {data.map((item) => {
        const pct = Math.round(item.score * 100);
        return (
          <Box
            key={item.tag}
            sx={{ display: "flex", gap: 1, alignItems: "center" }}
          >
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{
                width: 100,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {item.tag}
            </Typography>
            <Box
              sx={{
                flex: 1,
                height: 20,
                bgcolor: "action.hover",
                borderRadius: 1,
                overflow: "hidden",
              }}
            >
              <Box
                sx={{
                  height: "100%",
                  width: `${pct}%`,
                  bgcolor: "primary.main",
                  borderRadius: 1,
                  transition: "width 0.4s ease",
                }}
              />
            </Box>
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ width: 32, textAlign: "right" }}
            >
              {pct}%
            </Typography>
          </Box>
        );
      })}
    </Box>
  );
}

function computeWeeklyAverages(attempts: Attempt[]): WeeklyAvg[] {
  if (attempts.length === 0) return [];
  const weekMap = new Map<number, { sum: number; count: number }>();
  for (const a of attempts) {
    const date = new Date(a.created_at);
    const week = getISO8601Week(date);
    if (!weekMap.has(week)) weekMap.set(week, { sum: 0, count: 0 });
    const entry = weekMap.get(week)!;
    entry.sum += a.score;
    entry.count++;
  }
  return Array.from(weekMap.entries())
    .sort((a, b) => a[0] - b[0])
    .slice(-12)
    .map(([week, data]) => ({ week, score: data.sum / data.count }));
}

function computeTagAverages(
  attempts: Attempt[],
  tagNames: Record<string, string> = {},
): TagAvg[] {
  const tagMap = new Map<string, { sum: number; count: number }>();
  for (const a of attempts) {
    for (const tagId of Object.keys(a.user_tag_deltas)) {
      const tag = tagNames[tagId] ?? tagId;
      if (!tagMap.has(tag)) tagMap.set(tag, { sum: 0, count: 0 });
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
