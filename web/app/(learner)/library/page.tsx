"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import ShareOutlinedIcon from "@mui/icons-material/ShareOutlined";
import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";
import AddOutlinedIcon from "@mui/icons-material/AddOutlined";
import { LearningObjectives } from "@/components/LearningObjectives";
import { ShareModal } from "@/components/ShareModal";

interface Quiz {
  id: string;
  title: string;
  description?: string;
  status: string;
  course?: string;
  difficulty?: string;
  color?: string;
  objectives?: string[];
  due_date?: string;
  questionCount?: number;
  durationMin?: number;
  attemptLimit?: number;
  createdAt: string;
}

interface CohortStats {
  cohortAvg: number | null;
  percentile: number | null;
  completionRate: number | null;
}

type TabId = "all" | "assigned" | "completed" | "drafts";

export default function LibraryPage() {
  const { user, token } = useAuth();
  const router = useRouter();
  const [tab, setTab] = useState<TabId>("all");
  const [allQuizzes, setAllQuizzes] = useState<
    Record<TabId, { quizzes: Quiz[]; total: number }>
  >({
    all: { quizzes: [], total: 0 },
    assigned: { quizzes: [], total: 0 },
    completed: { quizzes: [], total: 0 },
    drafts: { quizzes: [], total: 0 },
  });
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);
  const [shareOpen, setShareOpen] = useState<string | null>(null);
  const [cohortStats, setCohortStats] = useState<CohortStats | null>(null);

  useEffect(() => {
    if (!token) return;
    setLoading(true);
    const client = makeClient(token);

    const fetchStatus = async (
      status: string,
    ): Promise<{ quizzes: Quiz[]; total: number }> => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (client as any).GET("/v1/quizzes", {
        params: { query: { status } },
      });
      const quizzes = Array.isArray(data?.quizzes) ? data.quizzes : [];
      return { quizzes, total: data?.total ?? quizzes.length };
    };

    const isInst = user?.role === "instructor" || user?.role === "admin";
    Promise.all([
      fetchStatus("active"),
      isInst
        ? fetchStatus("draft")
        : Promise.resolve({ quizzes: [], total: 0 }),
    ])
      .then(([active, drafts]) => {
        setAllQuizzes({
          all: active,
          assigned: active,
          completed: { quizzes: [], total: 0 },
          drafts,
        });
      })
      .finally(() => setLoading(false));
  }, [token, user?.role]);

  const upNextId =
    tab === "all" || tab === "assigned"
      ? (allQuizzes[tab].quizzes[0]?.id ?? null)
      : null;

  useEffect(() => {
    if (!token || !upNextId) return;
    setCohortStats(null);
    fetch(
      `${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080"}/v1/me/cohort-stats?quizId=${upNextId}`,
      { headers: { Authorization: `Bearer ${token}` } },
    )
      .then((r) => (r.ok ? r.json() : null))
      .then((cs: CohortStats | null) => {
        if (cs) setCohortStats(cs);
      })
      .catch(() => {});
  }, [token, upNextId]);

  async function startQuiz(quizId: string) {
    if (!token) return;
    setStarting(quizId);
    setStartError(null);
    try {
      const client = makeClient(token);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data, error } = await (client as any).POST("/v1/sessions", {
        body: { quizId },
      });
      if (error) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        setStartError((error as any)?.message ?? "Failed to start quiz");
        return;
      }
      if (data?.sessionId) {
        router.push(`/sessions/${data.sessionId}`);
      }
    } catch (err) {
      console.error(err);
      setStartError("Unexpected error — check the console");
    } finally {
      setStarting(null);
    }
  }

  const isInstructor = user?.role === "instructor" || user?.role === "admin";

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

  const { quizzes } = allQuizzes[tab];
  const featuredQuiz = allQuizzes.all.quizzes[0] ?? null;

  return (
    <Box sx={{ p: 4 }}>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{
          letterSpacing: 1.4,
          textTransform: "uppercase",
          display: "block",
        }}
      >
        Spring 2026 · Active Term
      </Typography>
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          mb: 3,
        }}
      >
        <Typography variant="h4" sx={{ fontWeight: 500 }}>
          Library
        </Typography>
        {isInstructor && (
          <Button
            variant="outlined"
            startIcon={<AddOutlinedIcon />}
            onClick={async () => {
              if (!token) return;
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              const { data } = await (makeClient(token) as any).POST(
                "/v1/quizzes",
                {
                  body: { title: "Untitled quiz" },
                },
              );
              if (data?.quiz?.id) router.push(`/author/${data.quiz.id}`);
            }}
          >
            New quiz
          </Button>
        )}
      </Box>

      {featuredQuiz && (
        <Card variant="outlined" sx={{ mb: 4, borderColor: "primary.main" }}>
          <CardContent>
            <Typography
              variant="overline"
              color="primary"
              sx={{ display: "block", mb: 0.5 }}
            >
              Up next
            </Typography>
            <Typography variant="h6" sx={{ mb: 1, fontWeight: 500 }}>
              {featuredQuiz.title}
            </Typography>
            {featuredQuiz.description && (
              <Typography
                variant="body2"
                color="text.secondary"
                sx={{ mb: 1.5 }}
              >
                {featuredQuiz.description}
              </Typography>
            )}
            <Stack direction="row" spacing={3} sx={{ mb: 2 }}>
              <Box>
                <Typography variant="caption" color="text.secondary">
                  Questions
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {featuredQuiz.questionCount ?? "—"}
                </Typography>
              </Box>
              <Box>
                <Typography variant="caption" color="text.secondary">
                  Duration
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {featuredQuiz.durationMin
                    ? `${featuredQuiz.durationMin}m`
                    : "—"}
                </Typography>
              </Box>
              <Box>
                <Typography variant="caption" color="text.secondary">
                  Attempts
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {featuredQuiz.attemptLimit ?? "Unlimited"}
                </Typography>
              </Box>
            </Stack>
            {featuredQuiz.objectives && featuredQuiz.objectives.length > 0 && (
              <Box sx={{ mb: 2 }}>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{
                    display: "block",
                    mb: 0.5,
                    letterSpacing: 1.2,
                    textTransform: "uppercase",
                  }}
                >
                  Recommended prep
                </Typography>
                <LearningObjectives items={featuredQuiz.objectives} />
              </Box>
            )}
            <Stack direction="row" spacing={1}>
              <Button
                size="small"
                variant="outlined"
                onClick={() =>
                  router.push(`/quizzes/${featuredQuiz.id}/preview`)
                }
              >
                Preview questions
              </Button>
              <Button
                size="small"
                variant="contained"
                startIcon={<PlayArrowOutlinedIcon />}
                disabled={starting === featuredQuiz.id}
                onClick={() => startQuiz(featuredQuiz.id)}
              >
                {starting === featuredQuiz.id ? "Starting…" : "Start"}
              </Button>
            </Stack>
          </CardContent>
        </Card>
      )}

      <Tabs
        value={tab}
        onChange={(_, v) => setTab(v)}
        sx={{ mb: 3, borderBottom: 1, borderColor: "divider" }}
      >
        <Tab
          value="all"
          label={`All quizzes (${allQuizzes.all.total})`}
          sx={{ textTransform: "none" }}
        />
        <Tab
          value="assigned"
          label={`Assigned to me (${allQuizzes.assigned.total})`}
          sx={{ textTransform: "none" }}
        />
        <Tab
          value="completed"
          label={`Completed (${allQuizzes.completed.total})`}
          sx={{ textTransform: "none" }}
        />
        {allQuizzes.drafts.total > 0 && (
          <Tab
            value="drafts"
            label={`Drafts (${allQuizzes.drafts.total})`}
            sx={{ textTransform: "none" }}
          />
        )}
      </Tabs>

      {startError && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {startError}
        </Alert>
      )}

      {quizzes.length === 0 ? (
        <Typography color="text.secondary">No quizzes in this tab.</Typography>
      ) : (
        <Stack spacing={2}>
          {quizzes.map((quiz) => (
            <Card key={quiz.id} variant="outlined">
              <CardContent>
                <Box
                  sx={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                  }}
                >
                  <Box sx={{ flex: 1 }}>
                    <Box
                      sx={{
                        display: "flex",
                        alignItems: "center",
                        gap: 1,
                        mb: 0.5,
                      }}
                    >
                      <Typography variant="subtitle1" sx={{ fontWeight: 500 }}>
                        {quiz.title}
                      </Typography>
                      {quiz.course && (
                        <Chip
                          label={quiz.course}
                          size="small"
                          variant="outlined"
                        />
                      )}
                      {quiz.difficulty && (
                        <Chip
                          label={quiz.difficulty}
                          size="small"
                          color="primary"
                          variant="outlined"
                        />
                      )}
                    </Box>
                    {quiz.description && (
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{ mb: 1 }}
                      >
                        {quiz.description}
                      </Typography>
                    )}
                    {quiz.objectives && quiz.objectives.length > 0 && (
                      <Box sx={{ mb: 1 }}>
                        <LearningObjectives items={quiz.objectives} />
                      </Box>
                    )}
                    <Stack direction="row" spacing={2}>
                      {quiz.questionCount != null && (
                        <Typography variant="caption" color="text.secondary">
                          {quiz.questionCount} questions
                        </Typography>
                      )}
                      {quiz.durationMin != null && (
                        <Typography variant="caption" color="text.secondary">
                          {quiz.durationMin} min
                        </Typography>
                      )}
                    </Stack>
                  </Box>
                  <Stack direction="row" spacing={1}>
                    <Button
                      size="small"
                      variant="outlined"
                      startIcon={<ShareOutlinedIcon />}
                      onClick={() => setShareOpen(quiz.id)}
                    >
                      Share
                    </Button>
                    <Button
                      size="small"
                      variant="contained"
                      startIcon={<PlayArrowOutlinedIcon />}
                      disabled={starting === quiz.id}
                      onClick={() => startQuiz(quiz.id)}
                    >
                      {starting === quiz.id ? "Starting…" : "Start"}
                    </Button>
                  </Stack>
                </Box>
              </CardContent>
            </Card>
          ))}
        </Stack>
      )}

      {shareOpen && (
        <ShareModal
          payload={{
            kind: "quiz",
            id: shareOpen,
            title: quizzes.find((q) => q.id === shareOpen)?.title || "",
          }}
          onClose={() => setShareOpen(null)}
          bearerToken={token}
        />
      )}
    </Box>
  );
}
