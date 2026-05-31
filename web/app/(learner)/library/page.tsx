"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import PublicIcon from "@mui/icons-material/Public";
import LockOutlinedIcon from "@mui/icons-material/LockOutlined";
import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";
import AddOutlinedIcon from "@mui/icons-material/AddOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import { LearningObjectives } from "@/components/LearningObjectives";
import { formatMinutes } from "@/utils/format";
import { tagColor } from "@/lib/tagColor";

interface Assessment {
  id: string;
  title: string;
  description?: string;
  status: string;
  visibility: "public" | "private";
  course?: string;
  difficulty?: string;
  objectives?: string[];
  questionCount?: number;
  durationMin?: number;
  completed?: boolean;
  lastSessionId?: string | null;
  createdAt: string;
}

type TabId = "all" | "completed" | "drafts";
type VisibilityFilter = "all" | "public" | "mine";

export default function LibraryPage() {
  const router = useRouter();
  const [tab, setTab] = useState<TabId>("all");
  const [visibilityFilter, setVisibilityFilter] =
    useState<VisibilityFilter>("all");
  const [activeAssessments, setActiveAssessments] = useState<Assessment[]>([]);
  const [draftAssessments, setDraftAssessments] = useState<Assessment[]>([]);
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);

  const fetchAssessments = useCallback(async () => {
    setLoading(true);
    const fetchStatus = async (status: string): Promise<Assessment[]> => {
      const { data } = await api.GET("/v1/assessments", {
        params: { query: { mode: "practice", status } },
      });
      if (data && "assessments" in data) {
        return data.assessments as Assessment[];
      }
      return Array.isArray(data) ? (data as Assessment[]) : [];
    };

    try {
      const [active, drafts] = await Promise.all([
        fetchStatus("active"),
        fetchStatus("draft"),
      ]);
      setActiveAssessments(active);
      setDraftAssessments(drafts);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAssessments();
  }, [fetchAssessments]);

  async function startAssessment(assessmentId: string) {
    setStarting(assessmentId);
    setStartError(null);
    try {
      const { data, error } = await api.POST("/v1/sessions", {
        body: { assessmentId },
      });
      if (error) {
        setStartError((error as any)?.message ?? "Failed to start assessment");
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

  // Apply visibility filter client-side (data already includes both public + own)
  const applyVisibility = (list: Assessment[]) => {
    if (visibilityFilter === "public")
      return list.filter((a) => a.visibility === "public");
    if (visibilityFilter === "mine")
      return list.filter((a) => a.visibility === "private");
    return list;
  };

  const filteredActive = applyVisibility(activeAssessments);
  const filteredDrafts = applyVisibility(draftAssessments);

  const pending = filteredActive.filter((a) => !a.completed);
  const completed = filteredActive.filter((a) => a.completed);

  const listed =
    tab === "completed"
      ? completed
      : tab === "drafts"
        ? filteredDrafts
        : filteredActive;

  const featuredAssessment = pending[0] ?? null;

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

  return (
    <Box sx={{ p: 4 }}>
      {/* ── Header ── */}
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          mb: 3,
        }}
      >
        <Typography variant="h4" sx={{ fontWeight: 500 }}>
          Assessments
        </Typography>
        <Button
          variant="outlined"
          startIcon={<AddOutlinedIcon />}
          onClick={async () => {
            const { data } = await api.POST("/v1/assessments", {
              body: {
                title: "Untitled assessment",
                description: null,
                mode: "practice",
                objectives: [],
                course: null,
                durationMin: null,
                timeLimitSeconds: null,
                passingPoints: null,
                showResultsDuring: false,
                affectsRating: true,
                visibility: "private",
                method: "manual",
              },
            });
            if (data?.id) router.push(`/author/${data.id}`);
          }}
        >
          New assessment
        </Button>
      </Box>

      {/* ── Up-next hero ── */}
      {featuredAssessment && (
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
              {featuredAssessment.title}
            </Typography>
            {featuredAssessment.description && (
              <Typography
                variant="body2"
                color="text.secondary"
                sx={{ mb: 1.5 }}
              >
                {featuredAssessment.description}
              </Typography>
            )}
            <Stack direction="row" spacing={3} sx={{ mb: 2 }}>
              <Box>
                <Typography variant="caption" color="text.secondary">
                  Questions
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {featuredAssessment.questionCount ?? "—"}
                </Typography>
              </Box>
              <Box>
                <Typography variant="caption" color="text.secondary">
                  Duration
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  {featuredAssessment.durationMin
                    ? formatMinutes(featuredAssessment.durationMin)
                    : "—"}
                </Typography>
              </Box>
            </Stack>
            {featuredAssessment.objectives &&
              featuredAssessment.objectives.length > 0 && (
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
                  <LearningObjectives items={featuredAssessment.objectives} />
                </Box>
              )}
            <Stack direction="row" spacing={1}>
              <Button
                size="small"
                variant="outlined"
                onClick={() =>
                  router.push(`/assessments/${featuredAssessment.id}/preview`)
                }
              >
                Preview questions
              </Button>
              <Button
                size="small"
                variant="contained"
                startIcon={<PlayArrowOutlinedIcon />}
                disabled={starting === featuredAssessment.id}
                onClick={() => startAssessment(featuredAssessment.id)}
              >
                {starting === featuredAssessment.id ? "Starting…" : "Start"}
              </Button>
            </Stack>
          </CardContent>
        </Card>
      )}

      {/* ── Tabs + Visibility filter ── */}
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          borderBottom: 1,
          borderColor: "divider",
          mb: 3,
        }}
      >
        <Tabs
          value={tab}
          onChange={(_, v) => setTab(v)}
          sx={{ "& .MuiTab-root": { textTransform: "none" } }}
        >
          <Tab
            value="all"
            label={`All (${filteredActive.length})`}
            id="tab-all"
          />
          {completed.length > 0 && (
            <Tab
              value="completed"
              label={`Completed (${completed.length})`}
              id="tab-completed"
            />
          )}
          {filteredDrafts.length > 0 && (
            <Tab
              value="drafts"
              label={`Drafts (${filteredDrafts.length})`}
              id="tab-drafts"
            />
          )}
        </Tabs>

        {/* Visibility segmented control */}
        <ToggleButtonGroup
          value={visibilityFilter}
          exclusive
          onChange={(_, v) => {
            if (v !== null) {
              setVisibilityFilter(v);
              // Reset to "all" tab if currently on drafts and switching filter
              if (tab === "drafts" && v === "public") setTab("all");
            }
          }}
          size="small"
          aria-label="visibility filter"
          sx={{ mb: 0.5 }}
        >
          <ToggleButton value="all" id="vis-filter-all" sx={{ px: 1.5 }}>
            <Typography variant="caption" sx={{ textTransform: "none" }}>
              All sets
            </Typography>
          </ToggleButton>
          <ToggleButton value="public" id="vis-filter-public" sx={{ px: 1.5 }}>
            <PublicIcon sx={{ fontSize: 14, mr: 0.5 }} />
            <Typography variant="caption" sx={{ textTransform: "none" }}>
              Public
            </Typography>
          </ToggleButton>
          <ToggleButton value="mine" id="vis-filter-mine" sx={{ px: 1.5 }}>
            <LockOutlinedIcon sx={{ fontSize: 14, mr: 0.5 }} />
            <Typography variant="caption" sx={{ textTransform: "none" }}>
              Mine
            </Typography>
          </ToggleButton>
        </ToggleButtonGroup>
      </Box>

      {startError && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {startError}
        </Alert>
      )}

      {/* ── Assessment list ── */}
      {listed.length === 0 ? (
        <Typography color="text.secondary">
          No assessments match this filter.
        </Typography>
      ) : (
        <Stack spacing={2}>
          {listed.map((assessment) => (
            <Card key={assessment.id} variant="outlined">
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
                        flexWrap: "wrap",
                      }}
                    >
                      <Typography variant="subtitle1" sx={{ fontWeight: 500 }}>
                        {assessment.title}
                      </Typography>

                      {/* Course chip */}
                      {assessment.course && (
                        <Chip
                          label={assessment.course}
                          size="small"
                          variant="outlined"
                          sx={tagColor(assessment.course)}
                        />
                      )}

                      {/* Visibility chip */}
                      <Chip
                        icon={
                          assessment.visibility === "public" ? (
                            <PublicIcon sx={{ fontSize: "0.85rem !important" }} />
                          ) : (
                            <LockOutlinedIcon
                              sx={{ fontSize: "0.85rem !important" }}
                            />
                          )
                        }
                        label={
                          assessment.visibility === "public"
                            ? "Public"
                            : "Private"
                        }
                        size="small"
                        variant="outlined"
                        color={
                          assessment.visibility === "public"
                            ? "info"
                            : "default"
                        }
                      />

                      {/* Completed badge */}
                      {assessment.completed && (
                        <Chip
                          label="Completed"
                          size="small"
                          color="success"
                          variant="outlined"
                        />
                      )}
                    </Box>

                    {assessment.description && (
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{ mb: 1 }}
                      >
                        {assessment.description}
                      </Typography>
                    )}

                    {assessment.objectives &&
                      assessment.objectives.length > 0 && (
                        <Box sx={{ mb: 1 }}>
                          <LearningObjectives items={assessment.objectives} />
                        </Box>
                      )}

                    <Stack direction="row" spacing={2}>
                      {assessment.questionCount != null && (
                        <Typography variant="caption" color="text.secondary">
                          {assessment.questionCount} questions
                        </Typography>
                      )}
                      {assessment.durationMin != null && (
                        <Typography variant="caption" color="text.secondary">
                          {formatMinutes(assessment.durationMin)}
                        </Typography>
                      )}
                    </Stack>
                  </Box>

                  {/* Action buttons */}
                  <Stack direction="row" spacing={1} sx={{ ml: 2, flexShrink: 0 }}>
                    {assessment.status === "draft" ? (
                      <Button
                        size="small"
                        variant="outlined"
                        startIcon={<EditOutlinedIcon />}
                        onClick={() =>
                          router.push(`/author/${assessment.id}`)
                        }
                      >
                        Edit
                      </Button>
                    ) : (
                      <>
                        <Button
                          size="small"
                          variant="outlined"
                          onClick={() =>
                            router.push(
                              `/assessments/${assessment.id}/preview`,
                            )
                          }
                        >
                          Preview
                        </Button>
                        {assessment.completed && assessment.lastSessionId && (
                          <Button
                            size="small"
                            variant="outlined"
                            color="success"
                            onClick={() =>
                              router.push(
                                `/sessions/${assessment.lastSessionId}/results`,
                              )
                            }
                          >
                            Last result
                          </Button>
                        )}
                        <Button
                          size="small"
                          variant="contained"
                          startIcon={<PlayArrowOutlinedIcon />}
                          disabled={starting === assessment.id}
                          onClick={() => startAssessment(assessment.id)}
                        >
                          {starting === assessment.id
                            ? "Starting…"
                            : assessment.completed
                              ? "Retake"
                              : "Start"}
                        </Button>
                      </>
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
