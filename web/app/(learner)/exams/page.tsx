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
import TextField from "@mui/material/TextField";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import Divider from "@mui/material/Divider";
import ShareOutlinedIcon from "@mui/icons-material/ShareOutlined";
import { LearningObjectives } from "@/components/LearningObjectives";
import { ShareModal } from "@/components/ShareModal";

interface ExamSection {
  id: string;
  title: string;
  weight: number;
  items: number;
  mix?: string;
  quizId?: string;
}

interface Exam {
  id: string;
  name: string;
  description?: string;
  status: string;
  method: string;
  composedBy?: string;
  compositionTrace?: unknown;
  durationMin?: number;
  passingPoints?: number;
  objectives: string[];
  sections: ExamSection[] | null;
  totalPoints: number;
  course?: string;
  tags?: string[];
}

type TabId = "all" | "published" | "scheduled" | "draft";

interface SectionDraft {
  title: string;
  weight: number;
  questionIds: string;
}

export default function ExamsPage() {
  const { token, user } = useAuth();
  const router = useRouter();
  const [exams, setExams] = useState<Exam[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [tab, setTab] = useState<TabId>("all");
  const [loading, setLoading] = useState(true);
  const [showCompose, setShowCompose] = useState(false);
  const [composeName, setComposeName] = useState("");
  const [composeDesc, setComposeDesc] = useState("");
  const [composeDuration, setComposeDuration] = useState(60);
  const [sections, setSections] = useState<SectionDraft[]>([
    { title: "", weight: 1, questionIds: "" },
  ]);
  const [composeError, setComposeError] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);
  const [shareExam, setShareExam] = useState<Exam | null>(null);
  const [starting, setStarting] = useState(false);

  const isInstructor = user?.role === "instructor" || user?.role === "admin";

  function load() {
    if (!token) return;
    setLoading(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET("/v1/exams")
      .then(({ data }: { data?: { exams: Exam[] } }) => {
        if (data?.exams) {
          setExams(data.exams);
          if (!selected && data.exams.length) setSelected(data.exams[0].id);
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token]);

  const tabs: { id: TabId; label: string }[] = [
    { id: "all", label: "All" },
    { id: "published", label: "Active" },
    { id: "scheduled", label: "Scheduled" },
    { id: "draft", label: "Drafts" },
  ];

  const filtered =
    tab === "all" ? exams : exams.filter((e) => e.status === tab);
  const exam = exams.find((e) => e.id === selected) ?? null;

  async function startExam() {
    if (!token || !selected) return;
    setStarting(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (makeClient(token) as any).POST("/v1/sessions", {
        body: { examId: selected },
      });
      if (data?.sessionId) router.push(`/sessions/${data.sessionId}`);
    } catch (err) {
      console.error(err);
    } finally {
      setStarting(false);
    }
  }

  async function handleCompose() {
    if (!token) return;
    setComposing(true);
    setComposeError(null);
    try {
      const body = {
        name: composeName,
        description: composeDesc || undefined,
        durationMin: composeDuration,
        sections: sections.map((s, i) => ({
          title: s.title || `Section ${i + 1}`,
          weight: s.weight,
          questionIds: s.questionIds
            .split(",")
            .map((x) => x.trim())
            .filter(Boolean),
          items: null,
          tags: [],
          types: [],
        })),
      };
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data, error } = await (makeClient(token) as any).POST(
        "/v1/exams",
        { body },
      );
      if (error) {
        if (
          typeof error === "object" &&
          "code" in error &&
          error.code === "pool_insufficient"
        ) {
          setComposeError(
            `Pool insufficient: ${(error as { message?: string }).message ?? "not enough questions match the section constraints"}`,
          );
        } else {
          setComposeError("Failed to compose exam. Check section constraints.");
        }
        return;
      }
      if (data?.examId) {
        setShowCompose(false);
        load();
        setSelected(data.examId);
      }
    } catch {
      setComposeError("Could not reach the API.");
    } finally {
      setComposing(false);
    }
  }

  async function handlePublish(id: string) {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (makeClient(token) as any).PATCH(`/v1/exams/${id}`, {
      body: { status: "published" },
    });
    load();
  }

  return (
    <Box sx={{ display: "flex", height: "100vh", overflow: "hidden" }}>
      {/* Left: exam list */}
      <Box
        sx={{
          width: 320,
          borderRight: 1,
          borderColor: "divider",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <Box sx={{ p: 2.5, borderBottom: 1, borderColor: "divider" }}>
          <Typography variant="h6" sx={{ fontWeight: 500 }}>
            Exams
          </Typography>
          <Tabs value={tab} onChange={(_, v) => setTab(v)} sx={{ mt: 1 }}>
            {tabs.map((t) => (
              <Tab
                key={t.id}
                value={t.id}
                label={t.label}
                sx={{ textTransform: "none", minWidth: 0, fontSize: 12 }}
              />
            ))}
          </Tabs>
        </Box>

        <Box sx={{ flex: 1, overflowY: "auto", p: 1 }}>
          {loading ? (
            <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
              <CircularProgress size={24} />
            </Box>
          ) : filtered.length === 0 ? (
            <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
              No exams.
            </Typography>
          ) : (
            filtered.map((e) => (
              <Card
                key={e.id}
                variant="outlined"
                onClick={() => setSelected(e.id)}
                sx={{
                  mb: 1,
                  cursor: "pointer",
                  ...(selected === e.id && { borderColor: "primary.main" }),
                }}
              >
                <CardContent sx={{ pb: "12px !important" }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 500 }}>
                    {e.name}
                  </Typography>
                  <Stack direction="row" spacing={0.75} sx={{ mt: 0.75 }}>
                    <Chip label={e.status} size="small" variant="outlined" />
                    {e.durationMin && (
                      <Chip
                        label={`${e.durationMin}m`}
                        size="small"
                        variant="outlined"
                      />
                    )}
                  </Stack>
                </CardContent>
              </Card>
            ))
          )}
        </Box>

        {isInstructor && (
          <Box sx={{ p: 1.5, borderTop: 1, borderColor: "divider" }}>
            <Button
              fullWidth
              variant="outlined"
              onClick={() => setShowCompose(true)}
            >
              Compose exam
            </Button>
          </Box>
        )}
      </Box>

      {/* Right: exam detail */}
      <Box sx={{ flex: 1, overflowY: "auto", p: 3 }}>
        {!exam ? (
          <Typography color="text.secondary">Select an exam.</Typography>
        ) : (
          <Stack spacing={2}>
            <Box
              sx={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "flex-start",
              }}
            >
              <Box>
                <Typography variant="h5" sx={{ fontWeight: 500 }}>
                  {exam.name}
                </Typography>
                {exam.description && (
                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{ mt: 0.5 }}
                  >
                    {exam.description}
                  </Typography>
                )}
              </Box>
              <Stack direction="row" spacing={1}>
                {exam.status === "published" && (
                  <Button
                    variant="contained"
                    disabled={starting}
                    onClick={startExam}
                  >
                    {starting ? "Starting…" : "Start exam"}
                  </Button>
                )}
                {isInstructor && exam.status !== "published" && (
                  <Button
                    variant="outlined"
                    onClick={() => handlePublish(exam.id)}
                  >
                    Publish
                  </Button>
                )}
                <Button
                  variant="outlined"
                  startIcon={<ShareOutlinedIcon />}
                  onClick={() => setShareExam(exam)}
                >
                  Share
                </Button>
              </Stack>
            </Box>

            <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
              <Chip label={exam.status} variant="outlined" />
              {exam.course && <Chip label={exam.course} variant="outlined" />}
              {exam.durationMin && (
                <Chip label={`${exam.durationMin} min`} variant="outlined" />
              )}
            </Stack>

            {exam.objectives.length > 0 && (
              <LearningObjectives items={exam.objectives} />
            )}

            {exam.sections && exam.sections.length > 0 && (
              <Box>
                <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 1 }}>
                  Sections
                </Typography>
                <Stack spacing={1}>
                  {exam.sections.map((s) => (
                    <Card key={s.id} variant="outlined">
                      <CardContent sx={{ pb: "12px !important" }}>
                        <Box
                          sx={{
                            display: "flex",
                            justifyContent: "space-between",
                          }}
                        >
                          <Typography variant="body2" sx={{ fontWeight: 500 }}>
                            {s.title}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {s.items} questions · {s.weight}pts
                          </Typography>
                        </Box>
                      </CardContent>
                    </Card>
                  ))}
                </Stack>
              </Box>
            )}
          </Stack>
        )}
      </Box>

      {showCompose && (
        <Box
          sx={{
            position: "fixed",
            inset: 0,
            bgcolor: "rgba(0,0,0,0.5)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1300,
          }}
        >
          <Card sx={{ width: 480, maxHeight: "80vh", overflowY: "auto" }}>
            <CardContent>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Compose exam
              </Typography>
              <Stack spacing={2}>
                <TextField
                  label="Name"
                  value={composeName}
                  onChange={(e) => setComposeName(e.target.value)}
                  fullWidth
                  size="small"
                />
                <TextField
                  label="Description"
                  value={composeDesc}
                  onChange={(e) => setComposeDesc(e.target.value)}
                  fullWidth
                  size="small"
                  multiline
                  rows={2}
                />
                <TextField
                  label="Duration (min)"
                  type="number"
                  value={composeDuration}
                  onChange={(e) => setComposeDuration(Number(e.target.value))}
                  fullWidth
                  size="small"
                />
                <Divider />
                <Typography variant="subtitle2">Sections</Typography>
                {sections.map((s, i) => (
                  <Stack key={i} spacing={1}>
                    <TextField
                      label="Section title"
                      value={s.title}
                      onChange={(e) => {
                        const ns = [...sections];
                        ns[i].title = e.target.value;
                        setSections(ns);
                      }}
                      fullWidth
                      size="small"
                    />
                    <TextField
                      label="Question IDs (comma-separated)"
                      value={s.questionIds}
                      onChange={(e) => {
                        const ns = [...sections];
                        ns[i].questionIds = e.target.value;
                        setSections(ns);
                      }}
                      fullWidth
                      size="small"
                    />
                  </Stack>
                ))}
                <Button
                  onClick={() =>
                    setSections([
                      ...sections,
                      { title: "", weight: 1, questionIds: "" },
                    ])
                  }
                  variant="outlined"
                  size="small"
                >
                  Add section
                </Button>
                {composeError && <Alert severity="error">{composeError}</Alert>}
              </Stack>
              <Stack
                direction="row"
                spacing={1}
                sx={{ justifyContent: "flex-end", mt: 2 }}
              >
                <Button onClick={() => setShowCompose(false)}>Cancel</Button>
                <Button
                  variant="contained"
                  disabled={composing}
                  onClick={handleCompose}
                >
                  {composing ? "Composing…" : "Compose"}
                </Button>
              </Stack>
            </CardContent>
          </Card>
        </Box>
      )}

      {shareExam && (
        <ShareModal
          payload={{ kind: "exam", id: shareExam.id, title: shareExam.name }}
          onClose={() => setShareExam(null)}
        />
      )}
    </Box>
  );
}
