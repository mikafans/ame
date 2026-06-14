"use client";

import { useState, useEffect, use } from "react";
import { formatDate } from "@/utils/format";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { MarkdownView } from "@/components/MarkdownView";
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
import Drawer from "@mui/material/Drawer";
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircle";
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import CloseIcon from "@mui/icons-material/Close";
import { HighlightedCode } from "@/components/HighlightedCode";

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
  const [assessmentId, setAssessmentId] = useState<string | null>(null);
  const [retaking, setRetaking] = useState(false);

  const [loading, setLoading] = useState(true);

  // Deepen Drawer states
  const [deepenOpen, setDeepenOpen] = useState(false);
  const [deepenQid, setDeepenQid] = useState<string | null>(null);
  const [deepenHistory, setDeepenHistory] = useState<string[]>([]);
  const [deepenData, setDeepenData] = useState<{
    question: {
      id: string;
      prompt: string;
      kind: string;
      payload: any;
      explanation?: string | null;
      deepDive?: string | null;
      source?: string | null;
      tags: string[];
    };
    related: Array<{
      id: string;
      prompt: string;
      kind: string;
      tags: string[];
    }>;
  } | null>(null);
  const [deepenLoading, setDeepenLoading] = useState(false);
  const [deepenError, setDeepenError] = useState<string | null>(null);

  function handleOpenDeepen(qid: string, isFromHistory = false) {
    setDeepenOpen(true);
    setDeepenQid(qid);
    if (!isFromHistory) {
      setDeepenHistory([]);
    }
  }

  const handleBack = () => {
    const nextHistory = [...deepenHistory];
    const prevQid = nextHistory.pop();
    if (prevQid) {
      setDeepenHistory(nextHistory);
      handleOpenDeepen(prevQid, true);
    }
  };

  useEffect(() => {
    if (!deepenOpen || !deepenQid) {
      setDeepenData(null);
      return;
    }

    setDeepenLoading(true);
    setDeepenError(null);

    // Build exclude list (include currently viewed question and history questions)
    const excludeIds = [deepenQid, ...deepenHistory];
    const excludeQuery = excludeIds.join(",");

    api
      .GET(
        "/v1/questions/{id}/deepen" as never,
        {
          params: {
            path: { id: deepenQid },
            query: { exclude: excludeQuery },
          },
        } as never,
      )
      .then(({ data: res }: { data?: any }) => {
        if (res) {
          setDeepenData(res);
        } else {
          setDeepenError("Failed to load question details.");
        }
      })
      .catch((err) => {
        console.error(err);
        setDeepenError(err?.message || "Failed to load question details.");
      })
      .finally(() => {
        setDeepenLoading(false);
      });
  }, [deepenQid, deepenOpen, deepenHistory]);

  useEffect(() => {
    setLoading(true);
    api
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .then(({ data: d }: { data?: any }) => {
        if (!d?.session) return;
        const session = d.session;
        setAssessmentId(session.assessment_id ?? null);
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

  const handleRetake = async () => {
    if (!assessmentId || retaking) return;
    setRetaking(true);
    try {
      const { data: d } = await api.POST("/v1/sessions", {
        body: { assessmentId },
      });
      if (d?.sessionId) {
        router.push(`/sessions/${d.sessionId}`);
      } else {
        setRetaking(false);
      }
    } catch (err) {
      console.error(err);
      setRetaking(false);
    }
  };

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
    <Box sx={{ px: { xs: 2, sm: 4 }, py: 4, maxWidth: 800, mx: "auto" }}>
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
            sx={{
              justifyContent: "space-between",
              alignItems: "center",
              flexWrap: "wrap",
              gap: 1,
            }}
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
          <Card
            key={a.qid}
            variant="outlined"
            sx={{
              position: "relative",
              overflow: "hidden",
              borderRadius: 2,
              transition: "all 0.3s cubic-bezier(0.4, 0, 0.2, 1)",
              ...(!a.correct && a.gradeStatus !== "pending_manual"
                ? {
                    borderColor: "error.light",
                    borderLeft: "6px solid",
                    borderLeftColor: "error.main",
                    background: (theme) =>
                      theme.palette.mode === "dark"
                        ? "linear-gradient(135deg, rgba(211, 47, 47, 0.12) 0%, rgba(30, 30, 30, 0.95) 100%)"
                        : "linear-gradient(135deg, rgba(211, 47, 47, 0.03) 0%, rgba(255, 255, 255, 1) 100%)",
                    boxShadow: (theme) =>
                      theme.palette.mode === "dark"
                        ? "0 4px 20px rgba(211, 47, 47, 0.15)"
                        : "0 4px 20px rgba(211, 47, 47, 0.06)",
                    "&:hover": {
                      transform: "translateY(-2px)",
                      boxShadow: (theme) =>
                        theme.palette.mode === "dark"
                          ? "0 8px 28px rgba(211, 47, 47, 0.25)"
                          : "0 8px 28px rgba(211, 47, 47, 0.12)",
                    },
                  }
                : {
                    "&:hover": {
                      transform: "translateY(-1px)",
                      boxShadow: "0 4px 12px rgba(0, 0, 0, 0.05)",
                    },
                  }),
            }}
          >
            <CardContent>
              <Box
                sx={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "flex-start",
                  mb: 2,
                }}
              >
                <Typography
                  variant="body2"
                  sx={{ fontWeight: 600, color: "text.primary", pr: 2 }}
                >
                  Q{i + 1}. {a.prompt}
                </Typography>
                <Stack
                  direction="row"
                  spacing={1}
                  sx={{ alignItems: "center", flexShrink: 0 }}
                >
                  <Button
                    size="small"
                    variant="text"
                    onClick={() => handleOpenDeepen(a.qid)}
                    sx={{
                      textTransform: "none",
                      fontSize: "0.75rem",
                      fontWeight: 600,
                    }}
                  >
                    Dive deeper
                  </Button>
                  {a.gradeStatus === "pending_manual" ? (
                    <Chip
                      label="Pending review"
                      size="small"
                      variant="outlined"
                    />
                  ) : a.correct ? (
                    <Chip
                      label={`${a.points}/${a.max} PTS`}
                      size="small"
                      color="success"
                      variant="outlined"
                      sx={{ fontWeight: 600 }}
                    />
                  ) : (
                    <Chip
                      label={`${a.points}/${a.max} PTS`}
                      size="small"
                      color="error"
                      sx={{
                        fontWeight: 700,
                        backgroundColor: (theme) =>
                          theme.palette.mode === "dark"
                            ? "rgba(211, 47, 47, 0.25)"
                            : "rgba(211, 47, 47, 0.08)",
                        animation: "pulse 2.5s infinite ease-in-out",
                        "@keyframes pulse": {
                          "0%": { transform: "scale(1)" },
                          "50%": { transform: "scale(1.04)" },
                          "100%": { transform: "scale(1)" },
                        },
                      }}
                    />
                  )}
                </Stack>
              </Box>

              {/* Response detail block */}
              {a.gradeStatus === "pending_manual" ? (
                <Box>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: "block", mb: 0.5, fontWeight: 600 }}
                  >
                    Your submission
                  </Typography>
                  {a.given ? (
                    <Box sx={{ mt: 0.5 }}>
                      <HighlightedCode
                        code={a.given}
                        language={a.type === "code" ? "python" : "text"}
                      />
                    </Box>
                  ) : (
                    <Typography variant="caption" color="text.secondary">
                      No answer submitted
                    </Typography>
                  )}
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: "block", mt: 1, fontStyle: "italic" }}
                  >
                    Awaiting manual review — code/essay isn’t auto-graded.
                  </Typography>
                </Box>
              ) : a.correct ? (
                // Correct answer style: clean and tidy
                <Box
                  sx={{
                    pl: 1,
                    borderLeft: "3px solid",
                    borderLeftColor: "success.light",
                  }}
                >
                  <Typography
                    variant="body2"
                    color="success.main"
                    sx={{ fontWeight: 500 }}
                  >
                    Correct: {a.given || "—"}
                  </Typography>
                </Box>
              ) : (
                // Wrong answer style: astonishing side-by-side comparison block
                <Box
                  sx={{
                    display: "flex",
                    flexDirection: { xs: "column", sm: "row" },
                    gap: 2,
                    mt: 1.5,
                    p: 2,
                    borderRadius: 2,
                    backgroundColor: (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(211, 47, 47, 0.08)"
                        : "rgba(211, 47, 47, 0.02)",
                    border: "1px dashed",
                    borderColor: "error.light",
                  }}
                >
                  <Box sx={{ flex: 1 }}>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ display: "block", fontWeight: 600, mb: 0.5 }}
                    >
                      Your Answer
                    </Typography>
                    {a.type === "code" && a.given ? (
                      <HighlightedCode code={a.given} language="python" />
                    ) : (
                      <Typography
                        variant="body2"
                        color="error.main"
                        sx={{ fontWeight: 600 }}
                      >
                        {a.given || "—"}
                      </Typography>
                    )}
                  </Box>
                  {a.correctAnswer && (
                    <Box
                      sx={{
                        flex: 1,
                        borderLeft: { sm: "1px solid" },
                        borderColor: { sm: "divider" },
                        pl: { sm: 2 },
                      }}
                    >
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ display: "block", fontWeight: 600, mb: 0.5 }}
                      >
                        Correct Solution
                      </Typography>
                      {a.type === "code" ? (
                        <HighlightedCode
                          code={a.correctAnswer}
                          language="python"
                        />
                      ) : (
                        <Typography
                          variant="body2"
                          color="success.main"
                          sx={{ fontWeight: 600 }}
                        >
                          {a.correctAnswer}
                        </Typography>
                      )}
                    </Box>
                  )}
                </Box>
              )}

              {a.explanation && (
                <>
                  <Divider sx={{ my: 2 }} />
                  <Box
                    sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}
                  >
                    <Typography
                      variant="caption"
                      color="primary.main"
                      sx={{ fontWeight: 600, flexShrink: 0 }}
                    >
                      Explanation:
                    </Typography>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ lineHeight: 1.4 }}
                    >
                      {a.explanation}
                    </Typography>
                  </Box>
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

      {/* Deepen Drawer */}
      <Drawer
        anchor="right"
        open={deepenOpen}
        onClose={() => setDeepenOpen(false)}
        slotProps={{
          paper: {
            sx: { width: { xs: "100%", sm: 540 }, maxWidth: "100%" },
          },
        }}
      >
        <Box
          sx={{
            display: "flex",
            flexDirection: "column",
            height: "100%",
            bgcolor: "background.default",
          }}
        >
          {/* Header */}
          <Box
            sx={{
              p: 3,
              borderBottom: "1px solid",
              borderColor: "divider",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              bgcolor: "background.paper",
            }}
          >
            <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              {deepenHistory.length > 0 && (
                <Button
                  size="small"
                  variant="text"
                  onClick={handleBack}
                  startIcon={<ArrowBackIcon />}
                  sx={{ textTransform: "none", mr: 1 }}
                >
                  Back
                </Button>
              )}
              <Typography variant="h6" fontWeight="bold">
                Dive Deeper
              </Typography>
            </Box>
            <Button
              size="small"
              onClick={() => setDeepenOpen(false)}
              sx={{ minWidth: 40, p: 1, borderRadius: "50%" }}
            >
              <CloseIcon />
            </Button>
          </Box>

          {/* Body */}
          <Box sx={{ flex: 1, overflowY: "auto", p: 3 }}>
            {deepenLoading ? (
              <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
                <CircularProgress size={28} />
              </Box>
            ) : deepenError ? (
              <Alert severity="error">{deepenError}</Alert>
            ) : deepenData ? (
              <Stack spacing={3}>
                {/* Question Prompt */}
                <Box>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{
                      textTransform: "uppercase",
                      fontWeight: 600,
                      display: "block",
                      mb: 1,
                    }}
                  >
                    Question Prompt
                  </Typography>
                  <Typography variant="body1" fontWeight={500}>
                    {deepenData.question.prompt}
                  </Typography>
                </Box>

                {/* Answer Options / Key (Live Preview matching admin style) */}
                <Box>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{
                      textTransform: "uppercase",
                      fontWeight: 600,
                      display: "block",
                      mb: 1.5,
                    }}
                  >
                    Answer Key
                  </Typography>

                  {deepenData.question.kind === "mc" && (
                    <Stack spacing={1}>
                      {(
                        ((deepenData.question.payload as any)
                          ?.options as string[]) ?? []
                      ).map((opt, idx) => {
                        const isCorrect =
                          idx ===
                          (deepenData.question.payload as any)?.correct_index;
                        return (
                          <Box
                            key={idx}
                            sx={{
                              p: 1.5,
                              borderRadius: 1,
                              border: "1px solid",
                              borderColor: isCorrect
                                ? "success.light"
                                : "divider",
                              bgcolor: isCorrect
                                ? "rgba(46, 125, 50, 0.08)"
                                : "background.paper",
                              display: "flex",
                              justifyContent: "space-between",
                              alignItems: "center",
                            }}
                          >
                            <Typography
                              variant="body2"
                              sx={{ fontWeight: isCorrect ? 600 : 400 }}
                            >
                              {opt}
                            </Typography>
                            {isCorrect && (
                              <Typography
                                variant="caption"
                                color="success.main"
                                sx={{ fontWeight: 600 }}
                              >
                                Correct Option
                              </Typography>
                            )}
                          </Box>
                        );
                      })}
                    </Stack>
                  )}

                  {deepenData.question.kind === "tf" && (
                    <Stack direction="row" spacing={2}>
                      {["True", "False"].map((opt) => {
                        const val = opt === "True";
                        const isCorrect =
                          val === (deepenData.question.payload as any)?.correct;
                        return (
                          <Box
                            key={opt}
                            sx={{
                              flex: 1,
                              p: 1.5,
                              borderRadius: 1,
                              border: "1px solid",
                              borderColor: isCorrect
                                ? "success.light"
                                : "divider",
                              bgcolor: isCorrect
                                ? "rgba(46, 125, 50, 0.08)"
                                : "background.paper",
                              textAlign: "center",
                            }}
                          >
                            <Typography
                              variant="body2"
                              sx={{ fontWeight: isCorrect ? 600 : 400 }}
                            >
                              {opt}
                            </Typography>
                          </Box>
                        );
                      })}
                    </Stack>
                  )}

                  {deepenData.question.kind === "short" && (
                    <Box
                      sx={{
                        p: 2,
                        borderRadius: 1,
                        bgcolor: "background.paper",
                        border: "1px solid",
                        borderColor: "divider",
                      }}
                    >
                      <Typography
                        variant="body2"
                        sx={{ mb: 1, fontWeight: 500 }}
                      >
                        Accepted Answers:
                      </Typography>
                      <Stack
                        direction="row"
                        spacing={1}
                        flexWrap="wrap"
                        useFlexGap
                      >
                        {(
                          ((deepenData.question.payload as any)
                            ?.accepted as string[]) ?? []
                        ).map((ans) => (
                          <Chip
                            key={ans}
                            label={ans}
                            size="small"
                            color="success"
                            variant="outlined"
                          />
                        ))}
                      </Stack>
                    </Box>
                  )}

                  {(deepenData.question.kind === "essay" ||
                    deepenData.question.kind === "code") && (
                    <Box
                      sx={{
                        p: 2,
                        borderRadius: 1,
                        bgcolor: "background.paper",
                        border: "1px solid",
                        borderColor: "divider",
                      }}
                    >
                      <Typography variant="body2" sx={{ fontWeight: 500 }}>
                        Model/Exemplar Answer:
                      </Typography>
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{ mt: 1, whiteSpace: "pre-wrap" }}
                      >
                        {(deepenData.question.payload as any)?.exemplar ||
                          "No exemplar answer provided."}
                      </Typography>
                    </Box>
                  )}
                </Box>

                {/* Explanation */}
                {deepenData.question.explanation && (
                  <Box>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{
                        textTransform: "uppercase",
                        fontWeight: 600,
                        display: "block",
                        mb: 1,
                      }}
                    >
                      Explanation
                    </Typography>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{ lineHeight: 1.5 }}
                    >
                      {deepenData.question.explanation}
                    </Typography>
                  </Box>
                )}

                {/* External Link */}
                {deepenData.question.source && (
                  <Box>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{
                        textTransform: "uppercase",
                        fontWeight: 600,
                        display: "block",
                        mb: 1.5,
                      }}
                    >
                      Reference Link
                    </Typography>
                    <Button
                      variant="outlined"
                      href={deepenData.question.source}
                      target="_blank"
                      rel="noopener noreferrer"
                      sx={{ textTransform: "none" }}
                    >
                      Visit External Source
                    </Button>
                  </Box>
                )}

                {/* Study Notes */}
                {deepenData.question.deepDive && (
                  <Box>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{
                        textTransform: "uppercase",
                        fontWeight: 600,
                        display: "block",
                        mb: 1.5,
                      }}
                    >
                      Deep Dive Study Notes
                    </Typography>
                    <Box
                      sx={{
                        border: "1px solid",
                        borderColor: "divider",
                        borderRadius: 1,
                        p: 2,
                        bgcolor: "background.paper",
                      }}
                    >
                      <MarkdownView content={deepenData.question.deepDive} />
                    </Box>
                  </Box>
                )}

                {/* Tags */}
                {deepenData.question.tags &&
                  deepenData.question.tags.length > 0 && (
                    <Box>
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{
                          textTransform: "uppercase",
                          fontWeight: 600,
                          display: "block",
                          mb: 1.5,
                        }}
                      >
                        Tags
                      </Typography>
                      <Stack
                        direction="row"
                        spacing={1}
                        flexWrap="wrap"
                        useFlexGap
                      >
                        {deepenData.question.tags.map((t) => (
                          <Chip key={t} label={t} size="small" />
                        ))}
                      </Stack>
                    </Box>
                  )}

                {/* Related Questions */}
                <Box>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{
                      textTransform: "uppercase",
                      fontWeight: 600,
                      display: "block",
                      mb: 1.5,
                    }}
                  >
                    Related Questions (By Shared Tags)
                  </Typography>
                  {deepenData.related && deepenData.related.length > 0 ? (
                    <Stack spacing={1.5}>
                      {deepenData.related.map((rq) => (
                        <Card
                          key={rq.id}
                          variant="outlined"
                          sx={{
                            cursor: "pointer",
                            bgcolor: "background.paper",
                            "&:hover": {
                              borderColor: "primary.main",
                              bgcolor: "action.hover",
                            },
                          }}
                          onClick={() => {
                            setDeepenHistory([...deepenHistory, deepenQid!]);
                            handleOpenDeepen(rq.id, true);
                          }}
                        >
                          <CardContent sx={{ p: "12px !important" }}>
                            <Typography
                              variant="body2"
                              sx={{ fontWeight: 500 }}
                            >
                              {rq.prompt}
                            </Typography>
                            <Stack direction="row" spacing={0.5} sx={{ mt: 1 }}>
                              {rq.tags.map((t) => (
                                <Chip
                                  key={t}
                                  label={t}
                                  size="small"
                                  sx={{ fontSize: 9, height: 16 }}
                                />
                              ))}
                            </Stack>
                          </CardContent>
                        </Card>
                      ))}
                    </Stack>
                  ) : (
                    <Typography variant="caption" color="text.secondary">
                      No related questions found.
                    </Typography>
                  )}
                </Box>
              </Stack>
            ) : null}
          </Box>
        </Box>
      </Drawer>

      {/* Footer */}
      <Stack direction="row" spacing={2} sx={{ justifyContent: "center" }}>
        <Button variant="outlined" onClick={() => router.push("/explore")}>
          Back to Explore
        </Button>
        {assessmentId && (
          <Button
            variant="contained"
            onClick={handleRetake}
            disabled={retaking}
          >
            {retaking ? "Starting…" : "Re-take"}
          </Button>
        )}
      </Stack>
    </Box>
  );
}
