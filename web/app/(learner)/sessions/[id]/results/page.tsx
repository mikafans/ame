"use client";

import { useState, useEffect, use } from "react";
import { formatDate } from "@/utils/format";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Divider from "@mui/material/Divider";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircle";
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined";

interface Answer {
  qid: string;
  correct: boolean;
  points: number;
  max: number;
  type: "mc" | "tf" | "short" | "essay" | "code";
  prompt: string;
  given: string;
  correctAnswer?: string;
  note: string;
  gradeStatus: string;
  explanation?: string;
}

interface ResultData {
  id: string;
  assessment_title: string | null;
  course: string | null;
  attempt_number: number | null;
  total_attempts: number | null;
  score: number;
  total: number;
  feedback?: string;
  answers: Answer[];
}

interface SessionSummary {
  id: string;
  pointsAwarded?: number;
  maxPoints?: number;
  startedAt: string;
  finishedAt?: string;
  attemptNumber: number;
  totalAttempts: number;
}

interface CohortHistogramBucket {
  bucket_start: number;
  bucket_end: number;
  count: number;
}

interface CohortStats {
  cohortAvg: number | null;
  percentile: number | null;
  completionRate: number | null;
  histogram: CohortHistogramBucket[];
}

export default function ResultsPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { user } = useAuth();
  const router = useRouter();
  const [data, setData] = useState<ResultData | null>(null);
  const [cohortStats, setCohortStats] = useState<CohortStats | null>(null);
  const [history, setHistory] = useState<SessionSummary[]>([]);

  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    api
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .then(({ data: d }: { data?: any }) => {
        if (!d?.session) return;
        const session = d.session;
        const questions: any[] = d.questions ?? [];
        const attempts: any[] = d.attempts ?? [];
        const byQuestion = new Map(
          attempts.map((a: any) => [a.question_id, a]),
        );
        const result = session.result ?? {};
        setData({
          id: session.id,
          assessment_title: session.assessment_title ?? null,
          course: session.course_title ?? null,
          attempt_number: null,
          total_attempts: null,
          score: result.points_awarded ?? 0,
          total: result.max_points ?? 0,
          answers: questions.map((q: any) => {
            const attempt = byQuestion.get(q.questionId);
            const r = attempt?.response as Record<string, unknown> | undefined;
            const p = attempt?.presentation as
              | Record<string, unknown>
              | undefined;
            const ca = attempt?.correct_answer as
              | Record<string, unknown>
              | undefined;

            let given = "";
            let correctAnswer = "";

            if (r) {
              if ("selected_position" in r) {
                const pos = r.selected_position as number;
                const label =
                  ["A", "B", "C", "D", "E", "F"][pos] ?? `Option ${pos + 1}`;
                const optionOrder = p?.option_order as number[] | undefined;
                if (optionOrder && q.options) {
                  const canonicalIdx = optionOrder[pos];
                  const optText = q.options[canonicalIdx]?.text ?? "";
                  given = `${label}: ${optText}`;
                } else {
                  given = label;
                }
              } else if ("answer" in r) {
                const ans = r.answer;
                given =
                  typeof ans === "boolean"
                    ? ans
                      ? "True"
                      : "False"
                    : String(ans ?? "");
              } else if ("body" in r) {
                given = String(r.body ?? "");
              } else if ("source" in r) {
                given = String(r.source ?? "");
              }
            }

            if (ca && !attempt?.is_correct) {
              if ("correct_index" in ca) {
                const cIdx = ca.correct_index as number;
                const optionOrder = p?.option_order as number[] | undefined;
                if (optionOrder) {
                  const pos = optionOrder.indexOf(cIdx);
                  const label =
                    ["A", "B", "C", "D", "E", "F"][pos] ?? `Option ${pos + 1}`;
                  const optText = q.options?.[cIdx]?.text ?? "";
                  correctAnswer = `${label}: ${optText}`;
                }
              } else if ("correct" in ca) {
                correctAnswer = ca.correct ? "True" : "False";
              } else if ("accepted" in ca) {
                correctAnswer = (ca.accepted as string[]).join(", ");
              } else if ("exemplar" in ca) {
                correctAnswer = String(ca.exemplar);
              }
            }

            const score = attempt?.score ?? 0;
            const points = Math.round(score * q.points);
            const status = attempt?.grade_status ?? "ungraded";
            return {
              qid: q.questionId,
              correct: attempt?.is_correct ?? false,
              points: points,
              max: q.points,
              type: q.kind,
              prompt: q.prompt,
              given,
              correctAnswer,
              gradeStatus: status,
              note:
                status === "pending_manual"
                  ? "Pending manual review"
                  : status === "graded"
                    ? ""
                    : "Not graded yet",
              explanation: q.explanation ?? undefined,
            };
          }),
        });
        const apiUrl =
          process.env.NEXT_PUBLIC_API_URL ??
          (typeof window !== "undefined"
            ? `http://${window.location.hostname}:28080`
            : "http://localhost:28080");

        if (session.assessment_id) {
          fetch(
            `${apiUrl}/v1/me/cohort-stats?assessmentId=${session.assessment_id}`,
            { credentials: "include" },
          )
            .then((r) => (r.ok ? r.json() : null))
            .then((cs: CohortStats | null) => {
              if (cs) setCohortStats(cs);
            })
            .catch(() => {});
        }
        // Attempt history for this assessment — populates "Attempt N of M"
        // and the list of previous attempts.
        const histQuery = session.assessment_id
          ? `assessmentId=${session.assessment_id}`
          : null;
        if (histQuery) {
          fetch(`${apiUrl}/v1/sessions?${histQuery}`, {
            credentials: "include",
          })
            .then((r) => (r.ok ? r.json() : null))
            .then((h: { sessions?: SessionSummary[] } | null) => {
              const sessions = h?.sessions ?? [];
              setHistory(sessions);
              const mine = sessions.find((s) => s.id === session.id);
              if (mine) {
                setData((prev) =>
                  prev
                    ? {
                        ...prev,
                        attempt_number: mine.attemptNumber,
                        total_attempts: mine.totalAttempts,
                      }
                    : prev,
                );
              }
            })
            .catch(() => {});
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [id]);

  if (loading) {
    return (
      <Box
        sx={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: "60vh",
        }}
      >
        <CircularProgress />
      </Box>
    );
  }

  if (!data)
    return (
      <Box sx={{ p: 4 }}>
        <Typography color="text.secondary">Results not found.</Typography>
      </Box>
    );

  const scorePercent =
    data.total > 0 ? Math.round((data.score / data.total) * 100) : 0;
  const passed = scorePercent >= 70;
  const answers = data.answers;

  return (
    <Box sx={{ p: 4, maxWidth: 800, mx: "auto" }}>
      <Typography variant="h5" sx={{ fontWeight: 500, mb: 0.5 }}>
        Quiz Results — {data.assessment_title || "Results"}
      </Typography>
      {data.attempt_number != null && data.total_attempts != null && (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
          Attempt {data.attempt_number} of {data.total_attempts}
        </Typography>
      )}
      {data.attempt_number == null && <Box sx={{ mb: 3 }} />}

      {/* Score card */}
      <Card variant="outlined" sx={{ mb: 3 }}>
        <CardContent>
          <Stack
            direction="row"
            sx={{ justifyContent: "space-between", alignItems: "center" }}
          >
            <Box>
              <Typography variant="h3" sx={{ fontWeight: 600 }}>
                {scorePercent}%
              </Typography>
              <Typography variant="body2" color="text.secondary">
                {data.score} / {data.total} points
              </Typography>
              {data.feedback && (
                <Typography
                  variant="body2"
                  color="text.secondary"
                  sx={{ mt: 1, fontStyle: "italic" }}
                >
                  {data.feedback}
                </Typography>
              )}
              {cohortStats?.cohortAvg != null && (
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ display: "block", mt: 0.5 }}
                >
                  Class average: {Math.round(cohortStats.cohortAvg * 100)}%
                  {cohortStats.percentile != null &&
                    ` · ${cohortStats.percentile}th percentile`}
                </Typography>
              )}
            </Box>
            <Chip
              icon={
                passed ? <CheckCircleOutlineIcon /> : <CancelOutlinedIcon />
              }
              label={passed ? "Passed" : "Not passed"}
              color={passed ? "success" : "error"}
              variant="outlined"
            />
          </Stack>
        </CardContent>
      </Card>

      {/* Answer review */}
      <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 2 }}>
        Answer review
      </Typography>
      <Stack spacing={1.5} sx={{ mb: 4 }}>
        {answers.map((a, i) => (
          <Card key={a.qid} variant="outlined">
            <CardContent>
              <Box
                sx={{ display: "flex", justifyContent: "space-between", mb: 1 }}
              >
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  Q{i + 1}. {a.prompt}
                </Typography>
                <Stack
                  direction="row"
                  spacing={0.75}
                  sx={{ alignItems: "center" }}
                >
                  {a.gradeStatus === "pending_manual" ? (
                    <Chip
                      label="Pending review"
                      size="small"
                      variant="outlined"
                    />
                  ) : (
                    <Chip
                      label={`${a.points}/${a.max}`}
                      size="small"
                      color={a.correct ? "success" : "error"}
                      variant="outlined"
                    />
                  )}
                </Stack>
              </Box>
              {a.type === "code" ? (
                <Box>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: "block", mb: 0.5 }}
                  >
                    Your submission
                  </Typography>
                  {a.given ? (
                    <Box
                      component="pre"
                      sx={{
                        m: 0,
                        p: 1.5,
                        borderRadius: 1,
                        bgcolor: "action.hover",
                        fontFamily: "monospace",
                        fontSize: 13,
                        lineHeight: 1.6,
                        overflowX: "auto",
                        whiteSpace: "pre-wrap",
                        wordBreak: "break-word",
                      }}
                    >
                      {a.given}
                    </Box>
                  ) : (
                    <Typography variant="caption" color="text.secondary">
                      No answer submitted
                    </Typography>
                  )}
                  {a.gradeStatus === "pending_manual" && (
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ display: "block", mt: 0.5, fontStyle: "italic" }}
                    >
                      Awaiting manual review — code isn’t auto-graded.
                    </Typography>
                  )}
                </Box>
              ) : (
                <Box>
                  <Typography variant="caption" color="text.secondary">
                    Your answer: {a.given || "—"}
                  </Typography>
                  {a.correctAnswer && (
                    <Typography
                      variant="caption"
                      color="error.main"
                      sx={{ display: "block", mt: 0.5, fontWeight: 500 }}
                    >
                      Correct answer: {a.correctAnswer}
                    </Typography>
                  )}
                </Box>
              )}
              {a.explanation && (
                <>
                  <Divider sx={{ my: 1 }} />
                  <Typography variant="caption" color="text.secondary">
                    {a.explanation}
                  </Typography>
                </>
              )}
            </CardContent>
          </Card>
        ))}
      </Stack>

      {/* Attempt history */}
      {history.length > 1 && (
        <Box sx={{ mb: 4 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 2 }}>
            All attempts
          </Typography>
          <Stack spacing={1}>
            {history.map((s) => {
              const pct =
                s.maxPoints && s.maxPoints > 0
                  ? Math.round(((s.pointsAwarded ?? 0) / s.maxPoints) * 100)
                  : null;
              const isCurrent = s.id === data.id;
              return (
                <Card
                  key={s.id}
                  variant="outlined"
                  sx={{
                    ...(isCurrent && { borderColor: "primary.main" }),
                    cursor: isCurrent ? "default" : "pointer",
                  }}
                  onClick={
                    isCurrent
                      ? undefined
                      : () => router.push(`/sessions/${s.id}/results`)
                  }
                >
                  <CardContent
                    sx={{
                      py: "12px !important",
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                    }}
                  >
                    <Box>
                      <Typography variant="body2" sx={{ fontWeight: 500 }}>
                        Attempt {s.attemptNumber}
                        {isCurrent && " (this one)"}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {formatDate(s.finishedAt ?? s.startedAt)}
                      </Typography>
                    </Box>
                    <Chip
                      label={pct != null ? `${pct}%` : "—"}
                      size="small"
                      variant="outlined"
                    />
                  </CardContent>
                </Card>
              );
            })}
          </Stack>
        </Box>
      )}

      {/* Footer */}
      <Stack direction="row" spacing={2} sx={{ justifyContent: "center" }}>
        <Button variant="outlined" onClick={() => router.push("/library")}>
          Back to assessments
        </Button>
      </Stack>
    </Box>
  );
}
