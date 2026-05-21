"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
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

type TabId = "all" | "active" | "scheduled" | "draft";
type ComposeStep = "form" | "sections";

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
  const [composeStep, setComposeStep] = useState<ComposeStep>("form");
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
    { id: "active", label: "Active" },
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

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
          marginBottom: 20,
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
              marginBottom: 6,
            }}
          >
            Composed assessments · multi-quiz · weighted
          </div>
          <h1
            style={{
              margin: 0,
              fontSize: 26,
              fontWeight: 600,
              color: "var(--text)",
            }}
          >
            Exams
          </h1>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          {isInstructor && (
            <button
              style={{
                padding: "8px 16px",
                background: "var(--surface-2)",
                border: "1px solid var(--border)",
                borderRadius: 4,
                color: "var(--text)",
                fontWeight: 500,
                fontSize: 13,
                cursor: "pointer",
              }}
            >
              Filter
            </button>
          )}
          {isInstructor && (
            <button
              onClick={() => setShowCompose(true)}
              style={{
                padding: "8px 16px",
                background: "var(--accent)",
                border: "none",
                borderRadius: 4,
                color: "#000",
                fontWeight: 600,
                fontSize: 13,
                cursor: "pointer",
              }}
            >
              + Compose exam
            </button>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div
        style={{
          display: "flex",
          gap: 4,
          borderBottom: "1px solid var(--border)",
          marginBottom: 22,
        }}
      >
        {tabs.map((t) => {
          const active = tab === t.id;
          const count =
            t.id === "all"
              ? exams.length
              : exams.filter((e) => e.status === t.id).length;
          return (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              style={{
                background: "transparent",
                border: "none",
                cursor: "pointer",
                padding: "10px 14px",
                fontSize: 13,
                fontWeight: active ? 600 : 500,
                color: active ? "var(--text)" : "var(--muted)",
                borderBottom: `2px solid ${active ? "var(--accent)" : "transparent"}`,
                marginBottom: -1,
              }}
            >
              {t.label}
              <span
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 11,
                  color: "var(--muted)",
                  marginLeft: 4,
                }}
              >
                ({count})
              </span>
            </button>
          );
        })}
      </div>

      {loading ? (
        <div
          style={{
            color: "var(--muted)",
            fontFamily: "var(--mono)",
            fontSize: 13,
          }}
        >
          Loading…
        </div>
      ) : (
        <div
          style={{ display: "grid", gridTemplateColumns: "340px 1fr", gap: 18 }}
        >
          {/* List */}
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {filtered.length === 0 ? (
              <div
                style={{
                  color: "var(--muted)",
                  fontSize: 13,
                  padding: "24px 0",
                }}
              >
                No exams.
              </div>
            ) : (
              filtered.map((e) => (
                <ExamListItem
                  key={e.id}
                  exam={e}
                  selected={selected === e.id}
                  onClick={() => setSelected(e.id)}
                />
              ))
            )}
          </div>

          {/* Detail */}
          {exam ? (
            <ExamDetail
              exam={exam}
              isInstructor={isInstructor}
              starting={starting}
              onStart={startExam}
              onShare={() => setShareExam(exam)}
            />
          ) : (
            <div
              style={{
                color: "var(--muted)",
                fontSize: 13,
                padding: "48px 0",
                textAlign: "center",
              }}
            >
              Select an exam to view details.
            </div>
          )}
        </div>
      )}

      {/* Compose modal */}
      {showCompose && (
        <ComposeModal
          step={composeStep}
          setStep={setComposeStep}
          name={composeName}
          setName={setComposeName}
          desc={composeDesc}
          setDesc={setComposeDesc}
          duration={composeDuration}
          setDuration={setComposeDuration}
          sections={sections}
          setSections={setSections}
          error={composeError}
          composing={composing}
          onCompose={handleCompose}
          onClose={() => {
            setShowCompose(false);
            setComposeError(null);
            setComposeStep("form");
          }}
        />
      )}

      {shareExam && (
        <ShareModal
          payload={{
            kind: "exam",
            id: shareExam.id,
            title: shareExam.name,
          }}
          onClose={() => setShareExam(null)}
        />
      )}
    </div>
  );
}

function ExamListItem({
  exam,
  selected,
  onClick,
}: {
  exam: Exam;
  selected: boolean;
  onClick: () => void;
}) {
  const statusColor =
    exam.status === "active"
      ? "var(--accent)"
      : exam.status === "scheduled"
        ? "var(--blue, #4f8ef7)"
        : "var(--muted)";

  return (
    <button
      onClick={onClick}
      style={{
        width: "100%",
        textAlign: "left",
        cursor: "pointer",
        background: "var(--surface)",
        border: `1px solid ${selected ? "var(--accent-line, var(--accent))" : "var(--border)"}`,
        borderLeft: `3px solid ${selected ? "var(--accent)" : "transparent"}`,
        borderRadius: 6,
        padding: "14px 16px",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 6,
        }}
      >
        <span
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.2,
            textTransform: "uppercase",
            color: "var(--muted)",
          }}
        >
          {exam.course || "—"}
        </span>
        <span
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            color: statusColor,
            textTransform: "uppercase",
            letterSpacing: 0.8,
          }}
        >
          {exam.status}
        </span>
      </div>
      <div
        style={{
          fontFamily: "var(--serif, serif)",
          fontSize: 16,
          fontWeight: 500,
          lineHeight: 1.3,
          letterSpacing: -0.1,
          color: "var(--text)",
          marginBottom: 8,
        }}
      >
        {exam.name}
      </div>
      <div
        style={{
          display: "flex",
          gap: 14,
          alignItems: "center",
          fontFamily: "var(--mono)",
          fontSize: 11,
          color: "var(--muted)",
          letterSpacing: 0.4,
        }}
      >
        <span>◆ {exam.durationMin ?? "—"}m</span>
        <span>▪ {(exam.sections ?? []).length} sec</span>
        <span
          style={{
            marginLeft: "auto",
            color: exam.method === "agent" ? "var(--accent)" : "var(--text-2)",
          }}
        >
          {exam.method === "agent" ? "◇ agent" : "◇ manual"}
        </span>
      </div>
    </button>
  );
}

function ExamDetail({
  exam,
  isInstructor,
  starting,
  onStart,
  onShare,
}: {
  exam: Exam;
  isInstructor: boolean;
  starting: boolean;
  onStart: () => void;
  onShare: () => void;
}) {
  const sections = exam.sections ?? [];
  const totalWeight = sections.reduce((s, x) => s + x.weight, 0);

  return (
    <div
      style={{
        background: "var(--surface)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "24px 28px",
          borderBottom: "1px solid var(--border)",
        }}
      >
        <div
          style={{
            display: "flex",
            gap: 8,
            marginBottom: 12,
            flexWrap: "wrap",
          }}
        >
          <span
            style={{
              fontFamily: "var(--mono)",
              fontSize: 9,
              letterSpacing: 1.1,
              textTransform: "uppercase",
              color: "var(--muted)",
              background: "var(--surface-2)",
              padding: "4px 8px",
              borderRadius: 3,
              border: "1px solid var(--border)",
            }}
          >
            {exam.status}
          </span>
          {exam.course && (
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 9,
                letterSpacing: 1.1,
                textTransform: "uppercase",
                color: "var(--muted)",
                background: "var(--surface-2)",
                padding: "4px 8px",
                borderRadius: 3,
                border: "1px solid var(--border)",
              }}
            >
              {exam.course}
            </span>
          )}
          {exam.tags?.map((tag) => (
            <span
              key={tag}
              style={{
                fontFamily: "var(--mono)",
                fontSize: 9,
                letterSpacing: 1.1,
                textTransform: "uppercase",
                color: "var(--muted)",
                background: "var(--surface-2)",
                padding: "4px 8px",
                borderRadius: 3,
                border: "1px solid var(--border)",
              }}
            >
              {tag}
            </span>
          ))}
        </div>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "flex-start",
            gap: 24,
          }}
        >
          <div style={{ flex: 1, minWidth: 0 }}>
            <h2
              style={{
                margin: 0,
                fontFamily: "var(--serif, serif)",
                fontSize: 30,
                fontWeight: 500,
                letterSpacing: -0.4,
                color: "var(--text)",
                marginBottom: 6,
              }}
            >
              {exam.name}
            </h2>
            {exam.description && (
              <p
                style={{
                  color: "var(--text-2)",
                  fontSize: 13.5,
                  lineHeight: 1.6,
                  margin: 0,
                  maxWidth: 600,
                }}
              >
                {exam.description}
              </p>
            )}
          </div>
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: 8,
              alignItems: "flex-end",
              flexShrink: 0,
            }}
          >
            {exam.status === "active" && (
              <button
                onClick={onStart}
                disabled={starting}
                style={{
                  padding: "9px 20px",
                  background: "var(--accent)",
                  border: "none",
                  borderRadius: 4,
                  color: "#000",
                  fontWeight: 700,
                  fontSize: 13,
                  cursor: starting ? "not-allowed" : "pointer",
                  opacity: starting ? 0.7 : 1,
                }}
              >
                {starting ? "Starting…" : "Start exam →"}
              </button>
            )}
            <button
              onClick={onShare}
              style={{
                padding: "6px 12px",
                background: "var(--surface-2)",
                border: "1px solid var(--border)",
                borderRadius: 4,
                color: "var(--text-2)",
                fontSize: 12,
                cursor: "pointer",
                fontFamily: "var(--mono)",
              }}
            >
              ↑ Share
            </button>
          </div>
        </div>
      </div>

      {/* Stats strip */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(4, 1fr)",
          borderBottom: "1px solid var(--border)",
        }}
      >
        {[
          { l: "Duration", v: exam.durationMin ? `${exam.durationMin}m` : "—" },
          { l: "Sections", v: (exam.sections ?? []).length },
          { l: "Total pts", v: exam.totalPoints },
          {
            l: "Pass mark",
            v: exam.passingPoints ? `${exam.passingPoints} pts` : "—",
          },
        ].map((s, i) => (
          <div
            key={s.l}
            style={{
              padding: "14px 22px",
              borderRight: i < 3 ? "1px solid var(--border)" : "none",
            }}
          >
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10,
                letterSpacing: 1.2,
                textTransform: "uppercase",
                color: "var(--muted)",
                marginBottom: 4,
              }}
            >
              {s.l}
            </div>
            <div
              style={{
                fontSize: 20,
                fontWeight: 600,
                color: "var(--text)",
              }}
            >
              {String(s.v)}
            </div>
          </div>
        ))}
      </div>

      {/* Learning objectives */}
      {exam.objectives?.length > 0 && (
        <div
          style={{
            padding: "20px 28px",
            borderBottom: "1px solid var(--border)",
          }}
        >
          <LearningObjectives items={exam.objectives} />
        </div>
      )}

      {/* Composition */}
      <div
        style={{
          padding: "20px 28px",
          borderBottom: "1px solid var(--border)",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "baseline",
            marginBottom: 14,
          }}
        >
          <div
            style={{
              fontFamily: "var(--serif, serif)",
              fontSize: 17,
              fontWeight: 500,
              color: "var(--text)",
            }}
          >
            Composition
          </div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 11,
              color: "var(--muted)",
            }}
          >
            {sections.length} sections · {totalWeight} pts total
          </div>
        </div>

        {/* Weight bar */}
        <div
          style={{
            display: "flex",
            height: 8,
            borderRadius: 4,
            overflow: "hidden",
            border: "1px solid var(--border)",
            marginBottom: 14,
          }}
        >
          {sections.map((s, i) => {
            const colors = ["var(--accent)", "#4f8ef7", "#f59e0b", "#ef4444"];
            return (
              <div
                key={s.id}
                style={{
                  flex: s.weight,
                  background: colors[i % colors.length],
                  borderRight:
                    i < sections.length - 1
                      ? "1px solid var(--bg, #000)"
                      : "none",
                }}
              />
            );
          })}
        </div>

        {/* Section rows */}
        <div
          style={{
            border: "1px solid var(--border)",
            borderRadius: 6,
            overflow: "hidden",
          }}
        >
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "32px 1fr 80px 80px",
              padding: "8px 14px",
              background: "var(--surface-2)",
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.1,
              textTransform: "uppercase",
              color: "var(--muted)",
              borderBottom: "1px solid var(--border)",
            }}
          >
            <span>§</span>
            <span>Section</span>
            <span style={{ textAlign: "right" }}>Items</span>
            <span style={{ textAlign: "right" }}>Weight</span>
          </div>
          {sections.map((s, i) => {
            const colors = ["var(--accent)", "#4f8ef7", "#f59e0b", "#ef4444"];
            return (
              <div
                key={s.id}
                style={{
                  display: "grid",
                  gridTemplateColumns: "32px 1fr 80px 80px",
                  padding: "12px 14px",
                  borderBottom:
                    i < sections.length - 1
                      ? "1px solid var(--border)"
                      : "none",
                  alignItems: "center",
                }}
              >
                <span
                  style={{
                    fontSize: 16,
                    fontWeight: 600,
                    color: colors[i % colors.length],
                  }}
                >
                  {i + 1}
                </span>
                <div
                  style={{
                    fontSize: 13,
                    fontWeight: 500,
                    color: "var(--text)",
                  }}
                >
                  {s.title}
                </div>
                <span
                  style={{
                    textAlign: "right",
                    fontFamily: "var(--mono)",
                    fontSize: 13,
                    color: "var(--text)",
                  }}
                >
                  {s.items}
                </span>
                <span
                  style={{
                    textAlign: "right",
                    fontSize: 14,
                    fontWeight: 600,
                    color: "var(--text)",
                  }}
                >
                  {s.weight} pts
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Composition trace (instructor only) */}
      {isInstructor && (
        <div style={{ padding: "20px 28px" }}>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 10,
            }}
          >
            Composition trace
          </div>
          {exam.method === "agent" && exam.compositionTrace ? (
            <pre
              style={{
                background: "var(--surface-2)",
                border: "1px solid var(--border)",
                borderRadius: 4,
                padding: 12,
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--text-2)",
                lineHeight: 1.6,
                overflowX: "auto",
                margin: 0,
              }}
            >
              {JSON.stringify(exam.compositionTrace, null, 2)}
            </pre>
          ) : (
            <div
              style={{
                fontSize: 12.5,
                color: "var(--text-2)",
                lineHeight: 1.6,
              }}
            >
              Manually composed
              {exam.composedBy ? ` by ${exam.composedBy}` : ""}.
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function ComposeModal({
  step,
  setStep,
  name,
  setName,
  desc,
  setDesc,
  duration,
  setDuration,
  sections,
  setSections,
  error,
  composing,
  onCompose,
  onClose,
}: {
  step: ComposeStep;
  setStep: (s: ComposeStep) => void;
  name: string;
  setName: (v: string) => void;
  desc: string;
  setDesc: (v: string) => void;
  duration: number;
  setDuration: (v: number) => void;
  sections: SectionDraft[];
  setSections: (v: SectionDraft[]) => void;
  error: string | null;
  composing: boolean;
  onCompose: () => void;
  onClose: () => void;
}) {
  return (
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
          width: 560,
          maxHeight: "85vh",
          overflowY: "auto",
          padding: 28,
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 24,
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
              {step === "form" ? "Step 1 of 2" : "Step 2 of 2"}
            </div>
            <h2
              style={{
                margin: 0,
                fontSize: 20,
                fontWeight: 600,
                color: "var(--text)",
              }}
            >
              {step === "form" ? "Compose exam" : "Add sections"}
            </h2>
          </div>
          <button
            onClick={onClose}
            style={{
              background: "transparent",
              border: "none",
              color: "var(--muted)",
              fontSize: 18,
              cursor: "pointer",
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>

        {step === "form" ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <label style={{ display: "block" }}>
              <div style={labelStyle}>Exam name *</div>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                style={inputStyle}
                placeholder="e.g. Midterm Examination"
              />
            </label>
            <label style={{ display: "block" }}>
              <div style={labelStyle}>Description</div>
              <textarea
                value={desc}
                onChange={(e) => setDesc(e.target.value)}
                rows={2}
                style={{ ...inputStyle, resize: "vertical" }}
                placeholder="Optional description"
              />
            </label>
            <label style={{ display: "block" }}>
              <div style={labelStyle}>Duration (minutes)</div>
              <input
                type="number"
                min={5}
                value={duration}
                onChange={(e) => setDuration(parseInt(e.target.value) || 60)}
                style={{ ...inputStyle, width: 120 }}
              />
            </label>
            <div
              style={{
                display: "flex",
                gap: 10,
                justifyContent: "flex-end",
                marginTop: 8,
              }}
            >
              <button onClick={onClose} style={btnGhost}>
                Cancel
              </button>
              <button
                onClick={() => setStep("sections")}
                disabled={!name.trim()}
                style={{
                  ...btnPrimary,
                  opacity: name.trim() ? 1 : 0.5,
                  cursor: name.trim() ? "pointer" : "not-allowed",
                }}
              >
                Next: sections →
              </button>
            </div>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
            {sections.map((s, i) => (
              <div
                key={i}
                style={{
                  padding: 16,
                  background: "var(--surface-2)",
                  border: "1px solid var(--border)",
                  borderRadius: 6,
                }}
              >
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    marginBottom: 12,
                  }}
                >
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 10,
                      letterSpacing: 1.2,
                      textTransform: "uppercase",
                      color: "var(--muted)",
                    }}
                  >
                    Section {i + 1}
                  </div>
                  {sections.length > 1 && (
                    <button
                      onClick={() =>
                        setSections(sections.filter((_, j) => j !== i))
                      }
                      style={{
                        background: "transparent",
                        border: "none",
                        color: "var(--muted)",
                        cursor: "pointer",
                        fontSize: 12,
                      }}
                    >
                      Remove
                    </button>
                  )}
                </div>
                <div
                  style={{ display: "flex", flexDirection: "column", gap: 10 }}
                >
                  <label style={{ display: "block" }}>
                    <div style={labelStyle}>Title</div>
                    <input
                      value={s.title}
                      onChange={(e) => {
                        const next = [...sections];
                        next[i] = { ...next[i], title: e.target.value };
                        setSections(next);
                      }}
                      style={inputStyle}
                      placeholder={`Section ${i + 1}`}
                    />
                  </label>
                  <label style={{ display: "block" }}>
                    <div style={labelStyle}>Weight (points)</div>
                    <input
                      type="number"
                      min={1}
                      value={s.weight}
                      onChange={(e) => {
                        const next = [...sections];
                        next[i] = {
                          ...next[i],
                          weight: parseInt(e.target.value) || 1,
                        };
                        setSections(next);
                      }}
                      style={{ ...inputStyle, width: 100 }}
                    />
                  </label>
                  <label style={{ display: "block" }}>
                    <div style={labelStyle}>Question IDs (comma-separated)</div>
                    <input
                      value={s.questionIds}
                      onChange={(e) => {
                        const next = [...sections];
                        next[i] = { ...next[i], questionIds: e.target.value };
                        setSections(next);
                      }}
                      style={inputStyle}
                      placeholder="uuid, uuid, ..."
                    />
                  </label>
                </div>
              </div>
            ))}

            <button
              onClick={() =>
                setSections([
                  ...sections,
                  { title: "", weight: 1, questionIds: "" },
                ])
              }
              style={btnGhost}
            >
              + Add section
            </button>

            {error && (
              <div
                style={{
                  padding: "10px 14px",
                  background: "rgba(239,68,68,0.1)",
                  border: "1px solid #ef4444",
                  borderRadius: 4,
                  color: "#ef4444",
                  fontSize: 13,
                }}
              >
                {error}
              </div>
            )}

            <div
              style={{
                display: "flex",
                gap: 10,
                justifyContent: "flex-end",
                marginTop: 8,
              }}
            >
              <button onClick={() => setStep("form")} style={btnGhost}>
                ← Back
              </button>
              <button
                onClick={onCompose}
                disabled={composing}
                style={{ ...btnPrimary, opacity: composing ? 0.7 : 1 }}
              >
                {composing ? "Composing…" : "Compose exam"}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

const labelStyle: React.CSSProperties = {
  fontFamily: "var(--mono)",
  fontSize: 10,
  letterSpacing: 1.3,
  textTransform: "uppercase",
  color: "var(--muted)",
  marginBottom: 6,
  display: "block",
};

const inputStyle: React.CSSProperties = {
  width: "100%",
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

const btnGhost: React.CSSProperties = {
  padding: "8px 14px",
  background: "var(--surface-2)",
  border: "1px solid var(--border)",
  borderRadius: 4,
  color: "var(--text-2)",
  fontSize: 13,
  cursor: "pointer",
};
