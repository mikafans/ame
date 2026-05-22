"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Tag } from "@/components/ui/Tag";
import { Icon } from "@/components/ui/Icon";
import { KV } from "@/components/ui/KV";
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
  const tabs: Array<{ id: TabId; label: string }> = [
    { id: "all", label: "All quizzes" },
    { id: "assigned", label: "Assigned to me" },
    { id: "completed", label: "Completed" },
  ];
  if (isInstructor) {
    tabs.push({ id: "drafts", label: "Drafts" });
  }

  const currentData = allQuizzes[tab];
  const { quizzes } = currentData;
  const upNext = upNextId
    ? (quizzes.find((q) => q.id === upNextId) ?? null)
    : null;
  const remaining = upNext ? quizzes.slice(1) : quizzes;

  const daysUntilDue = (dueDate: string | undefined) => {
    if (!dueDate) return null;
    const due = new Date(dueDate);
    const now = new Date();
    const diff = Math.ceil(
      (due.getTime() - now.getTime()) / (1000 * 60 * 60 * 24),
    );
    return diff > 0 ? diff : null;
  };

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Header + Action */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-end",
          justifyContent: "space-between",
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
              marginBottom: 6,
            }}
          >
            Spring 2026 · Active term
          </div>
          <h1
            style={{
              margin: 0,
              fontSize: 26,
              fontWeight: 600,
              color: "var(--text)",
            }}
          >
            Library
          </h1>
        </div>
        {isInstructor && (
          <Button
            variant="outline"
            icon={<Icon name="plus" size={14} />}
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
      </div>

      {/* Start error */}
      {startError && (
        <div
          style={{
            marginBottom: 16,
            padding: "10px 14px",
            background: "var(--red-subtle, #fff1f0)",
            border: "1px solid var(--red, #f5222d)",
            borderRadius: 6,
            color: "var(--red, #cf1322)",
            fontSize: 13,
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span>{startError}</span>
          <button
            onClick={() => setStartError(null)}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              color: "inherit",
              fontWeight: 600,
              fontSize: 16,
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>
      )}

      {/* Tabs */}
      <div
        style={{
          display: "flex",
          gap: 4,
          borderBottom: "1px solid var(--border)",
          marginBottom: 24,
        }}
      >
        {tabs.map((t) => {
          const active = tab === t.id;
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
              {t.label}{" "}
              <span
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 11,
                  color: "var(--muted)",
                  marginLeft: 4,
                }}
              >
                ({allQuizzes[t.id].total})
              </span>
            </button>
          );
        })}
      </div>

      {/* Loading state */}
      {loading && (
        <div
          style={{
            color: "var(--muted)",
            fontFamily: "var(--mono)",
            fontSize: 13,
          }}
        >
          Loading…
        </div>
      )}

      {/* No quizzes state */}
      {!loading && quizzes.length === 0 && (
        <div
          style={{
            padding: "48px 0",
            textAlign: "center",
            color: "var(--muted)",
            fontSize: 14,
          }}
        >
          No quizzes found.
        </div>
      )}

      {/* Hero card (Up next) */}
      {!loading && upNext && (
        <Card style={{ marginBottom: 24, overflow: "hidden", padding: 0 }}>
          <div style={{ display: "grid", gridTemplateColumns: "1.3fr 1fr" }}>
            <div
              style={{
                padding: "28px 32px",
                borderRight: "1px solid var(--border)",
              }}
            >
              <div
                style={{
                  display: "flex",
                  gap: 8,
                  marginBottom: 14,
                  flexWrap: "wrap",
                }}
              >
                <Tag color="muted">{upNext.course || "Uncategorized"}</Tag>
                {upNext.difficulty && (
                  <Tag color="muted">{upNext.difficulty}</Tag>
                )}
                {daysUntilDue(upNext.due_date) !== null && (
                  <Tag color="amber">
                    DUE IN {daysUntilDue(upNext.due_date)} DAYS
                  </Tag>
                )}
              </div>
              <div
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 10,
                  letterSpacing: 1.2,
                  textTransform: "uppercase",
                  color: "var(--accent)",
                  marginBottom: 10,
                }}
              >
                Up next
              </div>
              <h2
                style={{
                  margin: 0,
                  fontFamily: "var(--serif)",
                  fontSize: 24,
                  fontWeight: 500,
                  letterSpacing: -0.3,
                  marginBottom: 10,
                }}
              >
                {upNext.title}
              </h2>
              {upNext.description && (
                <p
                  style={{
                    color: "var(--text-2)",
                    fontSize: 14,
                    lineHeight: 1.55,
                    margin: 0,
                    marginBottom: 18,
                  }}
                >
                  {upNext.description}
                </p>
              )}
              {upNext.objectives && upNext.objectives.length > 0 && (
                <div style={{ marginBottom: 22 }}>
                  <LearningObjectives items={upNext.objectives} />
                </div>
              )}
              {/* Stat strip */}
              <div
                style={{
                  display: "flex",
                  gap: 20,
                  marginBottom: 18,
                  fontSize: 12,
                  fontFamily: "var(--mono)",
                  color: "var(--muted)",
                  letterSpacing: 0.3,
                }}
              >
                {upNext.questionCount != null && (
                  <span>{upNext.questionCount} questions</span>
                )}
                {upNext.questionCount != null && (
                  <span>~{upNext.questionCount * 2} min</span>
                )}
              </div>
              <div style={{ display: "flex", gap: 12 }}>
                <Button
                  variant="primary"
                  icon={<Icon name="arrow" size={14} />}
                  onClick={() => startQuiz(upNext.id)}
                  disabled={starting === upNext.id}
                >
                  {starting === upNext.id ? "Starting…" : "Start"}
                </Button>
                <Button
                  variant="outline"
                  onClick={() => router.push(`/quizzes/${upNext.id}/preview`)}
                >
                  Preview questions
                </Button>
                <Button variant="ghost" onClick={() => setShareOpen(upNext.id)}>
                  Share
                </Button>
              </div>
            </div>
            <div
              style={{ padding: "28px 32px", background: "var(--surface-2)" }}
            >
              <div
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 10,
                  letterSpacing: 1.3,
                  textTransform: "uppercase",
                  color: "var(--muted)",
                  marginBottom: 14,
                }}
              >
                Cohort context
              </div>
              <KV
                label="Class average"
                value={
                  cohortStats?.cohortAvg != null
                    ? `${Math.round(cohortStats.cohortAvg * 100)}%`
                    : "—"
                }
              />
              <KV
                label="Completion rate"
                value={
                  cohortStats?.completionRate != null
                    ? `${Math.round(cohortStats.completionRate * 100)}%`
                    : "—"
                }
              />
              <KV
                label="Your percentile"
                value={
                  cohortStats?.percentile != null
                    ? `${cohortStats.percentile}th`
                    : "—"
                }
              />
              <KV label="Your last score" value="—" />
            </div>
          </div>
        </Card>
      )}

      {/* Quiz grid */}
      {!loading && remaining.length > 0 && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(330px, 1fr))",
            gap: 16,
          }}
        >
          {remaining.map((q) => (
            <QuizCardGrid
              key={q.id}
              quiz={q}
              starting={starting === q.id}
              onStart={() => startQuiz(q.id)}
              onShare={() => setShareOpen(q.id)}
            />
          ))}
        </div>
      )}

      {/* Share modal */}
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
    </div>
  );
}

function QuizCardGrid({
  quiz,
  starting,
  onStart,
  onShare,
}: {
  quiz: Quiz;
  starting: boolean;
  onStart: () => void;
  onShare: () => void;
}) {
  const accentColor =
    quiz.color === "amber"
      ? "var(--amber)"
      : quiz.color === "blue"
        ? "var(--blue)"
        : "var(--accent)";

  return (
    <Card
      style={{
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
        padding: 0,
      }}
    >
      <div
        style={{
          padding: "16px 18px",
          borderBottom: "1px solid var(--border)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 8,
        }}
      >
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.2,
            color: "var(--muted)",
            textTransform: "uppercase",
          }}
        >
          {quiz.course && quiz.difficulty
            ? `${quiz.course} · ${quiz.difficulty}`
            : quiz.course || quiz.difficulty || "Uncategorized"}
        </div>
        {quiz.status === "draft" && <Tag color="muted">Draft</Tag>}
      </div>
      <div style={{ padding: "18px 18px 0", flex: 1 }}>
        <div style={{ display: "flex", gap: 12, alignItems: "flex-start" }}>
          <div
            style={{
              width: 4,
              alignSelf: "stretch",
              background: accentColor,
              borderRadius: 2,
            }}
          />
          <div style={{ flex: 1, minWidth: 0 }}>
            <h3
              style={{
                margin: 0,
                fontFamily: "var(--serif)",
                fontSize: 16,
                fontWeight: 500,
                lineHeight: 1.25,
                letterSpacing: -0.2,
              }}
            >
              {quiz.title}
            </h3>
            {quiz.description && (
              <p
                style={{
                  color: "var(--muted)",
                  fontSize: 13,
                  lineHeight: 1.5,
                  margin: "8px 0 0",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  display: "-webkit-box",
                  WebkitLineClamp: 2,
                  WebkitBoxOrient: "vertical",
                }}
              >
                {quiz.description}
              </p>
            )}
          </div>
        </div>
      </div>
      <div
        style={{
          padding: "12px 18px",
          borderTop: "1px solid var(--border)",
          background: "var(--surface-2)",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: 12,
        }}
      >
        <div style={{ display: "flex", gap: 6 }}>
          {/* Tags could go here if needed */}
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <button
            onClick={onShare}
            style={{
              background: "transparent",
              border: "none",
              cursor: "pointer",
              color: "var(--muted)",
              padding: 0,
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              fontSize: 11,
              fontFamily: "var(--mono)",
              letterSpacing: 0.5,
            }}
          >
            <Icon name="stack" size={12} /> Share
          </button>
          <button
            onClick={onStart}
            disabled={starting}
            style={{
              background: "transparent",
              border: "none",
              color: "var(--accent)",
              fontWeight: 600,
              fontSize: 12,
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              cursor: starting ? "not-allowed" : "pointer",
              opacity: starting ? 0.6 : 1,
            }}
          >
            {starting ? "Starting…" : "Open"}{" "}
            <Icon name="arrow" size={12} color="var(--accent)" />
          </button>
        </div>
      </div>
    </Card>
  );
}
