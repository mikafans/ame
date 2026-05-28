"use client";

import { useState, useEffect, use } from "react";
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
  note: string;
  gradeStatus: string;
  explanation?: string;
}

interface ResultData {
  id: string;
  quiz_id?: string;
  quiz_title: string | null;
  course: string | null;
  attempt_number: number | null;
  total_attempts: number | null;
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
          quiz_id: session.quiz_id,
          quiz_title: session.quiz_title ?? null,
          course: session.course_title ?? null,
          attempt_number: null,
          total_attempts: null,
          score: result.points_awarded ?? 0,
          total: result.max_points ?? 0,
          answers: questions.map((q: any) => {
            const attempt = byQuestion.get(q.questionId);
            const r = attempt?.response as Record<string, unknown> | undefined;
            let given = "";
            if (r) {
              if ("selected_position" in r) {
                const pos = r.selected_position as number;
                given =
                  ["A", "B", "C", "D", "E", "F"][pos] ?? `Option ${pos + 1}`;
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
        if (session.quiz_id) {
          fetch(
            `${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080"}/v1/me/cohort-stats?quizId=${session.quiz_id}`,
            { credentials: "include" },
          )
            .then((r) => (r.ok ? r.json() : null))
            .then((cs: CohortStats | null) => {
              if (cs) setCohortStats(cs);
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
      <Typography variant="h5" sx={{ fontWeight: 500, mb: 3 }}>
        Quiz Results — {data.quiz_title || "Results"}
      </Typography>

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
              <Typography variant="caption" color="text.secondary">
                Your answer: {a.given || "—"}
              </Typography>
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

      {/* Footer */}
      <Stack direction="row" spacing={2} sx={{ justifyContent: "center" }}>
        <Button variant="outlined" onClick={() => router.push("/library")}>
          Back to library
        </Button>
      </Stack>
    </Box>
  );
}
