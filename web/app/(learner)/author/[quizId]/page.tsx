"use client";

import { useState, useEffect, use } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { LearningObjectives } from "@/components/LearningObjectives";

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
  objectives: string[];
  questions: QuizQuestion[];
}

function kindLabel(kind: string): string {
  const map: Record<string, string> = {
    mc: "Multiple choice",
    tf: "True/false",
    short: "Short answer",
    essay: "Essay",
    code: "Code",
  };
  return map[kind] ?? kind;
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
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [publishing, setPublishing] = useState(false);
  const [publishError, setPublishError] = useState<string | null>(null);
  const [showGenerate, setShowGenerate] = useState(false);
  const [generateSource, setGenerateSource] = useState("");
  const [generating, setGenerating] = useState(false);
  const [generateWarnings, setGenerateWarnings] = useState<string[]>([]);

  // Editable fields for selected question
  const [editPrompt, setEditPrompt] = useState("");
  const [editExplanation, setEditExplanation] = useState("");
  const [editPoints, setEditPoints] = useState(1);

  // Editable quiz metadata
  const [editTitle, setEditTitle] = useState("");
  const [editObjectives, setEditObjectives] = useState<string[]>([]);
  const [objectivesWarning, setObjectivesWarning] = useState<string | null>(
    null,
  );

  function load() {
    if (!token) return;
    setLoading(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET(`/v1/quizzes/${quizId}`)
      .then(({ data }: { data?: Quiz }) => {
        if (data) {
          setQuiz(data);
          setEditTitle(data.title);
          setEditObjectives(data.objectives ?? []);
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
    }
  }, [selectedQ]);

  async function saveQuestion() {
    if (!token || !selectedId) return;
    setSaving(true);
    setSaveMsg(null);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (makeClient(token) as any).PATCH(`/questions/${selectedId}`, {
        body: {
          prompt: editPrompt,
          explanation: editExplanation || undefined,
          points: editPoints,
        },
      });
      setSaveMsg("Saved");
      load();
    } catch {
      setSaveMsg("Error saving");
    } finally {
      setSaving(false);
      setTimeout(() => setSaveMsg(null), 2000);
    }
  }

  async function saveMetadata() {
    if (!token) return;
    setSaving(true);
    setObjectivesWarning(null);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (makeClient(token) as any).PATCH(
        `/v1/quizzes/${quizId}`,
        { body: { title: editTitle, objectives: editObjectives } },
      );
      if (data?.warnings?.length) {
        setObjectivesWarning(data.warnings[0]);
      }
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
      const { error } = await (makeClient(token) as any).PATCH(
        `/v1/quizzes/${quizId}`,
        { body: { status: "active" } },
      );
      if (error) {
        const msg =
          typeof error === "object" && "message" in error
            ? String((error as { message: string }).message)
            : "Failed to publish";
        setPublishError(msg);
      } else {
        load();
      }
    } catch {
      setPublishError("Could not reach API");
    } finally {
      setPublishing(false);
    }
  }

  async function runGenerate() {
    if (!token) return;
    setGenerating(true);
    setGenerateWarnings([]);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (makeClient(token) as any).POST(
        "/v1/quizzes/generate",
        { body: { source: generateSource, questionCount: 5 } },
      );
      if (data?.warnings?.length) {
        setGenerateWarnings(data.warnings);
      }
    } catch {
      setGenerateWarnings(["Could not reach generate endpoint"]);
    } finally {
      setGenerating(false);
    }
  }

  const canPublish =
    quiz &&
    quiz.questions.length > 0 &&
    quiz.questions.every((q) => q.status === "live") &&
    quiz.status !== "active";

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
            Editing {quiz.status} · autosaved
          </div>
          <h1
            style={{
              margin: 0,
              fontSize: 22,
              fontWeight: 600,
              color: "var(--text)",
            }}
          >
            Author studio
          </h1>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          {saveMsg && (
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--muted)",
              }}
            >
              {saveMsg}
            </span>
          )}
          <button
            onClick={saveQuestion}
            disabled={saving || !selectedId}
            style={btnSolid}
          >
            Save draft
          </button>
          <button
            onClick={publish}
            disabled={!canPublish || publishing}
            title={
              !canPublish
                ? "Need ≥1 live question and quiz not already active"
                : "Publish quiz"
            }
            style={{
              ...btnPrimary,
              opacity: canPublish ? 1 : 0.4,
              cursor: canPublish ? "pointer" : "not-allowed",
            }}
          >
            {publishing ? "Publishing…" : "Publish →"}
          </button>
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

      {/* Metadata strip */}
      <div
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          padding: "16px 22px",
          marginBottom: 18,
          display: "flex",
          flexDirection: "column",
          gap: 12,
        }}
      >
        <label style={{ display: "flex", alignItems: "center", gap: 14 }}>
          <span style={monoLabel}>Title</span>
          <input
            value={editTitle}
            onChange={(e) => setEditTitle(e.target.value)}
            onBlur={saveMetadata}
            style={{ ...inlineInput, flex: 1 }}
          />
        </label>

        <div>
          <div style={monoLabel}>Learning objectives</div>
          {editObjectives.map((obj, i) => (
            <div
              key={i}
              style={{
                display: "flex",
                gap: 8,
                marginBottom: 6,
                alignItems: "center",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 11,
                  color: "var(--muted)",
                  width: 20,
                  textAlign: "right",
                  flexShrink: 0,
                }}
              >
                {String(i + 1).padStart(2, "0")}
              </span>
              <input
                value={obj}
                onChange={(e) => {
                  const next = [...editObjectives];
                  next[i] = e.target.value;
                  setEditObjectives(next);
                }}
                onBlur={saveMetadata}
                style={{ ...inlineInput, flex: 1 }}
              />
              <button
                onClick={() => {
                  setEditObjectives(editObjectives.filter((_, j) => j !== i));
                  saveMetadata();
                }}
                style={{
                  background: "transparent",
                  border: "none",
                  color: "var(--muted)",
                  cursor: "pointer",
                  fontSize: 14,
                  lineHeight: 1,
                }}
              >
                ×
              </button>
            </div>
          ))}
          <button
            onClick={() => setEditObjectives([...editObjectives, ""])}
            style={{
              background: "transparent",
              border: "none",
              color: "var(--accent)",
              cursor: "pointer",
              fontSize: 12,
              fontFamily: "var(--mono)",
              padding: "2px 0",
            }}
          >
            + Add objective
          </button>
          {objectivesWarning && (
            <div style={{ color: "#f59e0b", fontSize: 12, marginTop: 4 }}>
              ⚠ {objectivesWarning}
            </div>
          )}
        </div>

        {quiz.objectives.length > 0 && (
          <div style={{ borderTop: "1px solid var(--border)", paddingTop: 12 }}>
            <LearningObjectives items={quiz.objectives} compact />
          </div>
        )}
      </div>

      {/* Three-pane layout */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "300px 1fr 260px",
          gap: 18,
        }}
      >
        {/* Questions list */}
        <div
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            overflow: "hidden",
          }}
        >
          <div
            style={{
              padding: "12px 16px",
              borderBottom: "1px solid var(--border)",
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10,
                letterSpacing: 1.3,
                textTransform: "uppercase",
                color: "var(--muted)",
              }}
            >
              Questions
            </span>
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--muted)",
              }}
            >
              {quiz.questions.length} total
            </span>
          </div>

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
                    background: sel
                      ? "var(--accent-dim, rgba(0,200,100,0.08))"
                      : "transparent",
                    border: "none",
                    borderLeft: `2px solid ${sel ? "var(--accent)" : "transparent"}`,
                    borderBottom: "1px solid var(--border)",
                    textAlign: "left",
                    cursor: "pointer",
                    display: "grid",
                    gridTemplateColumns: "28px 1fr 40px",
                    gap: 8,
                    alignItems: "center",
                  }}
                >
                  <span
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 11,
                      color: "var(--muted)",
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
                      }}
                    >
                      {q.prompt}
                    </div>
                    <div
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 10,
                        color: "var(--muted)",
                        marginTop: 2,
                        textTransform: "uppercase",
                        letterSpacing: 0.5,
                      }}
                    >
                      {kindLabel(q.kind)} ·{" "}
                      {q.status !== "live" ? (
                        <span style={{ color: "#f59e0b" }}>{q.status}</span>
                      ) : (
                        q.status
                      )}
                    </div>
                  </div>
                  <span
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 11,
                      color: "var(--text-2)",
                      textAlign: "right",
                    }}
                  >
                    {q.points}pt
                  </span>
                </button>
              );
            })
          )}

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
            <button onClick={() => setShowGenerate(true)} style={btnSolid}>
              ✦ Generate questions
            </button>
          </div>
        </div>

        {/* Editor */}
        <div
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            overflow: "hidden",
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
                    }}
                  >
                    Editing Q
                    {quiz.questions.findIndex((q) => q.id === selectedId) + 1}
                  </div>
                  <div
                    style={{
                      fontSize: 17,
                      fontWeight: 600,
                      marginTop: 2,
                      color: "var(--text)",
                    }}
                  >
                    {kindLabel(selectedQ.kind)}
                  </div>
                </div>
              </div>

              <div style={{ padding: 22 }}>
                <label style={{ display: "block", marginBottom: 18 }}>
                  <div style={monoLabel}>Question prompt</div>
                  <textarea
                    value={editPrompt}
                    onChange={(e) => setEditPrompt(e.target.value)}
                    rows={3}
                    style={{
                      ...inlineInput,
                      width: "100%",
                      padding: 12,
                      resize: "vertical",
                    }}
                  />
                </label>

                {/* MC options preview (read-only payload) */}
                {selectedQ.kind === "mc" && (
                  <McOptionsEditor payload={selectedQ.payload} />
                )}

                <div
                  style={{
                    marginTop: 18,
                    paddingTop: 18,
                    borderTop: "1px solid var(--border)",
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
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
                      style={{ ...inlineInput, width: 80 }}
                    />
                  </label>
                </div>

                <label style={{ display: "block", marginTop: 18 }}>
                  <div style={monoLabel}>Explanation shown after answering</div>
                  <textarea
                    value={editExplanation}
                    onChange={(e) => setEditExplanation(e.target.value)}
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
                padding: "48px 22px",
                textAlign: "center",
                color: "var(--muted)",
                fontSize: 13,
              }}
            >
              Select a question to edit.
            </div>
          )}
        </div>

        {/* Right rail */}
        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          <div
            style={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: 6,
              padding: 18,
            }}
          >
            <div style={sectionLabel}>Summary</div>
            <KV k="Questions" v={String(quiz.questions.length)} />
            <KV
              k="Ready to publish"
              v={
                canPublish
                  ? "Yes"
                  : quiz.status === "active"
                    ? "Published"
                    : "No"
              }
            />
            <KV k="Status" v={quiz.status} />
          </div>

          <div
            style={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: 6,
              padding: 18,
            }}
          >
            <div style={sectionLabel}>Rubric · auto-grading</div>
            <div
              style={{ fontSize: 12, color: "var(--text-2)", lineHeight: 1.6 }}
            >
              MC and TF are graded instantly. Short answers compare against
              accepted strings. Essays use a 4-criterion rubric.
            </div>
          </div>

          <div
            style={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: 6,
              padding: 18,
            }}
          >
            <div style={sectionLabel}>Publish gating</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <GateRow
                ok={quiz.questions.length > 0}
                label="At least 1 question"
              />
              <GateRow
                ok={quiz.questions.every((q) => q.status === "live")}
                label="All questions live"
              />
              <GateRow
                ok={quiz.status !== "active"}
                label="Not yet published"
              />
            </div>
          </div>
        </div>
      </div>

      {/* Generate dialog */}
      {showGenerate && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.6)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 100,
          }}
        >
          <div
            style={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: 8,
              width: 500,
              padding: 28,
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: 18,
              }}
            >
              <h2
                style={{
                  margin: 0,
                  fontSize: 18,
                  fontWeight: 600,
                  color: "var(--text)",
                }}
              >
                Generate questions
              </h2>
              <button
                onClick={() => setShowGenerate(false)}
                style={{
                  background: "transparent",
                  border: "none",
                  color: "var(--muted)",
                  fontSize: 18,
                  cursor: "pointer",
                }}
              >
                ×
              </button>
            </div>
            <label style={{ display: "block", marginBottom: 14 }}>
              <div style={monoLabel}>Source material</div>
              <textarea
                value={generateSource}
                onChange={(e) => setGenerateSource(e.target.value)}
                rows={6}
                style={{
                  ...inlineInput,
                  width: "100%",
                  padding: 12,
                  resize: "vertical",
                }}
                placeholder="Paste lecture notes, PDF text, or a reading…"
              />
            </label>
            {generateWarnings.length > 0 && (
              <div
                style={{
                  padding: "10px 12px",
                  background: "rgba(245,158,11,0.1)",
                  border: "1px solid #f59e0b",
                  borderRadius: 4,
                  marginBottom: 14,
                }}
              >
                {generateWarnings.map((w) => (
                  <div key={w} style={{ fontSize: 12, color: "#f59e0b" }}>
                    ⚠ {w}
                  </div>
                ))}
              </div>
            )}
            <div
              style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}
            >
              <button onClick={() => setShowGenerate(false)} style={btnGhost}>
                Cancel
              </button>
              <button
                onClick={runGenerate}
                disabled={generating || !generateSource.trim()}
                style={{
                  ...btnPrimary,
                  opacity: generating || !generateSource.trim() ? 0.5 : 1,
                }}
              >
                {generating ? "Generating…" : "Generate"}
              </button>
            </div>
          </div>
        </div>
      )}
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
    <div>
      <div style={monoLabel}>Options</div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {p.options.map((o, i) => (
          <div
            key={i}
            style={{
              display: "flex",
              gap: 10,
              alignItems: "center",
              padding: "8px 12px",
              background: "var(--surface-2)",
              border: `1px solid ${i === p.correct_index ? "var(--accent)" : "var(--border)"}`,
              borderRadius: 4,
            }}
          >
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: i === p.correct_index ? "var(--accent)" : "var(--muted)",
                width: 16,
              }}
            >
              {i === p.correct_index ? "✓" : String.fromCharCode(65 + i)}
            </span>
            <span style={{ fontSize: 13, color: "var(--text-2)" }}>
              {o.text}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function GateRow({ ok, label }: { ok: boolean; label: string }) {
  return (
    <div
      style={{
        display: "flex",
        gap: 8,
        alignItems: "center",
        fontSize: 12,
        color: ok ? "var(--accent)" : "var(--muted)",
      }}
    >
      <span>{ok ? "✓" : "○"}</span>
      <span>{label}</span>
    </div>
  );
}

function KV({ k, v }: { k: string; v: string }) {
  return (
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        fontSize: 12,
        marginBottom: 6,
      }}
    >
      <span style={{ color: "var(--muted)" }}>{k}</span>
      <span style={{ color: "var(--text)", fontWeight: 500 }}>{v}</span>
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
};

const btnPrimary: React.CSSProperties = {
  padding: "8px 18px",
  background: "var(--accent)",
  border: "none",
  borderRadius: 4,
  color: "#000",
  fontWeight: 700,
  fontSize: 13,
  cursor: "pointer",
};

const btnSolid: React.CSSProperties = {
  padding: "7px 14px",
  background: "var(--surface-2)",
  border: "1px solid var(--border)",
  borderRadius: 4,
  color: "var(--text)",
  fontSize: 12,
  cursor: "pointer",
};

const btnGhost: React.CSSProperties = {
  padding: "8px 14px",
  background: "transparent",
  border: "1px solid var(--border)",
  borderRadius: 4,
  color: "var(--text-2)",
  fontSize: 13,
  cursor: "pointer",
};
