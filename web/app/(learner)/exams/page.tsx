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
import TextField from "@mui/material/TextField";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import Divider from "@mui/material/Divider";
import { LearningObjectives } from "@/components/LearningObjectives";

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

type TabId = "all" | "published" | "draft";

interface AvailableQuestion {
  id: string;
  kind: string;
  prompt: string;
  points: number;
}

interface SectionDraft {
  title: string;
  weight: number;
  selectedIds: Set<string>;
}

export default function ExamsPage() {
  const { user } = useAuth();
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
    { title: "", weight: 60, selectedIds: new Set() },
  ]);
  const [availableQuestions, setAvailableQuestions] = useState<
    AvailableQuestion[]
  >([]);
  const [composeError, setComposeError] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);

  const [starting, setStarting] = useState(false);

  const isInstructor = user?.role === "instructor" || user?.role === "admin";

  function load() {
    setLoading(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (api as any)
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
  }, []);

  const tabs: { id: TabId; label: string }[] = [
    { id: "all", label: "All" },
    { id: "published", label: "Active" },
    { id: "draft", label: "Drafts" },
  ];

  const filtered =
    tab === "all" ? exams : exams.filter((e) => e.status === tab);
  const exam = exams.find((e) => e.id === selected) ?? null;
  const examQuestionCount =
    exam?.sections && exam.sections.length > 0
      ? exam.sections.reduce((sum, s) => sum + (s.items ?? 0), 0)
      : null;

  async function startExam() {
    if (!selected) return;
    setStarting(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (api as any).POST("/v1/sessions", {
        body: { examId: selected },
      });
      if (data?.sessionId) router.push(`/sessions/${data.sessionId}`);
    } catch (err) {
      console.error(err);
    } finally {
      setStarting(false);
    }
  }

  function openCompose() {
    setShowCompose(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (api as any)
      .GET("/v1/questions", {
        params: { query: { status: "live", limit: 200 } },
      })
      .then(({ data }: { data?: { questions: AvailableQuestion[] } }) => {
        setAvailableQuestions(data?.questions ?? []);
      })
      .catch(console.error);
  }

  async function handleCompose() {
    setComposing(true);
    setComposeError(null);
    try {
      const body = {
        name: composeName,
        description: composeDesc || undefined,
        duration: composeDuration,
        sections: sections.map((s, i) => ({
          title: s.title || `Section ${i + 1}`,
          weight: s.weight / 100,
          questionIds: Array.from(s.selectedIds),
          items: null,
          tags: [],
          types: [],
        })),
      };
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data, error } = await (api as any).POST("/v1/exams", { body });
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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (api as any).PATCH(`/v1/exams/${id}`, {
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
            <Button fullWidth variant="outlined" onClick={openCompose}>
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
          <Stack spacing={3} sx={{ maxWidth: 820 }}>
            <Box>
              <Stack
                direction="row"
                spacing={1}
                sx={{ mb: 1, flexWrap: "wrap", alignItems: "center" }}
              >
                <Chip
                  label={exam.status === "published" ? "Active" : exam.status}
                  size="small"
                  color={exam.status === "published" ? "success" : "default"}
                  variant="outlined"
                />
                {exam.course && (
                  <Chip label={exam.course} size="small" variant="outlined" />
                )}
                {exam.method && (
                  <Chip
                    label={`Composed: ${exam.method}`}
                    size="small"
                    variant="outlined"
                  />
                )}
              </Stack>
              <Typography variant="h4" sx={{ fontWeight: 500 }}>
                {exam.name}
              </Typography>
              {exam.description && (
                <Typography
                  variant="body1"
                  color="text.secondary"
                  sx={{ mt: 1, lineHeight: 1.6 }}
                >
                  {exam.description}
                </Typography>
              )}
            </Box>

            {/* At-a-glance metrics */}
            <Card variant="outlined">
              <CardContent>
                <Stack
                  direction="row"
                  divider={<Divider orientation="vertical" flexItem />}
                  spacing={3}
                  sx={{ flexWrap: "wrap", rowGap: 2 }}
                >
                  {[
                    {
                      label: "Questions",
                      value:
                        examQuestionCount != null ? examQuestionCount : "—",
                    },
                    {
                      label: "Duration",
                      value: exam.durationMin ? `${exam.durationMin} min` : "—",
                    },
                    {
                      label: "Total points",
                      value: exam.totalPoints || "—",
                    },
                    {
                      label: "Pass mark",
                      value:
                        exam.passingPoints != null
                          ? `${exam.passingPoints} pts`
                          : "—",
                    },
                    {
                      label: "Sections",
                      value: exam.sections?.length ?? "—",
                    },
                  ].map((m) => (
                    <Box key={m.label} sx={{ minWidth: 84 }}>
                      <Typography variant="caption" color="text.secondary">
                        {m.label}
                      </Typography>
                      <Typography variant="h6" sx={{ fontWeight: 600 }}>
                        {m.value}
                      </Typography>
                    </Box>
                  ))}
                </Stack>
              </CardContent>
            </Card>

            {exam.objectives.length > 0 && (
              <Box>
                <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
                  What you&apos;ll be assessed on
                </Typography>
                <LearningObjectives items={exam.objectives} />
              </Box>
            )}

            {exam.sections && exam.sections.length > 0 && (
              <Box>
                <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
                  Exam format
                </Typography>
                <Stack spacing={1.25}>
                  {exam.sections.map((s, i) => (
                    <Card key={s.id} variant="outlined">
                      <CardContent sx={{ pb: "16px !important" }}>
                        <Box
                          sx={{
                            display: "flex",
                            justifyContent: "space-between",
                            alignItems: "baseline",
                            mb: 1,
                          }}
                        >
                          <Typography variant="body2" sx={{ fontWeight: 500 }}>
                            {i + 1}. {s.title}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {s.items} questions
                            {s.mix ? ` · ${s.mix}` : ""} ·{" "}
                            {Math.round(s.weight * 100)}%
                          </Typography>
                        </Box>
                        {/* Weight bar */}
                        <Box
                          sx={{
                            height: 6,
                            borderRadius: 3,
                            bgcolor: "action.hover",
                            overflow: "hidden",
                          }}
                        >
                          <Box
                            sx={{
                              width: `${Math.round(s.weight * 100)}%`,
                              height: "100%",
                              bgcolor: "primary.main",
                            }}
                          />
                        </Box>
                      </CardContent>
                    </Card>
                  ))}
                </Stack>
              </Box>
            )}

            {exam.tags && exam.tags.length > 0 && (
              <Stack direction="row" spacing={1} sx={{ flexWrap: "wrap" }}>
                {exam.tags.map((t) => (
                  <Chip key={t} label={t} size="small" variant="outlined" />
                ))}
              </Stack>
            )}

            {exam.status === "published" && (
              <Alert severity="info" variant="outlined">
                {exam.durationMin
                  ? `This is a timed exam — once you start, you have ${exam.durationMin} minutes to complete all sections.`
                  : "Once you start, complete all sections in one sitting."}
              </Alert>
            )}

            {(exam.status === "published" ||
              (isInstructor && exam.status !== "published")) && (
              <Box
                sx={{
                  display: "flex",
                  justifyContent: "flex-end",
                  gap: 1,
                  pt: 2,
                  borderTop: 1,
                  borderColor: "divider",
                }}
              >
                {exam.status === "published" && (
                  <Button
                    variant="contained"
                    size="large"
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
                  <Box
                    key={i}
                    sx={{
                      border: 1,
                      borderColor: "divider",
                      borderRadius: 1,
                      p: 1.5,
                    }}
                  >
                    <Stack spacing={1.5}>
                      <Stack direction="row" spacing={1}>
                        <TextField
                          label="Section title"
                          value={s.title}
                          onChange={(e) => {
                            const ns = [...sections];
                            ns[i] = { ...ns[i], title: e.target.value };
                            setSections(ns);
                          }}
                          fullWidth
                          size="small"
                        />
                        <TextField
                          label="Weight %"
                          type="number"
                          value={s.weight}
                          onChange={(e) => {
                            const ns = [...sections];
                            ns[i] = {
                              ...ns[i],
                              weight: Number(e.target.value),
                            };
                            setSections(ns);
                          }}
                          size="small"
                          sx={{ width: 100 }}
                          slotProps={{ htmlInput: { min: 1, max: 100 } }}
                        />
                      </Stack>
                      <Box>
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ display: "block", mb: 0.75 }}
                        >
                          Questions ({s.selectedIds.size} selected)
                        </Typography>
                        <Box
                          sx={{
                            maxHeight: 180,
                            overflowY: "auto",
                            border: 1,
                            borderColor: "divider",
                            borderRadius: 1,
                          }}
                        >
                          {availableQuestions.length === 0 ? (
                            <Typography
                              variant="caption"
                              color="text.secondary"
                              sx={{ p: 1.5, display: "block" }}
                            >
                              No live questions available.
                            </Typography>
                          ) : (
                            availableQuestions.map((q) => {
                              const checked = s.selectedIds.has(q.id);
                              return (
                                <Box
                                  key={q.id}
                                  onClick={() => {
                                    const ns = [...sections];
                                    const ids = new Set(ns[i].selectedIds);
                                    checked ? ids.delete(q.id) : ids.add(q.id);
                                    ns[i] = { ...ns[i], selectedIds: ids };
                                    setSections(ns);
                                  }}
                                  sx={{
                                    px: 1.5,
                                    py: 0.75,
                                    cursor: "pointer",
                                    display: "flex",
                                    gap: 1,
                                    alignItems: "flex-start",
                                    borderBottom: 1,
                                    borderColor: "divider",
                                    bgcolor: checked
                                      ? "action.selected"
                                      : "transparent",
                                    "&:last-child": { borderBottom: 0 },
                                    "&:hover": {
                                      bgcolor: checked
                                        ? "action.selected"
                                        : "action.hover",
                                    },
                                  }}
                                >
                                  <Typography
                                    variant="caption"
                                    color="text.secondary"
                                    sx={{
                                      fontFamily: "monospace",
                                      pt: 0.1,
                                      flexShrink: 0,
                                    }}
                                  >
                                    {q.kind.toUpperCase()}
                                  </Typography>
                                  <Typography
                                    variant="caption"
                                    sx={{ flex: 1, lineHeight: 1.4 }}
                                  >
                                    {q.prompt.length > 80
                                      ? q.prompt.slice(0, 80) + "…"
                                      : q.prompt}
                                  </Typography>
                                  <Typography
                                    variant="caption"
                                    color="text.secondary"
                                    sx={{ flexShrink: 0 }}
                                  >
                                    {q.points}pt
                                  </Typography>
                                </Box>
                              );
                            })
                          )}
                        </Box>
                      </Box>
                    </Stack>
                  </Box>
                ))}
                <Button
                  onClick={() =>
                    setSections([
                      ...sections,
                      { title: "", weight: 40, selectedIds: new Set() },
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
    </Box>
  );
}
