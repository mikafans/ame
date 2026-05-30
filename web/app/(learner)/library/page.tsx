"use client";

import { useState, useEffect } from "react";
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
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";
import AddOutlinedIcon from "@mui/icons-material/AddOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import { LearningObjectives } from "@/components/LearningObjectives";
import { formatMinutes } from "@/utils/format";

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
  completed?: boolean;
  createdAt: string;
}

type TabId = "all" | "completed" | "drafts";

export default function LibraryPage() {
  const { user } = useAuth();
  const router = useRouter();
  const [tab, setTab] = useState<TabId>("all");
  const [activeQuizzes, setActiveQuizzes] = useState<Quiz[]>([]);
  const [draftQuizzes, setDraftQuizzes] = useState<Quiz[]>([]);
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    const client = api;

    const fetchStatus = async (status: string): Promise<Quiz[]> => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (client as any).GET("/v1/quizzes", {
        params: { query: { status } },
      });
      return Array.isArray(data?.quizzes) ? data.quizzes : [];
    };

    const isInst = user?.role === "instructor" || user?.role === "admin";
    Promise.all([
      fetchStatus("active"),
      isInst ? fetchStatus("draft") : Promise.resolve([]),
    ])
      .then(([active, drafts]) => {
        setActiveQuizzes(active);
        setDraftQuizzes(drafts);
      })
      .finally(() => setLoading(false));
  }, [user?.role]);

  async function startQuiz(quizId: string) {
    setStarting(quizId);
    setStartError(null);
    try {
      const client = api;
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

  const pendingQuizzes = activeQuizzes.filter((q) => !q.completed);
  const completedQuizzes = activeQuizzes.filter((q) => q.completed);
  // "All quizzes" lists everything (completed ones carry a badge + Retake);
  // "Completed" is just a filtered view of the same set.
  const quizzes =
    tab === "completed"
      ? completedQuizzes
      : tab === "drafts"
        ? draftQuizzes
        : activeQuizzes;
  // "Up next" highlights the first quiz the learner hasn't finished yet.
  const featuredQuiz = pendingQuizzes[0] ?? null;

  return (
    <Box sx={{ p: 4 }}>
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
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              const { data } = await (api as any).POST("/v1/quizzes", {
                body: { title: "Untitled quiz" },
              });
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
                    ? formatMinutes(featuredQuiz.durationMin)
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
          label={`All quizzes (${activeQuizzes.length})`}
          sx={{ textTransform: "none" }}
        />
        {completedQuizzes.length > 0 && (
          <Tab
            value="completed"
            label={`Completed (${completedQuizzes.length})`}
            sx={{ textTransform: "none" }}
          />
        )}
        {draftQuizzes.length > 0 && (
          <Tab
            value="drafts"
            label={`Drafts (${draftQuizzes.length})`}
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
                      {quiz.completed && (
                        <Chip
                          label="Completed"
                          size="small"
                          color="success"
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
                          {formatMinutes(quiz.durationMin)}
                        </Typography>
                      )}
                    </Stack>
                  </Box>
                  <Stack direction="row" spacing={1}>
                    {quiz.status === "draft" ? (
                      <Button
                        size="small"
                        variant="outlined"
                        startIcon={<EditOutlinedIcon />}
                        onClick={() => router.push(`/author/${quiz.id}`)}
                      >
                        Edit
                      </Button>
                    ) : (
                      <Button
                        size="small"
                        variant="contained"
                        startIcon={<PlayArrowOutlinedIcon />}
                        disabled={starting === quiz.id}
                        onClick={() => startQuiz(quiz.id)}
                      >
                        {starting === quiz.id
                          ? "Starting…"
                          : quiz.completed
                            ? "Retake"
                            : "Start"}
                      </Button>
                    )}
                  </Stack>
                </Box>
              </CardContent>
            </Card>
          ))}
        </Stack>
      )}
    </Box>
  );
}
