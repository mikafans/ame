"use client";

import { useState, useEffect } from "react";
import { useAuth } from "@/hooks/useAuth";
import { Button, Card } from "@/components/ui";

interface PendingAttempt {
  attempt_id: string;
  session_id: string;
  user_id: string;
  user_email: string;
  user_display_name: string;
  question_id: string;
  question_prompt: string;
  response_body: string;
  response_word_count: number;
  created_at: string;
}

interface GradeState {
  score: string;
  notes: string;
  submitting: boolean;
  done: boolean;
}

export default function GradingPage() {
  const { token, user } = useAuth();
  const [attempts, setAttempts] = useState<PendingAttempt[]>([]);
  const [loading, setLoading] = useState(true);
  const [grades, setGrades] = useState<Record<string, GradeState>>({});

  const isInstructor = user?.role === "instructor" || user?.role === "admin";

  useEffect(() => {
    if (!token || !isInstructor) return;
    fetch(
      `${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080"}/v1/attempts/pending`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    )
      .then((r) => r.json())
      .then((data: PendingAttempt[]) => {
        setAttempts(data);
        const initial: Record<string, GradeState> = {};
        for (const a of data) {
          initial[a.attempt_id] = {
            score: "",
            notes: "",
            submitting: false,
            done: false,
          };
        }
        setGrades(initial);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [token, isInstructor]);

  const handleGrade = async (attemptId: string) => {
    const g = grades[attemptId];
    if (!g || !token) return;

    const scoreNum = parseFloat(g.score);
    if (isNaN(scoreNum) || scoreNum < 0 || scoreNum > 100) return;

    setGrades((prev) => ({
      ...prev,
      [attemptId]: { ...prev[attemptId], submitting: true },
    }));

    try {
      const res = await fetch(
        `${process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080"}/v1/attempts/${attemptId}/grade`,
        {
          method: "PATCH",
          headers: {
            Authorization: `Bearer ${token}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            score: scoreNum / 100,
            notes: g.notes || null,
          }),
        },
      );

      if (res.ok) {
        setGrades((prev) => ({
          ...prev,
          [attemptId]: { ...prev[attemptId], submitting: false, done: true },
        }));
        setAttempts((prev) => prev.filter((a) => a.attempt_id !== attemptId));
      } else {
        setGrades((prev) => ({
          ...prev,
          [attemptId]: { ...prev[attemptId], submitting: false },
        }));
      }
    } catch {
      setGrades((prev) => ({
        ...prev,
        [attemptId]: { ...prev[attemptId], submitting: false },
      }));
    }
  };

  if (!isInstructor) {
    return (
      <div
        style={{
          padding: "28px 36px",
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Access restricted to instructors and admins.
      </div>
    );
  }

  if (loading) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Loading pending essays…
      </div>
    );
  }

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      <div style={{ marginBottom: 28 }}>
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.3,
            textTransform: "uppercase",
            color: "var(--muted)",
            marginBottom: 8,
          }}
        >
          Instructor · Manual grading
        </div>
        <h1
          style={{
            fontFamily: "var(--serif)",
            fontSize: 40,
            fontWeight: 500,
            letterSpacing: -0.8,
            lineHeight: 1.1,
            color: "var(--text)",
            margin: 0,
          }}
        >
          Essay Grading
        </h1>
      </div>

      {attempts.length === 0 ? (
        <Card style={{ padding: 36, textAlign: "center" }}>
          <div
            style={{
              color: "var(--muted)",
              fontFamily: "var(--mono)",
              fontSize: 13,
            }}
          >
            No essays pending manual review.
          </div>
        </Card>
      ) : (
        <div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 11,
              color: "var(--muted)",
              marginBottom: 16,
              letterSpacing: 0.5,
            }}
          >
            {attempts.length} essay{attempts.length !== 1 ? "s" : ""} awaiting
            review
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
            {attempts.map((attempt) => {
              const g = grades[attempt.attempt_id] ?? {
                score: "",
                notes: "",
                submitting: false,
                done: false,
              };
              const scoreNum = parseFloat(g.score);
              const scoreValid =
                !isNaN(scoreNum) && scoreNum >= 0 && scoreNum <= 100;

              return (
                <Card key={attempt.attempt_id} style={{ padding: 0 }}>
                  <div
                    style={{
                      padding: "16px 22px",
                      borderBottom: "1px solid var(--border)",
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "flex-start",
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
                        Essay · {attempt.response_word_count} words
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--serif)",
                          fontSize: 16,
                          fontWeight: 500,
                          color: "var(--text)",
                          lineHeight: 1.4,
                        }}
                      >
                        {attempt.question_prompt}
                      </div>
                    </div>
                    <div
                      style={{
                        textAlign: "right",
                        flexShrink: 0,
                        marginLeft: 24,
                      }}
                    >
                      <div
                        style={{
                          fontFamily: "var(--mono)",
                          fontSize: 11,
                          color: "var(--text-2)",
                        }}
                      >
                        {attempt.user_display_name}
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--mono)",
                          fontSize: 10,
                          color: "var(--muted)",
                          marginTop: 2,
                        }}
                      >
                        {attempt.user_email}
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--mono)",
                          fontSize: 10,
                          color: "var(--muted)",
                          marginTop: 2,
                        }}
                      >
                        {new Date(attempt.created_at).toLocaleDateString()}
                      </div>
                    </div>
                  </div>

                  <div
                    style={{
                      padding: "16px 22px",
                      borderBottom: "1px solid var(--border)",
                      background: "var(--surface-2)",
                    }}
                  >
                    <div
                      style={{
                        fontFamily: "var(--mono)",
                        fontSize: 10,
                        letterSpacing: 1.2,
                        textTransform: "uppercase",
                        color: "var(--muted)",
                        marginBottom: 8,
                      }}
                    >
                      Learner's answer
                    </div>
                    <div
                      style={{
                        fontFamily: "var(--serif)",
                        fontSize: 14,
                        lineHeight: 1.7,
                        color: "var(--text)",
                        whiteSpace: "pre-wrap",
                      }}
                    >
                      {attempt.response_body || "(empty)"}
                    </div>
                  </div>

                  <div
                    style={{
                      padding: "16px 22px",
                      display: "grid",
                      gridTemplateColumns: "120px 1fr auto",
                      gap: 16,
                      alignItems: "flex-end",
                    }}
                  >
                    <div>
                      <label
                        style={{
                          display: "block",
                          fontFamily: "var(--mono)",
                          fontSize: 10,
                          letterSpacing: 1.2,
                          textTransform: "uppercase",
                          color: "var(--muted)",
                          marginBottom: 6,
                        }}
                      >
                        Score (0–100)
                      </label>
                      <input
                        type="number"
                        min={0}
                        max={100}
                        step={1}
                        value={g.score}
                        onChange={(e) =>
                          setGrades((prev) => ({
                            ...prev,
                            [attempt.attempt_id]: {
                              ...prev[attempt.attempt_id],
                              score: e.target.value,
                            },
                          }))
                        }
                        placeholder="e.g. 75"
                        style={{
                          width: "100%",
                          padding: "8px 10px",
                          background: "var(--surface)",
                          border: "1px solid var(--border)",
                          borderRadius: 4,
                          color: "var(--text)",
                          fontFamily: "var(--mono)",
                          fontSize: 13,
                          boxSizing: "border-box",
                        }}
                      />
                    </div>

                    <div>
                      <label
                        style={{
                          display: "block",
                          fontFamily: "var(--mono)",
                          fontSize: 10,
                          letterSpacing: 1.2,
                          textTransform: "uppercase",
                          color: "var(--muted)",
                          marginBottom: 6,
                        }}
                      >
                        Notes (optional)
                      </label>
                      <textarea
                        rows={2}
                        value={g.notes}
                        onChange={(e) =>
                          setGrades((prev) => ({
                            ...prev,
                            [attempt.attempt_id]: {
                              ...prev[attempt.attempt_id],
                              notes: e.target.value,
                            },
                          }))
                        }
                        placeholder="Good structure but missing examples…"
                        style={{
                          width: "100%",
                          padding: "8px 10px",
                          background: "var(--surface)",
                          border: "1px solid var(--border)",
                          borderRadius: 4,
                          color: "var(--text)",
                          fontFamily: "var(--serif)",
                          fontSize: 13,
                          lineHeight: 1.5,
                          resize: "vertical",
                          boxSizing: "border-box",
                        }}
                      />
                    </div>

                    <Button
                      variant="primary"
                      disabled={!scoreValid || g.submitting}
                      onClick={() => handleGrade(attempt.attempt_id)}
                    >
                      {g.submitting ? "Saving…" : "Grade"}
                    </Button>
                  </div>
                </Card>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
