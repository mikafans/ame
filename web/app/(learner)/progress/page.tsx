"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import CircularProgress from "@mui/material/CircularProgress";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import ToggleButton from "@mui/material/ToggleButton";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Paper from "@mui/material/Paper";
import Button from "@mui/material/Button";
import Chip from "@mui/material/Chip";
import Divider from "@mui/material/Divider";
import Tooltip from "@mui/material/Tooltip";
import IconButton from "@mui/material/IconButton";
import LinearProgress from "@mui/material/LinearProgress";
import { useTheme } from "@mui/material/styles";

// Icons
import HistoryOutlinedIcon from "@mui/icons-material/HistoryOutlined";
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";
import OpenInNewOutlinedIcon from "@mui/icons-material/OpenInNewOutlined";
import CheckCircleOutlinedIcon from "@mui/icons-material/CheckCircleOutlined";
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import TrendingUpIcon from "@mui/icons-material/TrendingUp";
import AssignmentTurnedInOutlinedIcon from "@mui/icons-material/AssignmentTurnedInOutlined";
import CalendarMonthOutlinedIcon from "@mui/icons-material/CalendarMonthOutlined";

import { formatScore, formatDuration, formatDateTime } from "@/utils/format";
import { PageShell } from "@/components/PageShell";
import { APP_VERSION } from "@/version";

type WindowType = "4w" | "all";

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
  agent_active_count?: number;
  agent_graded_attempts?: number;
  agent_curated_assessments?: number;
}

interface Attempt {
  id: string;
  is_correct: boolean;
  score: number;
  user_tag_deltas: Record<string, number>;
  created_at: string;
  question_id: string;
  grader_notes?: string | null;
}

interface SessionSummary {
  id: string;
  kind: string;
  status: string;
  assessmentId?: string | null;
  assessmentTitle?: string | null;
  pointsAwarded?: number | null;
  maxPoints?: number | null;
  startedAt: string;
  finishedAt?: string | null;
  attemptNumber: number;
  totalAttempts: number;
}

interface AgentSummary {
  id: string;
  label: string;
  createdAt: string;
  focusTags: string[];
  currentGoal?: string | null;
  nextTarget?: string | null;
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
  const router = useRouter();
  const { user } = useAuth();
  const [win, setWin] = useState<WindowType>("4w");
  const [stats, setStats] = useState<StatsResponse | null>(null);
  const [attempts, setAttempts] = useState<Attempt[]>([]);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [tagNames, setTagNames] = useState<Record<string, string>>({});
  const [statsLoading, setStatsLoading] = useState(true);
  const [attemptsLoading, setAttemptsLoading] = useState(true);
  const [sessionsLoading, setSessionsLoading] = useState(true);
  const [agentsLoading, setAgentsLoading] = useState(true);

  useEffect(() => {
    setStatsLoading(true);
    const apiWindow = win === "4w" ? "last30d" : "all";
    api
      .GET(
        "/v1/me/stats" as never,
        { params: { query: { window: apiWindow } } } as never,
      )
      .then(({ data }: { data?: StatsResponse }) => {
        if (data) setStats(data);
      })
      .catch(console.error)
      .finally(() => setStatsLoading(false));
  }, [win]);

  useEffect(() => {
    setAttemptsLoading(true);
    setSessionsLoading(true);
    setAgentsLoading(true);

    Promise.all([
      api
        .GET(
          "/v1/me/attempts" as never,
          { params: { query: { limit: 50 } } } as never,
        )
        .then(
          ({ data }: { data?: { attempts: Attempt[]; total: number } }) =>
            data?.attempts ?? [],
        ),
      api
        .GET("/v1/tags" as never, {} as never)
        .then(({ data }: { data?: { id: string; name: string }[] }) =>
          Array.isArray(data) ? data : [],
        ),
      api
        .GET(
          "/v1/sessions" as never,
          { params: { query: { limit: 10 } } } as never,
        )
        .then(
          ({
            data,
          }: {
            data?: { sessions: SessionSummary[]; total: number };
          }) => data?.sessions ?? [],
        ),
      api
        .GET("/v1/me/agents" as never, {} as never)
        .then(
          ({ data }: { data?: { agents: AgentSummary[] } }) =>
            data?.agents ?? [],
        ),
    ])
      .then(([fetchedAttempts, tags, fetchedSessions, fetchedAgents]) => {
        setAttempts(fetchedAttempts);
        setSessions(fetchedSessions);
        setAgents(fetchedAgents);

        const lookup: Record<string, string> = {};
        for (const t of tags) lookup[t.id] = t.name;
        setTagNames(lookup);
      })
      .catch(console.error)
      .finally(() => {
        setAttemptsLoading(false);
        setSessionsLoading(false);
        setAgentsLoading(false);
      });
  }, []);

  const filteredAttempts = filterByWindow(attempts, win);
  const weeklyAvgs = computeWeeklyAverages(filteredAttempts);
  const tagAvgs = computeTagAverages(filteredAttempts, tagNames);
  const topTags = tagAvgs.sort((a, b) => b.count - a.count).slice(0, 6);

  const windowAvgScore = filteredAttempts.length
    ? filteredAttempts.reduce((s, a) => s + a.score, 0) /
      filteredAttempts.length
    : null;

  return (
    <PageShell
      kicker="All courses"
      title="Progress dashboard"
      subtitle={`AME v${APP_VERSION}`}
      action={
        <ToggleButtonGroup
          value={win}
          exclusive
          onChange={(_, v) => v && setWin(v)}
          size="small"
        >
          <ToggleButton value="4w">4w</ToggleButton>
          <ToggleButton value="all">All</ToggleButton>
        </ToggleButtonGroup>
      }
    >
      {/* 1. Core Metrics Grid */}
      {statsLoading ? (
        <Typography color="text.secondary" sx={{ mb: 3 }}>
          Loading stats…
        </Typography>
      ) : stats ? (
        <Stack
          direction={{ xs: "column", sm: "row" }}
          spacing={2}
          sx={{ mb: 4 }}
        >
          {[
            {
              label: "Avg score",
              value:
                windowAvgScore !== null
                  ? formatScore(windowAvgScore)
                  : formatScore(stats.avg_score),
              desc: "Across all subjects",
            },
            {
              label: "Total Attempts",
              value: String(stats.attempts_total),
              desc: `${stats.attempts_this_week} this week`,
            },
            {
              label: "Study Time",
              value: formatDuration(stats.hours_spent ?? 0),
              desc: "Total duration",
            },
            {
              label: "Practice Streak",
              value: `${stats.current_streak ?? 0}d`,
              desc: `Best streak: ${stats.best_streak ?? 0}d`,
            },
          ].map(({ label, value, desc }) => (
            <Card
              key={label}
              variant="outlined"
              sx={{ flex: 1, position: "relative", overflow: "hidden" }}
            >
              <CardContent>
                <Typography
                  variant="h4"
                  sx={{ fontWeight: 600, mb: 0.5, letterSpacing: -0.5 }}
                >
                  {value}
                </Typography>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{
                    textTransform: "uppercase",
                    letterSpacing: 1,
                    fontWeight: 500,
                    display: "block",
                  }}
                >
                  {label}
                </Typography>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ fontSize: 11, mt: 0.5, display: "block" }}
                >
                  {desc}
                </Typography>
              </CardContent>
            </Card>
          ))}
        </Stack>
      ) : null}

      {/* 2. Main content: Two-column grid */}
      <Box
        sx={{
          display: "grid",
          gridTemplateColumns: { xs: "1fr", md: "1.4fr 1fr" },
          gap: 3.5,
          alignItems: "start",
        }}
      >
        {/* Left Column: Charts and Logs */}
        <Stack spacing={3.5} sx={{ minWidth: 0 }}>
          {/* Score Trend */}
          <Card variant="outlined">
            <CardContent>
              <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 0.5 }}>
                Score trend
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
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
                  <Typography color="text.secondary">
                    No data available for the trend
                  </Typography>
                )}
              </Box>
            </CardContent>
          </Card>

          {/* Recent Assessments (Sessions) */}
          <Card variant="outlined">
            <Box
              sx={{
                px: 2.5,
                py: 2,
                borderBottom: 1,
                borderColor: "divider",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <Stack direction="row" spacing={1.5} alignItems="center">
                <AssignmentTurnedInOutlinedIcon color="primary" />
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                  Recent Assessments Taken
                </Typography>
              </Stack>
              <Button
                size="small"
                variant="text"
                onClick={() => router.push("/results")}
              >
                View History
              </Button>
            </Box>

            {sessionsLoading ? (
              <Box sx={{ p: 4, display: "flex", justifyContent: "center" }}>
                <CircularProgress size={24} />
              </Box>
            ) : sessions.length === 0 ? (
              <Box sx={{ p: 4, textAlign: "center" }}>
                <Typography variant="body2" color="text.secondary">
                  No assessments taken yet. Practice more to see details here.
                </Typography>
              </Box>
            ) : (
              <TableContainer
                component={Paper}
                variant="outlined"
                sx={{ border: "none", borderRadius: 0 }}
              >
                <Table size="small">
                  <TableHead sx={{ bgcolor: "action.hover" }}>
                    <TableRow>
                      <TableCell sx={{ py: 1.5 }}>Assessment Title</TableCell>
                      <TableCell sx={{ py: 1.5 }}>Mode</TableCell>
                      <TableCell sx={{ py: 1.5 }}>Date Finished</TableCell>
                      <TableCell sx={{ py: 1.5 }} align="right">
                        Score
                      </TableCell>
                      <TableCell sx={{ py: 1.5 }} align="center">
                        Actions
                      </TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {sessions.slice(0, 5).map((session) => (
                      <TableRow key={session.id} hover>
                        <TableCell sx={{ py: 1.25, fontWeight: 500 }}>
                          {session.assessmentTitle ||
                            `Practice Session (${session.id.slice(0, 8)})`}
                        </TableCell>
                        <TableCell sx={{ py: 1.25 }}>
                          <Chip
                            label={session.kind}
                            size="small"
                            color={
                              session.kind === "exam" ? "secondary" : "default"
                            }
                            sx={{
                              textTransform: "capitalize",
                              height: 20,
                              fontSize: 11,
                            }}
                          />
                        </TableCell>
                        <TableCell sx={{ py: 1.25 }}>
                          {session.finishedAt
                            ? formatDateTime(session.finishedAt)
                            : "-"}
                        </TableCell>
                        <TableCell sx={{ py: 1.25 }} align="right">
                          {session.pointsAwarded !== null &&
                          session.pointsAwarded !== undefined &&
                          session.maxPoints !== null &&
                          session.maxPoints !== undefined &&
                          session.maxPoints > 0 ? (
                            <Typography
                              variant="body2"
                              sx={{ fontWeight: 600 }}
                            >
                              {session.pointsAwarded} / {session.maxPoints} (
                              {Math.round(
                                (session.pointsAwarded / session.maxPoints) *
                                  100,
                              )}
                              %)
                            </Typography>
                          ) : (
                            "-"
                          )}
                        </TableCell>
                        <TableCell sx={{ py: 1.25 }} align="center">
                          <Tooltip title="View results and rubrics">
                            <IconButton
                              size="small"
                              color="primary"
                              onClick={() =>
                                router.push(`/sessions/${session.id}/results`)
                              }
                            >
                              <OpenInNewOutlinedIcon sx={{ fontSize: 18 }} />
                            </IconButton>
                          </Tooltip>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            )}
          </Card>

          {/* Recent Individual Question Attempts */}
          <Card variant="outlined">
            <Box
              sx={{
                px: 2.5,
                py: 2,
                borderBottom: 1,
                borderColor: "divider",
                display: "flex",
                alignItems: "center",
                gap: 1.5,
              }}
            >
              <HistoryOutlinedIcon color="primary" />
              <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                Recent Question Attempts
              </Typography>
            </Box>

            {attemptsLoading ? (
              <Box sx={{ p: 4, display: "flex", justifyContent: "center" }}>
                <CircularProgress size={24} />
              </Box>
            ) : attempts.length === 0 ? (
              <Box sx={{ p: 4, textAlign: "center" }}>
                <Typography variant="body2" color="text.secondary">
                  No individual question attempts logged.
                </Typography>
              </Box>
            ) : (
              <Stack divider={<Divider />}>
                {attempts.slice(0, 5).map((attempt) => {
                  const activeTags = Object.keys(attempt.user_tag_deltas)
                    .map((tagId) => tagNames[tagId])
                    .filter(Boolean);

                  return (
                    <Box
                      key={attempt.id}
                      sx={{
                        p: 2.5,
                        display: "flex",
                        gap: 2,
                        alignItems: "flex-start",
                      }}
                    >
                      {attempt.is_correct ? (
                        <CheckCircleOutlinedIcon
                          color="success"
                          sx={{ mt: 0.25 }}
                        />
                      ) : (
                        <CancelOutlinedIcon color="error" sx={{ mt: 0.25 }} />
                      )}
                      <Box sx={{ flex: 1, minWidth: 0 }}>
                        <Stack
                          direction="row"
                          spacing={1}
                          alignItems="center"
                          sx={{ flexWrap: "wrap", gap: 0.5, mb: 1 }}
                        >
                          <Typography
                            variant="body2"
                            color="text.secondary"
                            sx={{ fontFamily: "monospace", fontSize: 12 }}
                          >
                            Question {attempt.question_id.slice(0, 8)}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            • {formatDateTime(attempt.created_at)}
                          </Typography>
                          {activeTags.map((name) => (
                            <Chip
                              key={name}
                              label={name}
                              size="small"
                              variant="outlined"
                              sx={{ height: 18, fontSize: 10 }}
                            />
                          ))}
                        </Stack>
                        <Typography
                          variant="body2"
                          sx={{
                            fontWeight: 500,
                            color: "text.primary",
                            mb: 0.5,
                          }}
                        >
                          Score: {attempt.score}
                        </Typography>
                        {attempt.grader_notes && (
                          <Typography
                            variant="caption"
                            sx={{
                              display: "block",
                              p: 1,
                              bgcolor: "action.hover",
                              borderRadius: 1,
                              borderLeft: 3,
                              borderColor: attempt.is_correct
                                ? "success.main"
                                : "error.main",
                              color: "text.secondary",
                              mt: 0.5,
                            }}
                          >
                            <strong>Grader Feedback:</strong>{" "}
                            {attempt.grader_notes}
                          </Typography>
                        )}
                      </Box>
                    </Box>
                  );
                })}
              </Stack>
            )}
          </Card>
        </Stack>

        {/* Right Column: Subject breakdown and Agents */}
        <Stack spacing={3.5}>
          {/* Performance By Subject */}
          <Card variant="outlined">
            <CardContent>
              <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 0.5 }}>
                By subject
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                Average score, {win === "4w" ? "last 4 weeks" : "all time"}
              </Typography>
              <Box
                sx={{
                  minHeight: 200,
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
                  <Typography color="text.secondary">
                    No topic averages computed yet
                  </Typography>
                )}
              </Box>
            </CardContent>
          </Card>

          {/* Active AI Agents */}
          <Card variant="outlined">
            <Box
              sx={{
                px: 2.5,
                py: 2,
                borderBottom: 1,
                borderColor: "divider",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <Stack direction="row" spacing={1.5} alignItems="center">
                <SmartToyOutlinedIcon color="secondary" />
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                  Active Course Agents
                </Typography>
              </Stack>
              <Button
                size="small"
                variant="text"
                color="secondary"
                onClick={() => router.push("/agent")}
              >
                Manage Keys
              </Button>
            </Box>

            {agentsLoading ? (
              <Box sx={{ p: 4, display: "flex", justifyContent: "center" }}>
                <CircularProgress size={24} />
              </Box>
            ) : agents.length === 0 ? (
              <Box sx={{ p: 4, textAlign: "center" }}>
                <Typography variant="body2" color="text.secondary">
                  No active agents configured. Create an agent to start
                  generating automated assessments.
                </Typography>
              </Box>
            ) : (
              <Stack divider={<Divider />} sx={{ p: 1 }}>
                {agents.map((agent) => (
                  <Box key={agent.id} sx={{ p: 2 }}>
                    <Typography
                      variant="subtitle2"
                      sx={{
                        fontWeight: 600,
                        display: "flex",
                        alignItems: "center",
                        gap: 1,
                      }}
                    >
                      {agent.label}
                      <Chip
                        label="Active"
                        color="success"
                        variant="outlined"
                        size="small"
                        sx={{ height: 16, fontSize: 9 }}
                      />
                    </Typography>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ display: "block", mt: 0.25, mb: 1 }}
                    >
                      Created: {formatDateTime(agent.createdAt)}
                    </Typography>

                    {agent.focusTags && agent.focusTags.length > 0 && (
                      <Box sx={{ mb: 1 }}>
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ display: "block", fontWeight: 600, mb: 0.5 }}
                        >
                          Focus Subjects:
                        </Typography>
                        <Stack
                          direction="row"
                          spacing={0.5}
                          sx={{ flexWrap: "wrap", gap: 0.5 }}
                        >
                          {agent.focusTags.map((tag) => (
                            <Chip
                              key={tag}
                              label={tag}
                              size="small"
                              sx={{ height: 18, fontSize: 10 }}
                            />
                          ))}
                        </Stack>
                      </Box>
                    )}

                    {(agent.currentGoal || agent.nextTarget) && (
                      <Box
                        sx={{
                          mt: 1,
                          p: 1,
                          bgcolor: "action.hover",
                          borderRadius: 1,
                        }}
                      >
                        {agent.currentGoal && (
                          <Typography
                            variant="caption"
                            sx={{
                              display: "block",
                              mb: 0.5,
                              color: "text.primary",
                            }}
                          >
                            <strong>Current Goal:</strong> {agent.currentGoal}
                          </Typography>
                        )}
                        {agent.nextTarget && (
                          <Typography
                            variant="caption"
                            sx={{ display: "block", color: "text.secondary" }}
                          >
                            <strong>Next Target:</strong> {agent.nextTarget}
                          </Typography>
                        )}
                      </Box>
                    )}
                  </Box>
                ))}
              </Stack>
            )}
          </Card>

          {/* Agent Insights / Platform Statistics */}
          {stats && (
            <Card variant="outlined">
              <Box
                sx={{
                  px: 2.5,
                  py: 2,
                  borderBottom: 1,
                  borderColor: "divider",
                  display: "flex",
                  alignItems: "center",
                  gap: 1.5,
                }}
              >
                <TrendingUpIcon color="secondary" />
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                  Agent Activity Metrics
                </Typography>
              </Box>
              <CardContent sx={{ py: 1 }}>
                <Stack spacing={2} sx={{ my: 1 }}>
                  {[
                    {
                      label: "Active Agent Count",
                      value: stats.agent_active_count ?? 0,
                    },
                    {
                      label: "Agent-Graded attempts",
                      value: stats.agent_graded_attempts ?? 0,
                    },
                    {
                      label: "Agent-Curated assessments",
                      value: stats.agent_curated_assessments ?? 0,
                    },
                  ].map(({ label, value }) => (
                    <Box
                      key={label}
                      sx={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <Typography variant="body2" color="text.secondary">
                        {label}
                      </Typography>
                      <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                        {value}
                      </Typography>
                    </Box>
                  ))}
                </Stack>
              </CardContent>
            </Card>
          )}
        </Stack>
      </Box>
    </PageShell>
  );
}

function ScoreLineChart({ data }: { data: WeeklyAvg[] }) {
  const theme = useTheme();
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
      {/* Grid lines */}
      {[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
        const y = padding + innerHeight - ratio * innerHeight;
        return (
          <g key={ratio}>
            <line
              x1={padding}
              y1={y}
              x2={width - padding}
              y2={y}
              stroke={theme.palette.divider}
              strokeDasharray="4 4"
              strokeWidth={1}
            />
            <text
              x={padding - 8}
              y={y + 4}
              textAnchor="end"
              fill={theme.palette.text.secondary}
              style={{ fontSize: 10, fontFamily: "sans-serif" }}
            >
              {Math.round(ratio * 100)}%
            </text>
          </g>
        );
      })}

      {/* Line path */}
      <polyline
        points={points.map((p) => `${p.x},${p.y}`).join(" ")}
        fill="none"
        stroke={theme.palette.primary.main}
        strokeWidth={2.5}
      />

      {/* Data circles */}
      {points.map((p, i) => (
        <g key={i}>
          <circle cx={p.x} cy={p.y} r={4} fill={theme.palette.primary.main} />
          <text
            x={p.x}
            y={p.y - 8}
            textAnchor="middle"
            fill={theme.palette.text.primary}
            style={{ fontSize: 9, fontFamily: "monospace", fontWeight: 600 }}
          >
            W{data[i].week}
          </text>
        </g>
      ))}
    </svg>
  );
}

function BarChart({ data }: { data: TagAvg[] }) {
  return (
    <Box
      sx={{
        display: "flex",
        flexDirection: "column",
        gap: 1.75,
        width: "100%",
      }}
    >
      {data.map((item) => {
        const pctText = formatScore(item.score);
        const pct = Math.round(item.score * 100);
        return (
          <Box
            key={item.tag}
            sx={{ display: "flex", gap: 1.5, alignItems: "center" }}
          >
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{
                width: 90,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                fontWeight: 500,
              }}
            >
              {item.tag}
            </Typography>
            <Box
              sx={{
                flex: 1,
                height: 12,
                bgcolor: "action.hover",
                borderRadius: 1,
                overflow: "hidden",
                border: 1,
                borderColor: "divider",
              }}
            >
              <Box
                sx={{
                  height: "100%",
                  width: `${pct}%`,
                  bgcolor:
                    pct >= 80
                      ? "success.main"
                      : pct >= 50
                        ? "warning.main"
                        : "error.main",
                  borderRadius: 1,
                  transition: "width 0.4s ease",
                }}
              />
            </Box>
            <Typography
              variant="caption"
              color="text.primary"
              sx={{ width: 40, textAlign: "right", fontWeight: 600 }}
            >
              {pctText}
            </Typography>
          </Box>
        );
      })}
    </Box>
  );
}

function filterByWindow(attempts: Attempt[], win: WindowType): Attempt[] {
  if (win === "all") return attempts;
  const cutoff = Date.now() - 4 * 7 * 24 * 60 * 60 * 1000;
  return attempts.filter((a) => new Date(a.created_at).getTime() >= cutoff);
}

function yearWeekKey(date: Date): string {
  const d = new Date(
    Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()),
  );
  const dayNum = d.getUTCDay() || 7;
  d.setUTCDate(d.getUTCDate() + 4 - dayNum);
  const year = d.getUTCFullYear();
  const yearStart = new Date(Date.UTC(year, 0, 1));
  const week = Math.ceil(
    ((d.getTime() - yearStart.getTime()) / 86400000 + 1) / 7,
  );
  return `${year}-${String(week).padStart(2, "0")}`;
}

function computeWeeklyAverages(attempts: Attempt[]): WeeklyAvg[] {
  if (attempts.length === 0) return [];
  const weekMap = new Map<string, { sum: number; count: number }>();
  for (const a of attempts) {
    const key = yearWeekKey(new Date(a.created_at));
    const entry = weekMap.get(key) ?? { sum: 0, count: 0 };
    entry.sum += a.score;
    entry.count++;
    weekMap.set(key, entry);
  }
  return Array.from(weekMap.entries())
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([week, data]) => ({
      week: parseInt(week.split("-")[1]),
      score: data.sum / data.count,
    }));
}

function computeTagAverages(
  attempts: Attempt[],
  tagNames: Record<string, string> = {},
): TagAvg[] {
  const tagMap = new Map<string, { sum: number; count: number }>();
  for (const a of attempts) {
    for (const tagId of Object.keys(a.user_tag_deltas)) {
      const name = tagNames[tagId];
      if (!name) continue; // skip unknown tag IDs
      const entry = tagMap.get(name) ?? { sum: 0, count: 0 };
      entry.sum += a.score;
      entry.count++;
      tagMap.set(name, entry);
    }
  }
  return Array.from(tagMap.entries()).map(([tag, data]) => ({
    tag,
    score: data.sum / data.count,
    count: data.count,
  }));
}
