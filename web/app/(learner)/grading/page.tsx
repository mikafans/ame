"use client";

import { useState, useEffect } from "react";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import TextField from "@mui/material/TextField";
import CircularProgress from "@mui/material/CircularProgress";

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
      { headers: { Authorization: `Bearer ${token}` } },
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
      <Box sx={{ p: 4, color: "text.secondary", fontSize: 14 }}>
        Access restricted to instructors and admins.
      </Box>
    );
  }

  if (loading) {
    return (
      <Box
        sx={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <CircularProgress size={24} />
      </Box>
    );
  }

  return (
    <Box sx={{ p: "28px 36px 56px" }}>
      <Box sx={{ mb: 3.5 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            letterSpacing: 1.3,
            textTransform: "uppercase",
            display: "block",
            mb: 1,
          }}
        >
          Instructor · Manual grading
        </Typography>
        <Typography variant="h4" sx={{ fontWeight: 500 }}>
          Essay Grading
        </Typography>
      </Box>

      {attempts.length === 0 ? (
        <Card variant="outlined" sx={{ p: 4.5, textAlign: "center" }}>
          <Typography variant="body2" color="text.secondary">
            No essays pending manual review.
          </Typography>
        </Card>
      ) : (
        <Stack spacing={2.5}>
          <Typography variant="caption" color="text.secondary">
            {attempts.length} essay{attempts.length !== 1 ? "s" : ""} awaiting
            review
          </Typography>
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
              <Card key={attempt.attempt_id} variant="outlined">
                <Box
                  sx={{
                    px: "22px",
                    py: 2,
                    borderBottom: 1,
                    borderColor: "divider",
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                  }}
                >
                  <Box>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{
                        display: "block",
                        letterSpacing: 1.2,
                        textTransform: "uppercase",
                        mb: 0.5,
                      }}
                    >
                      Essay · {attempt.response_word_count} words
                    </Typography>
                    <Typography
                      variant="subtitle1"
                      sx={{ fontWeight: 500, lineHeight: 1.4 }}
                    >
                      {attempt.question_prompt}
                    </Typography>
                  </Box>
                  <Box sx={{ textAlign: "right", flexShrink: 0, ml: 3 }}>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ display: "block" }}
                    >
                      {attempt.user_display_name}
                    </Typography>
                    <Typography
                      variant="caption"
                      color="text.disabled"
                      sx={{ display: "block" }}
                    >
                      {attempt.user_email}
                    </Typography>
                    <Typography
                      variant="caption"
                      color="text.disabled"
                      sx={{ display: "block" }}
                    >
                      {new Date(attempt.created_at).toLocaleDateString()}
                    </Typography>
                  </Box>
                </Box>

                <Box
                  sx={{
                    px: "22px",
                    py: 2,
                    borderBottom: 1,
                    borderColor: "divider",
                    bgcolor: "action.hover",
                  }}
                >
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{
                      display: "block",
                      letterSpacing: 1.2,
                      textTransform: "uppercase",
                      mb: 1,
                    }}
                  >
                    Learner&apos;s answer
                  </Typography>
                  <Typography
                    variant="body2"
                    sx={{ lineHeight: 1.7, whiteSpace: "pre-wrap" }}
                  >
                    {attempt.response_body || "(empty)"}
                  </Typography>
                </Box>

                <Box
                  sx={{
                    px: "22px",
                    py: 2,
                    display: "grid",
                    gridTemplateColumns: "120px 1fr auto",
                    gap: 2,
                    alignItems: "flex-end",
                  }}
                >
                  <TextField
                    label="Score (0–100)"
                    type="number"
                    size="small"
                    slotProps={{ htmlInput: { min: 0, max: 100, step: 1 } }}
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
                  />
                  <TextField
                    label="Notes (optional)"
                    size="small"
                    multiline
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
                  />
                  <Button
                    variant="contained"
                    disabled={!scoreValid || g.submitting}
                    onClick={() => handleGrade(attempt.attempt_id)}
                  >
                    {g.submitting ? "Saving…" : "Grade"}
                  </Button>
                </Box>
              </Card>
            );
          })}
        </Stack>
      )}
    </Box>
  );
}
