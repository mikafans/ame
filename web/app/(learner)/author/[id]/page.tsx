"use client";

import React, { useState, useEffect, use } from "react";
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
import Alert from "@mui/material/Alert";
import AddOutlinedIcon from "@mui/icons-material/AddOutlined";
import AutoAwesomeOutlinedIcon from "@mui/icons-material/AutoAwesomeOutlined";
import ArrowForwardOutlinedIcon from "@mui/icons-material/ArrowForwardOutlined";
import CheckOutlinedIcon from "@mui/icons-material/CheckOutlined";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";

interface AssessmentQuestion {
  id: string;
  kind: string;
  prompt: string;
  payload: unknown;
  explanation?: string;
  deepDive?: string;
  source?: string;
  points: number;
  status: string;
  orderIndex: number;
}

interface Assessment {
  id: string;
  title: string;
  status: string;
  course?: string;
  description?: string;
  duration?: number;
  difficulty?: string;
  attempts?: number;
  questions: AssessmentQuestion[];
  updated_at?: string;
}

function kindLabel(kind: string): string {
  const map: Record<string, string> = {
    mc: "MC",
    tf: "T/F",
    short: "Short",
    essay: "Essay",
    code: "Code",
  };
  return map[kind] ?? kind;
}

function getMinutesAgo(iso: string): string {
  if (!iso) return "unknown";
  const now = new Date();
  const then = new Date(iso);
  const mins = Math.floor((now.getTime() - then.getTime()) / 60000);
  if (mins < 1) return "now";
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

export default function AuthorStudioPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { user } = useAuth();
  const router = useRouter();
  const [assessment, setAssessment] = useState<Assessment | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const [editTitle, setEditTitle] = useState("");
  const [editCourse, setEditCourse] = useState("");
  const [editDuration, setEditDuration] = useState(30);
  const [editDifficulty, setEditDifficulty] = useState("intermediate");
  const [editAttempts, setEditAttempts] = useState(2);

  const [editPrompt, setEditPrompt] = useState("");
  const [editExplanation, setEditExplanation] = useState("");
  const [editDeepDive, setEditDeepDive] = useState("");
  const [editSource, setEditSource] = useState("");
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [editPoints, setEditPoints] = useState(1);
  const [editTag, setEditTag] = useState("");
  const [editQuestionDifficulty, setEditQuestionDifficulty] =
    useState("intermediate");
  const [editPayload, setEditPayload] = useState<Record<string, unknown>>({});

  const [, setSaving] = useState(false);
  const [publishing, setPublishing] = useState(false);
  const [publishError, setPublishError] = useState<string | null>(null);
  const [showKindPicker, setShowKindPicker] = useState(false);

  function load(showSpinner = false) {
    if (showSpinner) setLoading(true);
    api
      .GET("/v1/assessments/{id}", { params: { path: { id } } })
      .then(({ data }) => {
        if (data) {
          const mappedAssessment: Assessment = {
            id: (data as any).id,
            title: (data as any).title,
            description: (data as any).description ?? undefined,
            status: (data as any).status,
            course: (data as any).course ?? undefined,
            questions: (data as any).questions.map((q: any) => ({
              id: q.id,
              kind: q.kind,
              prompt: q.prompt,
              codeSnippet: q.codeSnippet ?? undefined,
              payload: q.payload,
              explanation: q.explanation ?? undefined,
              deepDive: q.deepDive ?? undefined,
              source: q.source ?? undefined,
              points: q.points,
              status: q.status,
              orderIndex: q.orderIndex,
            })),
          };
          setAssessment(mappedAssessment);
          setEditTitle(mappedAssessment.title || "");
          setEditCourse(mappedAssessment.course || "");
          if (mappedAssessment.questions.length && !selectedId) {
            setSelectedId(mappedAssessment.questions[0].id);
          }
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    load(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  const selectedQ =
    assessment?.questions.find((q) => q.id === selectedId) ?? null;

  useEffect(() => {
    if (selectedQ) {
      setEditPrompt(selectedQ.prompt);
      setEditExplanation(selectedQ.explanation ?? "");
      setEditDeepDive(selectedQ.deepDive ?? "");
      setEditSource(selectedQ.source ?? "");
      setSourceError(null);
      setEditPoints(selectedQ.points);
      setEditTag("");
      setEditQuestionDifficulty("intermediate");
      setEditPayload((selectedQ.payload as Record<string, unknown>) ?? {});
    }
  }, [selectedQ]);

  async function saveMetadata() {
    setSaving(true);
    try {
      await api.PATCH("/v1/assessments/{id}", {
        params: { path: { id } },
        body: {
          title: editTitle,
        },
      });
      load();
    } catch {
      // noop
    } finally {
      setSaving(false);
    }
  }

  async function saveQuestion(payloadOverride?: Record<string, unknown>) {
    if (!selectedId) return;

    // Validate URL scheme if present
    if (editSource) {
      const isSchemeValid =
        editSource.startsWith("http://") || editSource.startsWith("https://");
      if (!isSchemeValid) {
        setSourceError("URL must start with http:// or https://");
        return;
      }
    }
    setSourceError(null);

    setSaving(true);
    const tags = editTag
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);
    try {
      await api.PATCH("/v1/questions/{id}", {
        params: { path: { id: selectedId } },
        body: {
          prompt: editPrompt,
          explanation: editExplanation || undefined,
          deepDive: editDeepDive || undefined,
          source: editSource || undefined,
          points: editPoints,
          tags: tags.length > 0 ? tags : undefined,
          payload: payloadOverride ?? editPayload,
        },
      });
      load();
    } catch {
      // noop
    } finally {
      setSaving(false);
    }
  }

  async function addQuestion(kind: string) {
    setShowKindPicker(false);
    setSaving(true);
    try {
      const { data } = await api.POST("/v1/assessments/{id}/questions", {
        params: { path: { id } },
        body: { kind: kind as any, prompt: "" },
      });
      load();
      if (data?.questionId) setSelectedId(data.questionId);
    } catch {
      // noop
    } finally {
      setSaving(false);
    }
  }

  async function deleteQuestion(idToDelete: string) {
    const remaining =
      assessment?.questions.filter((q) => q.id !== idToDelete) ?? [];
    if (selectedId === idToDelete)
      setSelectedId(remaining.length > 0 ? remaining[0].id : null);
    setAssessment((q) =>
      q
        ? { ...q, questions: q.questions.filter((qq) => qq.id !== idToDelete) }
        : q,
    );
    try {
      await api.DELETE("/v1/assessments/{id}/questions/{question_id}", {
        params: { path: { id, question_id: idToDelete } },
      });
      load();
    } catch (err) {
      console.error("delete failed", err);
      load();
    }
  }

  async function changeKind(newKind: string) {
    if (!selectedId || !selectedQ) return;
    if (newKind === selectedQ.kind) return;
    setSaving(true);
    try {
      const { data } = await api.POST("/v1/assessments/{id}/questions", {
        params: { path: { id } },
        body: {
          kind: newKind as any,
          prompt: editPrompt,
        },
      });
      await api.POST("/v1/questions/{id}/archive", {
        params: { path: { id: selectedId } },
      });
      load();
      if (data?.questionId) setSelectedId(data.questionId);
    } catch {
      // noop
    } finally {
      setSaving(false);
    }
  }

  async function publish() {
    setPublishing(true);
    setPublishError(null);
    const { error } = await api.PATCH("/v1/assessments/{id}", {
      params: { path: { id } },
      body: { status: "active" },
    });
    setPublishing(false);
    if (error) {
      const msg =
        typeof error === "object" && error && "message" in error
          ? String((error as { message: string }).message)
          : "Failed to publish";
      setPublishError(msg);
      return;
    }
    router.push(`/assessments/${id}/preview`);
  }

  const outlineComplete =
    editTitle.trim().length > 0 && (assessment?.questions.length ?? 0) > 0;
  const incompleteQuestions =
    assessment?.questions
      .map((q, idx) => {
        const issues: string[] = [];
        if (!q.prompt?.trim()) issues.push("no prompt");
        if (!q.points) issues.push("0 pts");
        return issues.length > 0 ? { idx: idx + 1, id: q.id, issues } : null;
      })
      .filter(Boolean) ?? [];
  const questionsNeedingReview = incompleteQuestions.length;
  const totalPoints =
    assessment?.questions.reduce((sum, q) => sum + (q.points || 0), 0) ?? 0;
  const minutesAgo = getMinutesAgo(assessment?.updated_at ?? "");

  if (loading) {
    return (
      <Box sx={{ p: "48px 36px", color: "text.secondary", fontSize: 13 }}>
        Loading…
      </Box>
    );
  }

  if (!assessment) {
    return (
      <Box sx={{ p: "48px 36px", color: "text.secondary", fontSize: 14 }}>
        Assessment not found.
      </Box>
    );
  }

  return (
    <Box sx={{ pt: 5, px: { xs: 2, sm: 5 }, pb: 8 }}>
      {/* Header */}
      <Box
        sx={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
          mb: 2.25,
        }}
      >
        <Box>
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              fontFamily: "monospace",
              letterSpacing: 1.3,
              textTransform: "uppercase",
              display: "block",
              mb: 0.5,
            }}
          >
            Editing draft · {editCourse} · autosaved
          </Typography>
          <Typography variant="h5" sx={{ fontWeight: 400 }}>
            Author studio
          </Typography>
        </Box>
        <Stack
          direction="row"
          spacing={1}
          sx={{ alignItems: "center", flexWrap: "wrap" }}
        >
          <Button
            variant="outlined"
            size="small"
            disabled
            title="Not yet available"
          >
            Import
          </Button>
          <Button
            variant="outlined"
            size="small"
            onClick={() => router.push(`/assessments/${id}/preview`)}
          >
            Preview
          </Button>
          <Button
            variant="outlined"
            size="small"
            onClick={async () => {
              await saveMetadata();
              router.push("/author");
            }}
          >
            Save draft
          </Button>
          <Button
            variant="contained"
            size="small"
            onClick={publish}
            disabled={assessment.status === "active" || publishing}
            endIcon={<ArrowForwardOutlinedIcon sx={{ fontSize: 14 }} />}
          >
            {publishing ? "Publishing…" : "Publish"}
          </Button>
        </Stack>
      </Box>

      {publishError && (
        <Alert severity="error" sx={{ mb: 1.75 }}>
          {publishError}
        </Alert>
      )}

      {/* Metadata row */}
      <Card variant="outlined" sx={{ mb: 2.25 }}>
        <Box
          sx={{
            display: "grid",
            gridTemplateColumns: {
              xs: "1fr",
              sm: "2fr 1fr 1fr",
              md: "2fr 1fr 1fr 1fr 1fr",
            },
            borderBottom: 1,
            borderColor: "divider",
          }}
        >
          {[
            {
              label: "Title",
              el: (
                <input
                  value={editTitle}
                  onChange={(e) => setEditTitle(e.target.value)}
                  onBlur={saveMetadata}
                  style={inlineInput}
                />
              ),
            },
            {
              label: "Course",
              el: (
                <input
                  value={editCourse}
                  onChange={(e) => setEditCourse(e.target.value)}
                  onBlur={saveMetadata}
                  style={inlineInput}
                />
              ),
            },
            {
              label: "Duration",
              el: (
                <input
                  type="number"
                  value={editDuration}
                  onChange={(e) =>
                    setEditDuration(parseInt(e.target.value) || 30)
                  }
                  onBlur={saveMetadata}
                  style={inlineInput}
                />
              ),
            },
            {
              label: "Difficulty",
              el: (
                <select
                  value={editDifficulty}
                  onChange={(e) => setEditDifficulty(e.target.value)}
                  onBlur={saveMetadata}
                  style={inlineInput}
                >
                  <option value="intro">Intro</option>
                  <option value="intermediate">Intermediate</option>
                  <option value="advanced">Advanced</option>
                </select>
              ),
            },
            {
              label: "Attempts",
              el: (
                <input
                  type="number"
                  value={editAttempts}
                  onChange={(e) =>
                    setEditAttempts(parseInt(e.target.value) || 1)
                  }
                  onBlur={saveMetadata}
                  style={inlineInput}
                />
              ),
            },
          ].map((field, i, arr) => (
            <Box
              key={field.label}
              component="label"
              sx={{
                p: "14px 22px",
                borderRight: i < arr.length - 1 ? 1 : 0,
                borderColor: "divider",
                display: "block",
              }}
            >
              <Typography
                variant="caption"
                color="text.secondary"
                sx={monoLabel as React.CSSProperties}
              >
                {field.label}
              </Typography>
              {field.el}
            </Box>
          ))}
        </Box>

        {/* Validation bar */}
        <Box
          sx={{
            px: "22px",
            py: 1.75,
            display: "flex",
            gap: 3.5,
            alignItems: "center",
            fontSize: 12.5,
            color: "text.secondary",
          }}
        >
          <Box
            sx={{
              display: "inline-flex",
              alignItems: "center",
              gap: 0.75,
              color: outlineComplete ? "success.main" : "text.disabled",
            }}
          >
            <span>{outlineComplete ? "✓" : "✗"}</span>
            <span>Outline complete</span>
          </Box>
          <Box
            sx={{
              display: "inline-flex",
              alignItems: "center",
              gap: 0.75,
              color:
                questionsNeedingReview === 0 ? "success.main" : "text.disabled",
            }}
          >
            {questionsNeedingReview === 0 ? (
              <>
                <span>✓</span>
                <span>All questions ready</span>
              </>
            ) : (
              <>
                <span>✗</span>
                <span>
                  {incompleteQuestions
                    .map((q) => `Q${q!.idx} (${q!.issues.join(", ")})`)
                    .join(" · ")}
                </span>
              </>
            )}
          </Box>
          <Typography variant="body2" color="text.secondary">
            ≈ {assessment.questions.length} questions · {totalPoints} pts
          </Typography>
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ ml: "auto", fontFamily: "monospace", letterSpacing: 0.5 }}
          >
            Last edit · {minutesAgo} ago · by you
          </Typography>
        </Box>
      </Card>

      {/* Three-pane layout */}
      <Box
        sx={{
          display: "grid",
          gridTemplateColumns: { xs: "1fr", md: "320px 1fr 280px" },
          gap: 2.25,
          minHeight: 600,
        }}
      >
        {/* LEFT - Questions list */}
        <Card
          variant="outlined"
          sx={{ overflow: "hidden", display: "flex", flexDirection: "column" }}
        >
          <Box
            sx={{
              px: 2,
              py: 1.75,
              borderBottom: 1,
              borderColor: "divider",
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <Typography
              variant="caption"
              color="text.secondary"
              sx={monoLabel as React.CSSProperties}
            >
              Questions
            </Typography>
            <Button
              size="small"
              variant="outlined"
              startIcon={<AddOutlinedIcon sx={{ fontSize: 12 }} />}
              onClick={() => setShowKindPicker((v) => !v)}
              sx={{ minWidth: 0 }}
            >
              Add
            </Button>
          </Box>

          {showKindPicker && (
            <Box
              sx={{
                px: 2,
                py: 1.5,
                borderBottom: 1,
                borderColor: "divider",
                bgcolor: "action.hover",
              }}
            >
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{
                  fontFamily: "monospace",
                  display: "block",
                  mb: 1,
                  fontSize: 10,
                  letterSpacing: 1,
                }}
              >
                Choose type
              </Typography>
              <Stack direction="row" spacing={0.75} flexWrap="wrap" useFlexGap>
                {(["mc", "tf", "short", "essay", "code"] as const).map((k) => (
                  <Button
                    key={k}
                    size="small"
                    variant="outlined"
                    onClick={() => addQuestion(k)}
                    sx={{ minWidth: 0, fontSize: 11, px: 1.25, py: 0.5 }}
                  >
                    {kindLabel(k)}
                  </Button>
                ))}
              </Stack>
            </Box>
          )}

          <Box sx={{ flex: 1, overflow: "auto" }}>
            {assessment.questions.length === 0 ? (
              <Box
                sx={{
                  p: "24px 16px",
                  color: "text.secondary",
                  fontSize: 13,
                  textAlign: "center",
                }}
              >
                No questions yet.
              </Box>
            ) : (
              assessment.questions.map((q, idx) => {
                const sel = selectedId === q.id;
                const incomplete = !q.prompt?.trim() || !q.points;
                return (
                  <Box
                    key={q.id}
                    onClick={() => setSelectedId(q.id)}
                    sx={{
                      width: "100%",
                      px: 2,
                      py: 1.5,
                      bgcolor: sel
                        ? (theme) =>
                            theme.palette.mode === "dark"
                              ? "rgba(25, 118, 210, 0.16)"
                              : "rgba(25, 118, 210, 0.08)"
                        : "transparent",
                      borderLeft: `2px solid`,
                      borderLeftColor: sel ? "primary.main" : "transparent",
                      borderBottom: 1,
                      borderColor: "divider",
                      textAlign: "left",
                      cursor: "pointer",
                      display: "grid",
                      gridTemplateColumns: "28px 1fr 40px 28px",
                      gap: 1,
                      alignItems: "start",
                      "&:hover .delete-btn": { opacity: 1 },
                    }}
                  >
                    <Box
                      sx={{
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "center",
                        gap: 0.5,
                      }}
                    >
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ fontFamily: "monospace", pt: 0.25 }}
                      >
                        Q{idx + 1}
                      </Typography>
                      {incomplete && (
                        <Box
                          sx={{
                            width: 6,
                            height: 6,
                            borderRadius: "50%",
                            bgcolor: "warning.main",
                          }}
                          title={[
                            !q.prompt?.trim() ? "no prompt" : null,
                            !q.points ? "0 pts" : null,
                          ]
                            .filter(Boolean)
                            .join(", ")}
                        />
                      )}
                    </Box>
                    <Box sx={{ minWidth: 0 }}>
                      <Typography
                        variant="body2"
                        sx={{
                          color: sel ? "text.primary" : "text.secondary",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          fontWeight: sel ? 500 : 400,
                          mb: 0.5,
                        }}
                      >
                        {q.prompt.slice(0, 40)}
                        {q.prompt.length > 40 ? "…" : ""}
                      </Typography>
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{
                          fontFamily: "monospace",
                          textTransform: "uppercase",
                          letterSpacing: 0.5,
                          fontSize: 10,
                        }}
                      >
                        {kindLabel(q.kind)}
                      </Typography>
                    </Box>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{
                        fontFamily: "monospace",
                        textAlign: "right",
                        pt: 0.25,
                      }}
                    >
                      {q.points}pt
                    </Typography>
                    <Box
                      component="button"
                      type="button"
                      className="delete-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteQuestion(q.id);
                      }}
                      sx={{
                        opacity: 0,
                        transition: "opacity 0.15s",
                        border: "none",
                        bgcolor: "transparent",
                        cursor: "pointer",
                        color: "error.main",
                        p: 0,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        pt: 0.25,
                      }}
                    >
                      <DeleteOutlineIcon sx={{ fontSize: 15 }} />
                    </Box>
                  </Box>
                );
              })
            )}
          </Box>

          {/* Generate footer */}
          <Box
            sx={{
              p: 1.75,
              bgcolor: "action.hover",
              borderTop: 1,
              borderColor: "divider",
            }}
          >
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ display: "block", lineHeight: 1.5, mb: 1 }}
            >
              Generate from source — paste notes or a reading.
            </Typography>
            <Button
              variant="outlined"
              size="small"
              disabled
              title="Not yet available"
              startIcon={<AutoAwesomeOutlinedIcon sx={{ fontSize: 12 }} />}
            >
              Generate questions
            </Button>
          </Box>
        </Card>

        {/* MIDDLE - Question editor */}
        <Card
          variant="outlined"
          sx={{ overflow: "hidden", display: "flex", flexDirection: "column" }}
        >
          {selectedQ ? (
            <>
              <Box
                sx={{
                  px: "22px",
                  py: 2,
                  borderBottom: 1,
                  borderColor: "divider",
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <Box>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{
                      fontFamily: "monospace",
                      letterSpacing: 1.3,
                      textTransform: "uppercase",
                      display: "block",
                      mb: 0.5,
                    }}
                  >
                    Editing Q
                    {assessment.questions.findIndex(
                      (q) => q.id === selectedId,
                    ) + 1}
                  </Typography>
                  <select
                    value={selectedQ.kind}
                    onChange={(e) => changeKind(e.target.value)}
                    style={{
                      ...inlineInput,
                      width: "auto",
                      fontSize: 14,
                      fontWeight: 500,
                    }}
                    title="Change question type (creates a new question)"
                  >
                    <option value="mc">MC</option>
                    <option value="tf">T/F</option>
                    <option value="short">Short</option>
                    <option value="essay">Essay</option>
                    <option value="code">Code</option>
                  </select>
                </Box>
              </Box>

              <Box sx={{ flex: 1, overflow: "auto", p: "22px" }}>
                <Box component="label" sx={{ display: "block", mb: 2.25 }}>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={monoLabel as React.CSSProperties}
                  >
                    Question prompt
                  </Typography>
                  <textarea
                    value={editPrompt}
                    onChange={(e) => setEditPrompt(e.target.value)}
                    onBlur={() => saveQuestion()}
                    rows={3}
                    style={{
                      ...inlineInput,
                      width: "100%",
                      padding: 12,
                      fontSize: 16,
                      resize: "vertical",
                    }}
                  />
                </Box>

                {selectedQ.kind === "mc" && (
                  <McOptionsEditor
                    payload={editPayload}
                    onChange={(p) => {
                      setEditPayload(p);
                      saveQuestion(p);
                    }}
                    onTextChange={(p) => setEditPayload(p)}
                    onTextBlur={(p) => saveQuestion(p)}
                  />
                )}
                {selectedQ.kind === "tf" && (
                  <TfOptionsEditor
                    payload={editPayload}
                    onChange={(p) => {
                      setEditPayload(p);
                      saveQuestion(p);
                    }}
                  />
                )}

                <Box
                  sx={{
                    mt: 2.25,
                    pt: 2.25,
                    borderTop: 1,
                    borderColor: "divider",
                    display: "grid",
                    gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr 1fr" },
                    gap: 2.25,
                  }}
                >
                  <Box component="label" sx={{ display: "block" }}>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={monoLabel as React.CSSProperties}
                    >
                      Points
                    </Typography>
                    <input
                      type="number"
                      min={0}
                      value={editPoints}
                      onChange={(e) =>
                        setEditPoints(
                          Math.max(0, parseInt(e.target.value) || 0),
                        )
                      }
                      onBlur={() => saveQuestion()}
                      style={inlineInput}
                    />
                  </Box>
                  <Box component="label" sx={{ display: "block" }}>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={monoLabel as React.CSSProperties}
                    >
                      Tag
                    </Typography>
                    <input
                      value={editTag}
                      placeholder="tag1, tag2"
                      onChange={(e) => setEditTag(e.target.value)}
                      onBlur={() => saveQuestion()}
                      style={inlineInput}
                    />
                  </Box>
                  <Box component="label" sx={{ display: "block" }}>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={monoLabel as React.CSSProperties}
                    >
                      Difficulty
                    </Typography>
                    <select
                      value={editQuestionDifficulty}
                      onChange={(e) =>
                        setEditQuestionDifficulty(e.target.value)
                      }
                      onBlur={() => saveQuestion()}
                      style={inlineInput}
                    >
                      <option value="intro">Intro</option>
                      <option value="intermediate">Intermediate</option>
                      <option value="advanced">Advanced</option>
                    </select>
                  </Box>
                </Box>

                <Box component="label" sx={{ display: "block", mt: 2.75 }}>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={monoLabel as React.CSSProperties}
                  >
                    Explanation shown after answering
                  </Typography>
                  <textarea
                    value={editExplanation}
                    onChange={(e) => setEditExplanation(e.target.value)}
                    onBlur={() => saveQuestion()}
                    rows={2}
                    style={{
                      ...inlineInput,
                      width: "100%",
                      padding: 12,
                      resize: "vertical",
                    }}
                  />
                </Box>

                {/* External Source URL */}
                <Box component="label" sx={{ display: "block", mt: 2.75 }}>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={monoLabel as React.CSSProperties}
                  >
                    Reference URL (External Source)
                  </Typography>
                  <input
                    value={editSource}
                    placeholder="https://example.com/reference"
                    onChange={(e) => {
                      setEditSource(e.target.value);
                      if (sourceError) setSourceError(null);
                    }}
                    onBlur={() => saveQuestion()}
                    style={{
                      ...inlineInput,
                      borderColor: sourceError ? "#d32f2f" : undefined,
                    }}
                  />
                  {sourceError && (
                    <Typography
                      variant="caption"
                      color="error"
                      sx={{ display: "block", mt: 0.5 }}
                    >
                      {sourceError}
                    </Typography>
                  )}
                </Box>

                {/* Deep Dive Study Notes */}
                <Box component="label" sx={{ display: "block", mt: 2.75 }}>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={monoLabel as React.CSSProperties}
                  >
                    Deep Dive Study Notes (Markdown / Mermaid supported)
                  </Typography>
                  <textarea
                    value={editDeepDive}
                    placeholder="# Detailed Study Notes\n\nUse markdown here..."
                    onChange={(e) => setEditDeepDive(e.target.value)}
                    onBlur={() => saveQuestion()}
                    rows={6}
                    style={{
                      ...inlineInput,
                      width: "100%",
                      padding: 12,
                      resize: "vertical",
                    }}
                  />
                </Box>

                {/* Live Preview Area */}
                {(editDeepDive ||
                  (editSource &&
                    (!editSource ||
                      editSource.startsWith("http://") ||
                      editSource.startsWith("https://")))) && (
                  <Box
                    sx={{
                      mt: 3,
                      p: 2,
                      border: "1px dashed",
                      borderColor: "divider",
                      borderRadius: 1,
                      bgcolor: "background.default",
                    }}
                  >
                    <Typography
                      variant="caption"
                      color="primary"
                      sx={{
                        ...monoLabel,
                        display: "block",
                        mb: 1.5,
                        fontWeight: 600,
                        textTransform: "uppercase",
                      }}
                    >
                      Live Preview
                    </Typography>

                    {editSource &&
                      (!editSource ||
                        editSource.startsWith("http://") ||
                        editSource.startsWith("https://")) && (
                        <Box sx={{ mb: 2 }}>
                          <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{ display: "block", mb: 0.5 }}
                          >
                            Reference Link:
                          </Typography>
                          <a
                            href={editSource}
                            target="_blank"
                            rel="noopener noreferrer"
                            style={{
                              color: "#1976d2",
                              textDecoration: "underline",
                              fontSize: "0.875rem",
                            }}
                          >
                            {editSource}
                          </a>
                        </Box>
                      )}

                    {editDeepDive && (
                      <Box>
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ display: "block", mb: 1 }}
                        >
                          Study Notes:
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
                          <MarkdownView content={editDeepDive} />
                        </Box>
                      </Box>
                    )}
                  </Box>
                )}
              </Box>
            </>
          ) : (
            <Box
              sx={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                p: "48px 22px",
                textAlign: "center",
              }}
            >
              <Typography variant="body2" color="text.secondary">
                Select a question to edit.
              </Typography>
            </Box>
          )}
        </Card>

        {/* RIGHT - Rail */}
        <Stack spacing={2.25}>
          <Card variant="outlined">
            <CardContent>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{
                  fontFamily: "monospace",
                  letterSpacing: 1.3,
                  textTransform: "uppercase",
                  display: "block",
                  mb: 1.5,
                }}
              >
                Distribution
              </Typography>
              <KV label="Most correct" value="#1" />
              <KV label="Hardest item" value="#2" />
            </CardContent>
          </Card>

          <Card variant="outlined">
            <CardContent>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{
                  fontFamily: "monospace",
                  letterSpacing: 1.3,
                  textTransform: "uppercase",
                  display: "block",
                  mb: 1.5,
                }}
              >
                Rubric
              </Typography>
              <Box
                component="pre"
                sx={{
                  m: 0,
                  p: "10px 12px",
                  bgcolor: "action.hover",
                  border: 1,
                  borderColor: "divider",
                  borderRadius: 0.5,
                  fontFamily: "monospace",
                  fontSize: 12,
                  color: "text.secondary",
                  lineHeight: 1.5,
                }}
              >
                {`criteria:\n  clarity: 0–2\n  evidence: 0–2\n  mechanism: 0–1\n  link: 0–1`}
              </Box>
            </CardContent>
          </Card>

          <Card variant="outlined">
            <CardContent>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{
                  fontFamily: "monospace",
                  letterSpacing: 1.3,
                  textTransform: "uppercase",
                  display: "block",
                  mb: 1.5,
                }}
              >
                Recent activity
              </Typography>
              <Stack spacing={1.25}>
                <ActivityItem name="You" action="edited Q1" time="4m" />
                <ActivityItem name="You" action="added 2 questions" time="2h" />
                <ActivityItem
                  name="System"
                  action="auto-saved draft"
                  time="5m"
                />
              </Stack>
            </CardContent>
          </Card>
        </Stack>
      </Box>
    </Box>
  );
}

function McOptionsEditor({
  payload,
  onChange,
  onTextChange,
  onTextBlur,
}: {
  payload: unknown;
  onChange: (p: Record<string, unknown>) => void;
  onTextChange: (p: Record<string, unknown>) => void;
  onTextBlur: (p: Record<string, unknown>) => void;
}) {
  const p = payload as {
    options?: string[];
    correct_index?: number;
  } | null;
  if (!p?.options) return null;

  function updateText(i: number, text: string) {
    const options = p!.options!.map((o, j) => (j === i ? text : o));
    return { ...p, options };
  }

  return (
    <Box sx={{ mb: 2.25 }}>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={monoLabel as React.CSSProperties}
      >
        Options · click row to mark correct
      </Typography>
      <Stack spacing={1}>
        {p.options.map((o, i) => (
          <Box
            key={i}
            sx={{
              display: "grid",
              gridTemplateColumns: "30px 1fr",
              gap: 1.25,
              alignItems: "center",
              p: "6px 12px 6px 10px",
              bgcolor:
                i === p.correct_index
                  ? (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(25, 118, 210, 0.16)"
                        : "rgba(25, 118, 210, 0.08)"
                  : "action.hover",
              border: 1,
              borderColor: i === p.correct_index ? "primary.main" : "divider",
              borderRadius: 0.5,
              cursor: "pointer",
              "&:hover": { borderColor: "primary.light" },
            }}
            onClick={() => onChange({ ...p, correct_index: i })}
          >
            <Box
              sx={{
                width: 22,
                height: 22,
                borderRadius: "50%",
                bgcolor: i === p.correct_index ? "primary.main" : "transparent",
                border: 1,
                borderColor:
                  i === p.correct_index ? "primary.main" : "action.disabled",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                flexShrink: 0,
              }}
            >
              {i === p.correct_index && (
                <CheckOutlinedIcon sx={{ fontSize: 12, color: "#fff" }} />
              )}
            </Box>
            <input
              value={o}
              placeholder={`Option ${i + 1}`}
              onChange={(e) => onTextChange(updateText(i, e.target.value))}
              onBlur={(e) => onTextBlur(updateText(i, e.target.value))}
              onClick={(e) => e.stopPropagation()}
              style={{
                background: "transparent",
                border: "none",
                outline: "none",
                fontSize: 13.5,
                color: "inherit",
                fontFamily: "inherit",
                width: "100%",
                cursor: "text",
              }}
            />
          </Box>
        ))}
      </Stack>
    </Box>
  );
}

function TfOptionsEditor({
  payload,
  onChange,
}: {
  payload: unknown;
  onChange: (p: Record<string, unknown>) => void;
}) {
  const p = payload as { correct?: boolean } | null;
  const correct = p?.correct ?? true;
  return (
    <Box sx={{ mb: 2.25 }}>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={monoLabel as React.CSSProperties}
      >
        Correct answer
      </Typography>
      <Stack direction="row" spacing={1}>
        {([true, false] as const).map((val) => (
          <Box
            key={String(val)}
            component="button"
            type="button"
            onClick={() => onChange({ correct: val })}
            sx={{
              flex: 1,
              py: 1.5,
              border: 1,
              borderColor: correct === val ? "primary.main" : "divider",
              borderRadius: 0.5,
              bgcolor:
                correct === val
                  ? (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(25, 118, 210, 0.16)"
                        : "rgba(25, 118, 210, 0.08)"
                  : "action.hover",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: 1,
              "&:hover": { borderColor: "primary.light" },
            }}
          >
            {correct === val && (
              <CheckOutlinedIcon sx={{ fontSize: 14, color: "primary.main" }} />
            )}
            <Typography
              variant="body2"
              sx={{ fontWeight: correct === val ? 600 : 400 }}
            >
              {val ? "True" : "False"}
            </Typography>
          </Box>
        ))}
      </Stack>
    </Box>
  );
}

function KV({ label, value }: { label: string; value: string }) {
  return (
    <Box sx={{ display: "flex", justifyContent: "space-between", mb: 0.5 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="caption">{value}</Typography>
    </Box>
  );
}

function ActivityItem({
  name,
  action,
  time,
}: {
  name: string;
  action: string;
  time: string;
}) {
  const isYou = name === "You";
  return (
    <Box sx={{ display: "flex", gap: 1 }}>
      <Box
        sx={{
          width: 4,
          alignSelf: "stretch",
          bgcolor: isYou ? "primary.main" : "action.disabled",
          borderRadius: 0.25,
        }}
      />
      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography variant="caption" sx={{ color: "text.primary" }}>
          <Box
            component="span"
            sx={{ color: isYou ? "primary.main" : "text.secondary" }}
          >
            {name}
          </Box>{" "}
          {action}
        </Typography>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: "monospace",
            fontSize: 10.5,
            display: "block",
            mt: 0.25,
            letterSpacing: 0.5,
          }}
        >
          {time} ago
        </Typography>
      </Box>
    </Box>
  );
}

const monoLabel = {
  fontFamily: "monospace",
  fontSize: 10,
  letterSpacing: 1.3,
  textTransform: "uppercase",
  color: "rgba(0,0,0,0.5)",
  marginBottom: 6,
  display: "block",
} as const;

const inlineInput: React.CSSProperties = {
  background: "transparent",
  border: "1px solid rgba(0,0,0,0.23)",
  borderRadius: 6,
  padding: "9px 12px",
  color: "inherit",
  fontFamily: "inherit",
  fontSize: 13.5,
  outline: "none",
  boxSizing: "border-box",
  width: "100%",
};
