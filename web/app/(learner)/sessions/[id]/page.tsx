"use client";

import { useState, useEffect, useCallback, useRef, use } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import LinearProgress from "@mui/material/LinearProgress";
import CircularProgress from "@mui/material/CircularProgress";
import Paper from "@mui/material/Paper";
import ArrowBackOutlinedIcon from "@mui/icons-material/ArrowBackOutlined";
import ArrowForwardOutlinedIcon from "@mui/icons-material/ArrowForwardOutlined";
import ExitToAppOutlinedIcon from "@mui/icons-material/ExitToAppOutlined";
import { McqRenderer } from "@/components/question/McqRenderer";
import { ShortRenderer } from "@/components/question/ShortRenderer";
import { EssayRenderer } from "@/components/question/EssayRenderer";
import { ClozeRenderer } from "@/components/question/ClozeRenderer";
import { TfRenderer } from "@/components/question/TfRenderer";
import { CodeRenderer } from "@/components/question/CodeRenderer";

interface SessionQuestion {
  questionId: string;
  kind: string;
  prompt: string;
  points: number;
  codeSnippet?: { language: string; body: string };
  options?: Array<{ text: string }>;
  optionOrder?: number[];
}

interface SessionData {
  session: {
    id: string;
    status: string;
    deadline_at?: string;
    duration?: number;
    allowed_materials?: string[];
    course_title?: string;
    quiz_title?: string;
  };
  questions: SessionQuestion[];
}

type Answer = string | number | boolean | null;

const QUESTION_TYPES: Record<string, string> = {
  mc: "Multiple choice",
  tf: "True / false",
  short: "Short answer",
  essay: "Essay",
  code: "Code",
  mcq: "Multiple choice",
  free_text: "Free text",
  cloze: "Fill in the blank",
};

const DEFAULT_ALLOWED_MATERIALS = [
  "One sheet of notes (any)",
  "Class textbook (printed)",
  "Standard calculator",
];

export default function ActiveQuizPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { token } = useAuth();
  const router = useRouter();
  const [session, setSession] = useState<SessionData | null>(null);
  const [idx, setIdx] = useState(0);
  const [answers, setAnswers] = useState<Record<string, Answer>>({});
  const [flagged, setFlagged] = useState<Record<string, boolean>>({});
  const [timeLeft, setTimeLeft] = useState<number | null>(null);
  const [finishing, setFinishing] = useState(false);
  const autosaveScheduled = useRef(false);

  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET("/v1/sessions/{id}" as never, { params: { path: { id } } } as never)
      .then(({ data }: { data?: SessionData }) => {
        if (data) {
          setSession(data);
          if (data.session.deadline_at) {
            const remaining = Math.max(
              0,
              Math.floor(
                (new Date(data.session.deadline_at).getTime() - Date.now()) /
                  1000,
              ),
            );
            setTimeLeft(remaining);
          } else if (data.questions.length > 0) {
            setTimeLeft(Math.max(20, data.questions.length * 2) * 60);
          }
        }
      })
      .catch(console.error);
  }, [token, id]);

  const handleFinish = useCallback(async () => {
    if (finishing || !token) return;
    setFinishing(true);
    try {
      const client = makeClient(token);
      for (const [qid, val] of Object.entries(answers)) {
        const q = session?.questions.find((x) => x.questionId === qid);
        if (!q) continue;
        let response: Record<string, unknown>;
        if (q.kind === "mc") {
          const displayPos = q.optionOrder
            ? q.optionOrder.indexOf(val as number)
            : (val as number);
          response = {
            selected_position: displayPos >= 0 ? displayPos : (val as number),
          };
        } else if (q.kind === "tf") {
          response = { answer: val as boolean };
        } else if (q.kind === "essay") {
          const body = String(val ?? "");
          response = {
            body,
            word_count: body.trim().split(/\s+/).filter(Boolean).length,
          };
        } else {
          response = { answer: String(val ?? "") };
        }
        await (
          client as never as {
            POST: (p: string, o: unknown) => Promise<unknown>;
          }
        ).POST("/v1/sessions/{id}/answer", {
          params: { path: { id } },
          body: { questionId: qid, response },
        });
      }
      await (
        client as never as { POST: (p: string, o: unknown) => Promise<unknown> }
      ).POST("/v1/sessions/{id}/finish", { params: { path: { id } } });
      try {
        localStorage.setItem("ame.lastSessionId", id);
      } catch {
        /* ignore */
      }
      router.push(`/sessions/${id}/results`);
    } catch (err) {
      console.error(err);
      setFinishing(false);
    }
  }, [token, id, router, finishing, answers, session]);

  useEffect(() => {
    if (timeLeft === null || timeLeft <= 0) return;
    const t = setInterval(() => {
      setTimeLeft((prev) => {
        if (prev === null || prev <= 1) {
          clearInterval(t);
          handleFinish();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [timeLeft !== null && timeLeft > 0]);

  useEffect(() => {
    if (
      !token ||
      Object.keys(answers).length === 0 ||
      autosaveScheduled.current
    )
      return;
    autosaveScheduled.current = true;
    const timer = setTimeout(() => {
      const answersList = Object.entries(answers).map(([qid, val]) => ({
        questionId: qid,
        answer: val,
      }));
      makeClient(token)
        .PATCH(
          "/v1/sessions/{id}/answers" as never,
          {
            params: { path: { id } },
            body: { answers: answersList } as never,
          } as never,
        )
        .catch(console.error)
        .finally(() => {
          autosaveScheduled.current = false;
        });
    }, 8000);
    return () => {
      clearTimeout(timer);
      autosaveScheduled.current = false;
    };
  }, [answers, token, id]);

  const handleSaveExit = useCallback(async () => {
    if (!token) return;
    try {
      await makeClient(token).PATCH(
        "/v1/sessions/{id}" as never,
        {
          params: { path: { id } },
          body: { status: "abandoned" },
        } as never,
      );
      router.push("/sessions");
    } catch (err) {
      console.error(err);
    }
  }, [token, id, router]);

  if (!session) {
    return (
      <Box
        sx={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: "100vh",
        }}
      >
        <CircularProgress />
      </Box>
    );
  }

  const questions = session.questions;
  const cur = questions[idx];
  const answered = Object.keys(answers).length;
  const curAnswer = cur ? (answers[cur.questionId] ?? null) : null;

  const mm = timeLeft !== null ? Math.floor(timeLeft / 60) : null;
  const ss = timeLeft !== null ? String(timeLeft % 60).padStart(2, "0") : null;

  const allowedMaterials =
    session.session.allowed_materials &&
    session.session.allowed_materials.length > 0
      ? session.session.allowed_materials
      : DEFAULT_ALLOWED_MATERIALS;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      {/* Sticky header */}
      <Paper
        elevation={0}
        sx={{
          borderBottom: 1,
          borderColor: "divider",
          px: 3,
          py: 1.5,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          position: "sticky",
          top: 0,
          zIndex: 100,
          bgcolor: "background.default",
        }}
      >
        <Typography variant="subtitle2" sx={{ fontWeight: 500 }}>
          {session.session.quiz_title || "Quiz"}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {answered}/{questions.length} answered
          {mm !== null && ss !== null && ` · ${mm}:${ss}`}
        </Typography>
        <Button
          size="small"
          variant="outlined"
          startIcon={<ExitToAppOutlinedIcon />}
          onClick={handleSaveExit}
        >
          Save &amp; exit
        </Button>
      </Paper>

      {/* Progress bar */}
      <LinearProgress
        variant="determinate"
        value={questions.length ? (answered / questions.length) * 100 : 0}
      />

      {/* Question */}
      <Box
        sx={{
          flex: 1,
          overflowY: "auto",
          p: 4,
          maxWidth: 800,
          mx: "auto",
          width: "100%",
        }}
      >
        {cur && (
          <>
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ display: "block", mb: 1 }}
            >
              Question {idx + 1} of {questions.length} ·{" "}
              {QUESTION_TYPES[cur.kind] ?? cur.kind} · {cur.points}{" "}
              {cur.points === 1 ? "pt" : "pts"}
            </Typography>
            <Typography variant="h6" sx={{ mb: 3 }}>
              {cur.prompt}
            </Typography>

            <Box data-testid="question-input">
              <QuestionInput
                question={cur}
                value={curAnswer}
                onChange={(v) =>
                  setAnswers((prev) => ({ ...prev, [cur.questionId]: v }))
                }
                disabled={false}
              />
            </Box>
          </>
        )}
      </Box>

      {/* Footer nav */}
      <Paper
        elevation={0}
        sx={{
          borderTop: 1,
          borderColor: "divider",
          px: 3,
          py: 1.5,
          display: "flex",
          justifyContent: "space-between",
        }}
      >
        <Button
          startIcon={<ArrowBackOutlinedIcon />}
          disabled={idx === 0}
          onClick={() => setIdx((i) => Math.max(0, i - 1))}
        >
          Previous
        </Button>
        {idx < questions.length - 1 ? (
          <Button
            endIcon={<ArrowForwardOutlinedIcon />}
            variant="contained"
            onClick={() => setIdx((i) => Math.min(questions.length - 1, i + 1))}
          >
            Next
          </Button>
        ) : (
          <Button
            variant="contained"
            color="success"
            disabled={finishing}
            onClick={handleFinish}
          >
            {finishing ? "Submitting…" : "Submit"}
          </Button>
        )}
      </Paper>
    </Box>
  );
}

function QuestionInput({
  question,
  value,
  onChange,
  disabled,
}: {
  question: SessionQuestion;
  value: Answer;
  onChange: (v: Answer) => void;
  disabled: boolean;
}) {
  if ((question.kind === "mc" || question.kind === "mcq") && question.options) {
    const order = question.optionOrder ?? question.options.map((_, i) => i);
    const opts = order.map((pos) => ({
      text: question.options![pos].text,
      index: pos,
    }));
    return (
      <McqRenderer
        options={opts}
        value={typeof value === "number" ? value : null}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  if (question.kind === "tf") {
    return (
      <TfRenderer
        value={typeof value === "boolean" ? value : null}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  if (question.kind === "cloze") {
    return (
      <ClozeRenderer
        prompt={question.prompt}
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  if (question.kind === "code") {
    return (
      <CodeRenderer
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  const isEssay =
    question.kind === "free_text" &&
    String(value ?? "")
      .trim()
      .split(/\s+/).length > 30;
  if (isEssay || question.kind === "essay") {
    return (
      <EssayRenderer
        value={String(value ?? "")}
        onChange={onChange}
        disabled={disabled}
      />
    );
  }
  return (
    <ShortRenderer
      value={String(value ?? "")}
      onChange={onChange}
      disabled={disabled}
    />
  );
}
