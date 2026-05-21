"use client";

import { useState, useEffect, use } from "react";
import { Button, Card, Icon, KV } from "@/components/ui";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";

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
  const days = Math.floor(hours / 24);
  return `${days}d`;
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

  // Metadata state
  const [editTitle, setEditTitle] = useState("");
  const [editCourse, setEditCourse] = useState("");
  const [editDuration, setEditDuration] = useState(30);
  const [editDifficulty, setEditDifficulty] = useState("intermediate");
  const [editAttempts, setEditAttempts] = useState(2);

  // Question editor state
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
        body: {
          kind: "mc",
          prompt: "",
        },
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
      <div
        style={{
          padding: "48px 36px",
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Loading…
      </div>
    );
  }

  if (!quiz) {
    return (
      <div
        style={{ padding: "48px 36px", color: "var(--muted)", fontSize: 14 }}
      >
        Quiz not found.
      </div>
    );
  }

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
          marginBottom: 18,
        }}
      >
        <div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 4,
            }}
          >
            Editing draft · {editCourse} · autosaved
          </div>
          <h1
            style={{
              margin: 0,
              fontSize: 26,
              fontWeight: 400,
              fontFamily: "var(--serif)",
              color: "var(--text)",
            }}
          >
            Author studio
          </h1>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <Button variant="ghost" size="md">
            Import
          </Button>
          <Button variant="ghost" size="md">
            Preview
          </Button>
          <Button variant="outline" size="md" onClick={saveMetadata}>
            Save draft
          </Button>
          <Button
            variant="primary"
            size="md"
            onClick={publish}
            disabled={quiz.status === "active" || publishing}
            icon={<Icon name="arrow" size={14} />}
          >
            {publishing ? "Publishing…" : "Publish"}
          </Button>
        </div>
      </div>

      {publishError && (
        <div
          style={{
            padding: "10px 14px",
            background: "rgba(239,68,68,0.1)",
            border: "1px solid #ef4444",
            borderRadius: 4,
            color: "#ef4444",
            fontSize: 13,
            marginBottom: 14,
          }}
        >
          {publishError}
        </div>
      )}

      {/* Metadata row */}
      <Card style={{ marginBottom: 18, padding: 0, borderRadius: 6 }}>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "2fr 1fr 1fr 1fr 1fr",
            borderBottom: "1px solid var(--border)",
          }}
        >
          <label
            style={{
              padding: "14px 22px",
              borderRight: "1px solid var(--border)",
            }}
          >
            <div style={monoLabel}>Title</div>
            <input
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              onBlur={saveMetadata}
              style={inlineInput}
            />
          </label>
          <label
            style={{
              padding: "14px 22px",
              borderRight: "1px solid var(--border)",
            }}
          >
            <div style={monoLabel}>Course</div>
            <input
              value={editCourse}
              onChange={(e) => setEditCourse(e.target.value)}
              onBlur={saveMetadata}
              style={inlineInput}
            />
          </label>
          <label
            style={{
              padding: "14px 22px",
              borderRight: "1px solid var(--border)",
            }}
          >
            <div style={monoLabel}>Duration</div>
            <input
              type="number"
              value={editDuration}
              onChange={(e) => setEditDuration(parseInt(e.target.value) || 30)}
              onBlur={saveMetadata}
              style={inlineInput}
            />
          </label>
          <label
            style={{
              padding: "14px 22px",
              borderRight: "1px solid var(--border)",
            }}
          >
            <div style={monoLabel}>Difficulty</div>
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
          </label>
          <label style={{ padding: "14px 22px" }}>
            <div style={monoLabel}>Attempts</div>
            <input
              type="number"
              value={editAttempts}
              onChange={(e) => setEditAttempts(parseInt(e.target.value) || 1)}
              onBlur={saveMetadata}
              style={inlineInput}
            />
          </label>
        </div>

        {/* Validation status bar */}
        <div
          style={{
            padding: "14px 22px",
            display: "flex",
            gap: 28,
            alignItems: "center",
            fontSize: 12.5,
            color: "var(--text-2)",
          }}
        >
          <div
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              color: outlineComplete ? "var(--accent)" : "var(--muted)",
            }}
          >
            <span>{outlineComplete ? "✓" : "✗"}</span>
            <span>Outline complete</span>
          </div>

          <div
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              color:
                questionsNeedingReview === 0 ? "var(--accent)" : "var(--muted)",
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
          </div>

          <div style={{ color: "var(--text-2)" }}>
            <span>
              ≈ {quiz.questions.length} questions · {totalPoints} pts
            </span>
          </div>

          <span
            style={{
              marginLeft: "auto",
              color: "var(--muted)",
              fontFamily: "var(--mono)",
              fontSize: 11,
              letterSpacing: 0.5,
            }}
          >
            Last edit · {minutesAgo} ago · by you
          </span>
        </div>
      </Card>

      {/* Three-pane layout */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "320px 1fr 280px",
          gap: 18,
          minHeight: 600,
        }}
      >
        {/* LEFT - Questions list */}
        <Card
          style={{
            padding: 0,
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <div
            style={{
              padding: "14px 16px",
              borderBottom: "1px solid var(--border)",
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10,
                letterSpacing: 1.3,
                textTransform: "uppercase",
                color: "var(--muted)",
              }}
            >
              Questions
            </div>
            <Button
              variant="ghost"
              size="sm"
              icon={<Icon name="plus" size={12} />}
              onClick={addQuestion}
            >
              Add
            </Button>
          </div>

          <div style={{ flex: 1, overflow: "auto" }}>
            {quiz.questions.length === 0 ? (
              <div
                style={{
                  padding: "24px 16px",
                  color: "var(--muted)",
                  fontSize: 13,
                  textAlign: "center",
                }}
              >
                No questions yet.
              </div>
            ) : (
              quiz.questions.map((q, idx) => {
                const sel = selectedId === q.id;
                return (
                  <button
                    key={q.id}
                    onClick={() => setSelectedId(q.id)}
                    style={{
                      width: "100%",
                      padding: "12px 16px",
                      background: sel ? "var(--accent-dim)" : "transparent",
                      border: "none",
                      borderLeft: `2px solid ${
                        sel ? "var(--accent)" : "transparent"
                      }`,
                      borderBottom: "1px solid var(--border)",
                      textAlign: "left",
                      cursor: "pointer",
                      display: "grid",
                      gridTemplateColumns: "28px 1fr 40px",
                      gap: 8,
                      alignItems: "start",
                    }}
                  >
                    <span
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 11,
                        color: "var(--muted)",
                        marginTop: 2,
                      }}
                    >
                      Q{idx + 1}
                    </span>
                    <div style={{ minWidth: 0 }}>
                      <div
                        style={{
                          fontSize: 13,
                          color: sel ? "var(--text)" : "var(--text-2)",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          fontWeight: sel ? 500 : 400,
                          marginBottom: 4,
                        }}
                      >
                        {q.prompt.slice(0, 40)}
                        {q.prompt.length > 40 ? "…" : ""}
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--mono)",
                          fontSize: 10,
                          color: "var(--muted)",
                          textTransform: "uppercase",
                          letterSpacing: 0.5,
                        }}
                      >
                        {kindLabel(q.kind)}
                      </div>
                    </div>
                    <span
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 11,
                        color: "var(--text-2)",
                        textAlign: "right",
                        marginTop: 2,
                      }}
                    >
                      {q.points}pt
                    </span>
                  </button>
                );
              })
            )}
          </div>

          {/* Generate footer */}
          <div
            style={{
              padding: 14,
              background: "var(--surface-2)",
              borderTop: "1px solid var(--border)",
            }}
          >
            <div
              style={{
                fontSize: 12,
                color: "var(--text-2)",
                lineHeight: 1.5,
                marginBottom: 8,
              }}
            >
              Generate from source — paste notes or a reading.
            </div>
            <Button
              variant="outline"
              size="sm"
              icon={<Icon name="sparkle" size={12} />}
            >
              Generate questions
            </Button>
          </div>
        </Card>

        {/* MIDDLE - Question editor */}
        <Card
          style={{
            padding: 0,
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
          }}
        >
          {selectedQ ? (
            <>
              <div
                style={{
                  padding: "16px 22px",
                  borderBottom: "1px solid var(--border)",
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <div>
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 10,
                      letterSpacing: 1.3,
                      color: "var(--muted)",
                      textTransform: "uppercase",
                      marginBottom: 4,
                    }}
                  >
                    Editing Q
                    {quiz.questions.findIndex((q) => q.id === selectedId) + 1}
                  </div>
                  <div
                    style={{
                      fontSize: 18,
                      fontWeight: 500,
                      fontFamily: "var(--serif)",
                      color: "var(--text)",
                    }}
                  >
                    {kindLabel(selectedQ.kind)}
                  </div>
                </div>
              </div>

              <div style={{ flex: 1, overflow: "auto", padding: 22 }}>
                <label style={{ display: "block", marginBottom: 18 }}>
                  <div style={monoLabel}>Question prompt</div>
                  <textarea
                    value={editPrompt}
                    onChange={(e) => setEditPrompt(e.target.value)}
                    onBlur={saveQuestion}
                    rows={3}
                    style={{
                      ...inlineInput,
                      width: "100%",
                      padding: 12,
                      fontFamily: "var(--serif)",
                      fontSize: 16,
                      resize: "vertical",
                    }}
                  />
                </label>

                {selectedQ.kind === "mc" && (
                  <McOptionsEditor payload={selectedQ.payload} />
                )}

                <div
                  style={{
                    marginTop: 18,
                    paddingTop: 18,
                    borderTop: "1px solid var(--border)",
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr 1fr",
                    gap: 18,
                  }}
                >
                  <label style={{ display: "block" }}>
                    <div style={monoLabel}>Points</div>
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
                  </label>
                  <label style={{ display: "block" }}>
                    <div style={monoLabel}>Tag</div>
                    <input
                      value={editTag}
                      onChange={(e) => setEditTag(e.target.value)}
                      onBlur={saveQuestion}
                      style={inlineInput}
                    />
                  </label>
                  <label style={{ display: "block" }}>
                    <div style={monoLabel}>Difficulty</div>
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
                  </label>
                </div>

                <label style={{ display: "block", marginTop: 22 }}>
                  <div style={monoLabel}>Explanation shown after answering</div>
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
                </label>
              </div>
            </>
          ) : (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                padding: "48px 22px",
                textAlign: "center",
                color: "var(--muted)",
                fontSize: 13,
              }}
            >
              Select a question to edit.
            </div>
          )}
        </Card>

        {/* RIGHT - Rail */}
        <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
          <Card>
            <div style={sectionLabel}>Distribution</div>
            <KV label="Most correct" value="#1" />
            <KV label="Hardest item" value="#2" />
          </Card>

          <Card>
            <div style={sectionLabel}>Rubric</div>
            <div
              style={{
                fontSize: 12,
                color: "var(--text-2)",
                fontFamily: "var(--mono)",
                lineHeight: 1.5,
                padding: "10px 12px",
                background: "var(--surface-2)",
                borderRadius: 4,
                border: "1px solid var(--border)",
              }}
            >
              criteria:
              <br />
              &nbsp;&nbsp;clarity: 0–2
              <br />
              &nbsp;&nbsp;evidence: 0–2
              <br />
              &nbsp;&nbsp;mechanism: 0–1
              <br />
              &nbsp;&nbsp;link: 0–1
            </div>
          </Card>

          <Card>
            <div style={sectionLabel}>Recent activity</div>
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 10,
                fontSize: 12,
              }}
            >
              <ActivityEntry name="You" action="edited Q1" time="4m" />
              <ActivityEntry name="You" action="added 2 questions" time="2h" />
              <ActivityEntry
                name="System"
                action="auto-saved draft"
                time="5m"
              />
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

function McOptionsEditor({ payload }: { payload: unknown }) {
  const p = payload as {
    options?: { text: string; correct_index?: number }[];
    correct_index?: number;
  } | null;
  if (!p?.options) return null;
  return (
    <div style={{ marginBottom: 18 }}>
      <div style={monoLabel}>Options · mark the correct answer</div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {p.options.map((o, i) => (
          <div
            key={i}
            style={{
              display: "grid",
              gridTemplateColumns: "30px 1fr",
              gap: 10,
              alignItems: "center",
              padding: "10px 12px",
              background: "var(--surface-2)",
              border: `1px solid ${
                i === p.correct_index ? "var(--accent)" : "var(--border)"
              }`,
              borderRadius: 4,
            }}
          >
            <button
              style={{
                width: 22,
                height: 22,
                borderRadius: "50%",
                background:
                  i === p.correct_index ? "var(--accent)" : "transparent",
                border: `1px solid ${
                  i === p.correct_index
                    ? "var(--accent)"
                    : "var(--border-strong)"
                }`,
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {i === p.correct_index ? (
                <Icon name="check" size={12} color="#0b1410" />
              ) : null}
            </button>
            <span style={{ fontSize: 13, color: "var(--text-2)" }}>
              {o.text}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function ActivityEntry({
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
    <div style={{ display: "flex", gap: 8 }}>
      <span
        style={{
          width: 4,
          alignSelf: "stretch",
          background: isYou ? "var(--accent)" : "var(--border-strong)",
          borderRadius: 2,
        }}
      />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ color: "var(--text)" }}>
          <span style={{ color: isYou ? "var(--accent)" : "var(--text-2)" }}>
            {name}
          </span>{" "}
          {action}
        </div>
        <div
          style={{
            color: "var(--muted)",
            fontFamily: "var(--mono)",
            fontSize: 10.5,
            marginTop: 2,
            letterSpacing: 0.5,
          }}
        >
          {time} ago
        </div>
      </div>
    </div>
  );
}

const monoLabel: React.CSSProperties = {
  fontFamily: "var(--mono)",
  fontSize: 10,
  letterSpacing: 1.3,
  textTransform: "uppercase",
  color: "var(--muted)",
  marginBottom: 6,
  display: "block",
};

const sectionLabel: React.CSSProperties = {
  fontFamily: "var(--mono)",
  fontSize: 10,
  letterSpacing: 1.3,
  textTransform: "uppercase",
  color: "var(--muted)",
  marginBottom: 12,
};

const inlineInput: React.CSSProperties = {
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 6,
  padding: "9px 12px",
  color: "var(--text)",
  fontFamily: "var(--sans, sans-serif)",
  fontSize: 13.5,
  outline: "none",
  boxSizing: "border-box",
  width: "100%",
};
