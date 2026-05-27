"use client";

import React, { useState, useEffect, use } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
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

interface QuizQuestion {
  id: string;
  kind: string;
  prompt: string;
  payload: unknown;
  explanation?: string;
  points: number;
  status: string;
  orderIndex: number;
}

interface Quiz {
  id: string;
  title: string;
  status: string;
  course?: string;
  duration?: number;
  difficulty?: string;
  attempts?: number;
  questions: QuizQuestion[];
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
  params: Promise<{ quizId: string }>;
}) {
  const { quizId } = use(params);
  const { token } = useAuth();
  const [quiz, setQuiz] = useState<Quiz | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const [editTitle, setEditTitle] = useState("");
  const [editCourse, setEditCourse] = useState("");
  const [editDuration, setEditDuration] = useState(30);
  const [editDifficulty, setEditDifficulty] = useState("intermediate");
  const [editAttempts, setEditAttempts] = useState(2);

  const [editPrompt, setEditPrompt] = useState("");
  const [editExplanation, setEditExplanation] = useState("");
  const [editPoints, setEditPoints] = useState(1);
  const [editTag, setEditTag] = useState("");
  const [editQuestionDifficulty, setEditQuestionDifficulty] =
    useState("intermediate");

  const [, setSaving] = useState(false);
  const [publishing, setPublishing] = useState(false);
  const [publishError, setPublishError] = useState<string | null>(null);

  function load() {
    if (!token) return;
    setLoading(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET(`/v1/quizzes/${quizId}`)
      .then(({ data }: { data?: Quiz }) => {
        if (data) {
          setQuiz(data);
          setEditTitle(data.title || "");
          setEditCourse(data.course || "");
          setEditDuration(data.duration || 30);
          setEditDifficulty(data.difficulty || "intermediate");
          setEditAttempts(data.attempts || 2);
          if (data.questions.length && !selectedId) {
            setSelectedId(data.questions[0].id);
          }
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, quizId]);

  const selectedQ = quiz?.questions.find((q) => q.id === selectedId) ?? null;

  useEffect(() => {
    if (selectedQ) {
      setEditPrompt(selectedQ.prompt);
      setEditExplanation(selectedQ.explanation ?? "");
      setEditPoints(selectedQ.points);
      setEditTag("");
      setEditQuestionDifficulty("intermediate");
    }
  }, [selectedQ]);

  async function saveMetadata() {
    if (!token) return;
    setSaving(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (makeClient(token) as any).PATCH(`/v1/quizzes/${quizId}`, {
        body: {
          title: editTitle,
          course: editCourse,
          duration: editDuration,
          difficulty: editDifficulty,
          attempts: editAttempts,
        },
      });
      load();
    } catch {
      // noop
    } finally {
      setSaving(false);
    }
  }

  async function saveQuestion() {
    if (!token || !selectedId) return;
    setSaving(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (makeClient(token) as any).PATCH(`/v1/questions/${selectedId}`, {
        body: {
          prompt: editPrompt,
          explanation: editExplanation || undefined,
          points: editPoints,
        },
      });
      load();
    } catch {
      // noop
    } finally {
      setSaving(false);
    }
  }

  async function addQuestion() {
    if (!token) return;
    setSaving(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (makeClient(token) as any).POST(`/v1/quizzes/${quizId}/questions`, {
        body: { kind: "mc", prompt: "" },
      });
      load();
    } catch {
      // noop
    } finally {
      setSaving(false);
    }
  }

  async function publish() {
    if (!token) return;
    setPublishing(true);
    setPublishError(null);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (makeClient(token) as any).PATCH(`/v1/quizzes/${quizId}`, {
        body: { status: "active" },
      });
      load();
    } catch (err) {
      const msg =
        typeof err === "object" && err && "message" in err
          ? String((err as { message: string }).message)
          : "Failed to publish";
      setPublishError(msg);
    } finally {
      setPublishing(false);
    }
  }

  const outlineComplete =
    editTitle.trim().length > 0 && (quiz?.questions.length ?? 0) > 0;
  const questionsNeedingReview =
    quiz?.questions.filter((q) => !q.points || !q.prompt).length ?? 0;
  const totalPoints =
    quiz?.questions.reduce((sum, q) => sum + (q.points || 0), 0) ?? 0;
  const minutesAgo = getMinutesAgo(quiz?.updated_at ?? "");

  if (loading) {
    return (
      <Box sx={{ p: "48px 36px", color: "text.secondary", fontSize: 13 }}>
        Loading…
      </Box>
    );
  }

  if (!quiz) {
    return (
      <Box sx={{ p: "48px 36px", color: "text.secondary", fontSize: 14 }}>
        Quiz not found.
      </Box>
    );
  }

  return (
    <Box sx={{ p: "28px 36px 56px" }}>
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
        <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
          <Button variant="outlined" size="small">
            Import
          </Button>
          <Button variant="outlined" size="small">
            Preview
          </Button>
          <Button variant="outlined" size="small" onClick={saveMetadata}>
            Save draft
          </Button>
          <Button
            variant="contained"
            size="small"
            onClick={publish}
            disabled={quiz.status === "active" || publishing}
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
            gridTemplateColumns: "2fr 1fr 1fr 1fr 1fr",
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
                <span>{questionsNeedingReview} questions need review</span>
              </>
            )}
          </Box>
          <Typography variant="body2" color="text.secondary">
            ≈ {quiz.questions.length} questions · {totalPoints} pts
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
          gridTemplateColumns: "320px 1fr 280px",
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
              onClick={addQuestion}
              sx={{ minWidth: 0 }}
            >
              Add
            </Button>
          </Box>

          <Box sx={{ flex: 1, overflow: "auto" }}>
            {quiz.questions.length === 0 ? (
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
              quiz.questions.map((q, idx) => {
                const sel = selectedId === q.id;
                return (
                  <Box
                    key={q.id}
                    component="button"
                    onClick={() => setSelectedId(q.id)}
                    sx={{
                      width: "100%",
                      px: 2,
                      py: 1.5,
                      bgcolor: sel ? "primary.50" : "transparent",
                      border: "none",
                      borderLeft: `2px solid`,
                      borderLeftColor: sel ? "primary.main" : "transparent",
                      borderBottom: 1,
                      borderColor: "divider",
                      textAlign: "left",
                      cursor: "pointer",
                      display: "grid",
                      gridTemplateColumns: "28px 1fr 40px",
                      gap: 1,
                      alignItems: "start",
                    }}
                  >
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ fontFamily: "monospace", pt: 0.25 }}
                    >
                      Q{idx + 1}
                    </Typography>
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
                    {quiz.questions.findIndex((q) => q.id === selectedId) + 1}
                  </Typography>
                  <Typography variant="subtitle1" sx={{ fontWeight: 500 }}>
                    {kindLabel(selectedQ.kind)}
                  </Typography>
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
                    onBlur={saveQuestion}
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
                  <McOptionsEditor payload={selectedQ.payload} />
                )}

                <Box
                  sx={{
                    mt: 2.25,
                    pt: 2.25,
                    borderTop: 1,
                    borderColor: "divider",
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr 1fr",
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
                        setEditPoints(parseInt(e.target.value) || 0)
                      }
                      onBlur={saveQuestion}
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
                      onChange={(e) => setEditTag(e.target.value)}
                      onBlur={saveQuestion}
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
                      onBlur={saveQuestion}
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
                    onBlur={saveQuestion}
                    rows={2}
                    style={{
                      ...inlineInput,
                      width: "100%",
                      padding: 12,
                      resize: "vertical",
                    }}
                  />
                </Box>
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

function McOptionsEditor({ payload }: { payload: unknown }) {
  const p = payload as {
    options?: { text: string; correct_index?: number }[];
    correct_index?: number;
  } | null;
  if (!p?.options) return null;
  return (
    <Box sx={{ mb: 2.25 }}>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={monoLabel as React.CSSProperties}
      >
        Options · mark the correct answer
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
              p: "10px 12px",
              bgcolor: "action.hover",
              border: 1,
              borderColor: i === p.correct_index ? "primary.main" : "divider",
              borderRadius: 0.5,
            }}
          >
            <Box
              component="button"
              sx={{
                width: 22,
                height: 22,
                borderRadius: "50%",
                bgcolor: i === p.correct_index ? "primary.main" : "transparent",
                border: 1,
                borderColor:
                  i === p.correct_index ? "primary.main" : "action.disabled",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {i === p.correct_index && (
                <CheckOutlinedIcon sx={{ fontSize: 12, color: "#fff" }} />
              )}
            </Box>
            <Typography variant="body2" color="text.secondary">
              {o.text}
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
