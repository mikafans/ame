"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { LearningObjectives } from "@/components/LearningObjectives";
import { ShareModal } from "@/components/ShareModal";

interface Quiz {
  id: string;
  title: string;
  status: string;
  createdAt: string;
}

type TabId = "all" | "active" | "archived";

export default function LibraryPage() {
  const { token } = useAuth();
  const router = useRouter();
  const [tab, setTab] = useState<TabId>("all");
  const [quizzes, setQuizzes] = useState<Quiz[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState<string | null>(null);

  useEffect(() => {
    if (!token) return;
    const status = tab === "all" ? "active" : tab;
    const client = makeClient(token);
    client
      .GET("/v1/quizzes" as never, { params: { query: { status } } } as never)
      .then(({ data }: { data?: { quizzes: Quiz[]; total: number } }) => {
        if (data) {
          setQuizzes(data.quizzes);
          setTotal(data.total);
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [token, tab]);

  async function startQuiz(quizId: string) {
    if (!token) return;
    setStarting(quizId);
    try {
      const client = makeClient(token);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (client as any).POST("/v1/sessions", {
        body: { quizId },
      });
      if (data) {
        router.push(`/sessions/${(data as { sessionId: string }).sessionId}`);
      }
    } catch (err) {
      console.error(err);
    } finally {
      setStarting(null);
    }
  }

  const tabs: { id: TabId; label: string }[] = [
    { id: "all", label: "All quizzes" },
    { id: "active", label: "Active" },
    { id: "archived", label: "Archived" },
  ];

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
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
            Quiz Library
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
        <a
          href="/practice"
          style={{
            padding: "8px 16px",
            background: "var(--accent)",
            color: "#000",
            borderRadius: 4,
            textDecoration: "none",
            fontWeight: 600,
            fontSize: 13,
          }}
        >
          Practice
        </a>
      </div>

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
              {t.label}
              {active && (
                <span
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 11,
                    color: "var(--muted)",
                    marginLeft: 6,
                  }}
                >
                  ({total})
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Content */}
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
      ) : quizzes.length === 0 ? (
        <div
          style={{
            padding: "48px 0",
            textAlign: "center",
            color: "var(--muted)",
            fontSize: 14,
          }}
        >
          No quizzes found.
          <br />
          <a
            href="/practice"
            style={{
              color: "var(--accent)",
              textDecoration: "none",
              fontSize: 13,
            }}
          >
            Start a practice session instead
          </a>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {quizzes.map((q) => (
            <QuizCard
              key={q.id}
              quiz={q}
              starting={starting === q.id}
              onStart={() => startQuiz(q.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function QuizCard({
  quiz,
  starting,
  onStart,
}: {
  quiz: Quiz;
  starting: boolean;
  onStart: () => void;
}) {
  const [showShare, setShowShare] = useState(false);

  return (
    <div
      style={{
        background: "var(--surface)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        padding: "20px 24px",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 16,
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
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
          {quiz.status}
        </div>
        <div
          style={{
            fontWeight: 600,
            fontSize: 15,
            color: "var(--text)",
            marginBottom: 6,
          }}
        >
          {quiz.title}
        </div>
        <LearningObjectives items={[]} compact />
      </div>
      <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
        <button
          onClick={() => setShowShare(true)}
          style={{
            padding: "7px 12px",
            background: "var(--surface-2)",
            border: "1px solid var(--border)",
            borderRadius: 4,
            color: "var(--text-2)",
            cursor: "pointer",
            fontSize: 12,
            fontFamily: "var(--mono)",
          }}
        >
          Share
        </button>
        <button
          onClick={onStart}
          disabled={starting}
          style={{
            padding: "7px 16px",
            background: "var(--accent)",
            border: "none",
            borderRadius: 4,
            color: "#000",
            cursor: starting ? "not-allowed" : "pointer",
            fontWeight: 600,
            fontSize: 13,
            opacity: starting ? 0.7 : 1,
          }}
        >
          {starting ? "Starting…" : "Start"}
        </button>
      </div>
      {showShare && (
        <ShareModal
          payload={{ kind: "quiz", id: quiz.id, title: quiz.title }}
          onClose={() => setShowShare(false)}
        />
      )}
    </div>
  );
}
