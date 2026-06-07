"use client";

import { useState, useEffect, useCallback, useRef, use } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import LinearProgress from "@mui/material/LinearProgress";
import CircularProgress from "@mui/material/CircularProgress";
import Paper from "@mui/material/Paper";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
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
  language?: string;
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
    assessment_title?: string;
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
  const { user } = useAuth();
  const router = useRouter();
  const [session, setSession] = useState<SessionData | null>(null);
  const [idx, setIdx] = useState(0);
  const [answers, setAnswers] = useState<Record<string, Answer>>({});
  const [flagged, setFlagged] = useState<Record<string, boolean>>({});
  const [timeLeft, setTimeLeft] = useState<number | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [quitDialogOpen, setQuitDialogOpen] = useState(false);
  const [submitDialogOpen, setSubmitDialogOpen] = useState(false);

  const questions = session?.questions ?? [];
  const answered = Object.keys(answers).length;

  useEffect(() => {
    api
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
  }, [id]);

  const handleFinish = useCallback(async () => {
    if (finishing) return;
    setFinishing(true);
    try {
      const client = api;
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
        } else if (q.kind === "code") {
          // Must match the grader's Code { source, language } shape, and the
          // language must equal the question's grading language or the answer
          // is rejected (and not saved).
          response = {
            source: String(val ?? ""),
            language: q.language ?? q.codeSnippet?.language ?? "python",
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
  }, [id, router, finishing, answers, session]);

  // Keep the latest handleFinish reachable from the once-created interval below
  // without rebuilding the interval — the interval closes over a stale closure
  // otherwise and would auto-submit an empty answer set on timeout.
  const finishRef = useRef(handleFinish);
  useEffect(() => {
    finishRef.current = handleFinish;
  }, [handleFinish]);

  useEffect(() => {
    if (timeLeft === null || timeLeft <= 0) return;
    const t = setInterval(() => {
      setTimeLeft((prev) => {
        if (prev === null || prev <= 1) {
          clearInterval(t);
          finishRef.current();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [timeLeft !== null && timeLeft > 0]);

  const handleSaveExit = useCallback(async () => {
    try {
      await api.PATCH(
        "/v1/sessions/{id}" as never,
        {
          params: { path: { id } },
          body: { status: "abandoned" },
        } as never,
      );
      router.push("/explore");
    } catch (err) {
      console.error(err);
    }
  }, [id, router]);

  const handleQuitClick = useCallback(() => {
    setQuitDialogOpen(true);
  }, []);

  const handleSubmitClick = useCallback(() => {
    setSubmitDialogOpen(true);
  }, []);

  // Enter advances to the next question. On the last question it opens the
  // submit confirmation dialog rather than finishing outright — submitting ends
  // the attempt and is destructive, so it must be confirmed exactly like the
  // Submit button (never a silent one-keystroke submit). In multiline fields
  // (essay/code) plain Enter inserts a newline, so advancing there requires
  // Ctrl/Cmd+Enter.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key !== "Enter" || e.shiftKey || !session) return;
      const target = e.target as HTMLElement | null;
      const isTextarea = target?.tagName === "TEXTAREA";
      if (isTextarea && !(e.ctrlKey || e.metaKey)) return;
      e.preventDefault();
      const last = session.questions.length - 1;
      if (idx < last) setIdx((i) => Math.min(last, i + 1));
      else setSubmitDialogOpen(true);
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [idx, session]);

  if (!session) {
    return (
      <Box
        sx={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: "100dvh",
        }}
      >
        <CircularProgress />
      </Box>
    );
  }

  const cur = questions[idx];
  const curAnswer = cur ? (answers[cur.questionId] ?? null) : null;

  const mm = timeLeft !== null ? Math.floor(timeLeft / 60) : null;
  const ss = timeLeft !== null ? String(timeLeft % 60).padStart(2, "0") : null;

  const allowedMaterials =
    session.session.allowed_materials &&
    session.session.allowed_materials.length > 0
      ? session.session.allowed_materials
      : DEFAULT_ALLOWED_MATERIALS;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", height: "100dvh" }}>
      {/* Sticky header */}
      <Paper
        elevation={0}
        sx={{
          borderBottom: 1,
          borderColor: "divider",
          px: { xs: 1.5, sm: 3 },
          py: 1.5,
          display: "flex",
          flexWrap: "wrap",
          gap: 1,
          alignItems: "center",
          justifyContent: "space-between",
          position: "sticky",
          top: 0,
          zIndex: 100,
          bgcolor: "background.default",
        }}
      >
        <Typography
          variant="subtitle2"
          sx={{ fontWeight: 500, minWidth: 0, flexShrink: 1 }}
          noWrap
        >
          {session.session.assessment_title || "Quiz"}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {answered}/{questions.length} answered
          {mm !== null && ss !== null && ` · ${mm}:${ss}`}
        </Typography>
        <Stack direction="row" spacing={1.5} sx={{ flexShrink: 0 }}>
          <Button
            size="small"
            variant="contained"
            color="success"
            disabled={finishing}
            onClick={handleSubmitClick}
          >
            {finishing ? "Submitting…" : "Submit"}
          </Button>
          <Button
            size="small"
            variant="outlined"
            color="error"
            startIcon={<ExitToAppOutlinedIcon />}
            onClick={handleQuitClick}
          >
            Quit
          </Button>
        </Stack>
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
          p: { xs: 2, sm: 4 },
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
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ alignSelf: "center", display: { xs: "none", sm: "block" } }}
        >
          Press Enter to continue
        </Typography>
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
            onClick={handleSubmitClick}
          >
            {finishing ? "Submitting…" : "Submit"}
          </Button>
        )}
      </Paper>

      {/* Quit Confirmation Dialog */}
      <Dialog
        open={quitDialogOpen}
        onClose={() => setQuitDialogOpen(false)}
        maxWidth="xs"
        fullWidth
      >
        <DialogTitle>Quit Assessment?</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Are you sure you want to quit? Your progress will not be saved and
            you cannot resume this assessment later.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setQuitDialogOpen(false)}>Cancel</Button>
          <Button
            color="error"
            variant="contained"
            onClick={() => {
              setQuitDialogOpen(false);
              handleSaveExit();
            }}
          >
            Quit
          </Button>
        </DialogActions>
      </Dialog>

      {/* Submit Confirmation Dialog */}
      <Dialog
        open={submitDialogOpen}
        onClose={() => setSubmitDialogOpen(false)}
        maxWidth="xs"
        fullWidth
      >
        <DialogTitle>Submit Assessment?</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {questions.length - answered > 0
              ? `You have ${questions.length - answered} unanswered question${
                  questions.length - answered !== 1 ? "s" : ""
                }. Are you sure you want to submit and finish this assessment?`
              : "Are you sure you want to submit and finish this assessment?"}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSubmitDialogOpen(false)}>Cancel</Button>
          <Button
            color="success"
            variant="contained"
            onClick={() => {
              setSubmitDialogOpen(false);
              handleFinish();
            }}
          >
            Submit
          </Button>
        </DialogActions>
      </Dialog>
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
        language={question.language ?? question.codeSnippet?.language}
        starter={question.codeSnippet?.body}
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
