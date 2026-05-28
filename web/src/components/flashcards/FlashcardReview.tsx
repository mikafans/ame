"use client";

import { useEffect, useState } from "react";
import Box from "@mui/material/Box";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Chip from "@mui/material/Chip";
import Stack from "@mui/material/Stack";
import { deriveBack, hasModelAnswer, type FlashQuestion } from "@/lib/flashcards";

const KIND_LABEL: Record<string, string> = {
  mc: "Multiple choice",
  tf: "True / false",
  short: "Short answer",
  essay: "Essay",
  code: "Code",
};

interface Props {
  question: FlashQuestion;
  index: number;
  total: number;
  onRate: (knew: boolean) => void;
}

export function FlashcardReview({ question, index, total, onRate }: Props) {
  const [revealed, setRevealed] = useState(false);
  const back = deriveBack(question);

  // Reset flip when the card changes.
  useEffect(() => {
    setRevealed(false);
  }, [question.id]);

  // Space flips; 1/ArrowLeft = missed, 2/ArrowRight = got it (only once revealed).
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === " ") {
        e.preventDefault();
        setRevealed(true);
      } else if (revealed && (e.key === "2" || e.key === "ArrowRight")) {
        onRate(true);
      } else if (revealed && (e.key === "1" || e.key === "ArrowLeft")) {
        onRate(false);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [revealed, onRate]);

  const snippet =
    question.code_snippet && typeof question.code_snippet === "object"
      ? (question.code_snippet as { code?: string }).code
      : undefined;

  return (
    <Box sx={{ maxWidth: 680 }}>
      <Stack
        direction="row"
        sx={{ alignItems: "center", justifyContent: "space-between", mb: 1.5 }}
      >
        <Chip label={KIND_LABEL[question.kind] ?? question.kind} size="small" />
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ fontFamily: "monospace" }}
        >
          {index + 1} / {total}
        </Typography>
      </Stack>

      <Card variant="outlined" sx={{ minHeight: 240 }}>
        <CardContent sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ fontWeight: 600, mb: snippet ? 2 : 0 }}>
            {question.prompt}
          </Typography>

          {snippet && (
            <Box
              component="pre"
              sx={{
                m: 0,
                p: 2,
                bgcolor: "action.hover",
                borderRadius: 1,
                fontFamily: "monospace",
                fontSize: 13,
                overflowX: "auto",
              }}
            >
              {snippet}
            </Box>
          )}

          {revealed && (
            <Box sx={{ mt: 3, pt: 3, borderTop: 1, borderColor: "divider" }}>
              {back.answer !== null ? (
                <>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ textTransform: "uppercase", letterSpacing: 1 }}
                  >
                    Answer
                  </Typography>
                  <Typography variant="body1" sx={{ fontWeight: 600, mb: back.explanation ? 2 : 0 }}>
                    {back.answer}
                  </Typography>
                </>
              ) : null}

              {back.explanation ? (
                <>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ textTransform: "uppercase", letterSpacing: 1 }}
                  >
                    Explanation
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {back.explanation}
                  </Typography>
                </>
              ) : null}

              {!hasModelAnswer(back) && (
                <Typography variant="body2" color="text.secondary">
                  No model answer provided.
                </Typography>
              )}
            </Box>
          )}
        </CardContent>
      </Card>

      <Box sx={{ mt: 2.5 }}>
        {!revealed ? (
          <Button variant="contained" size="large" onClick={() => setRevealed(true)}>
            Show answer (Space)
          </Button>
        ) : (
          <Stack direction="row" spacing={1.5}>
            <Button
              variant="outlined"
              color="error"
              size="large"
              onClick={() => onRate(false)}
            >
              Missed it (1)
            </Button>
            <Button
              variant="contained"
              color="success"
              size="large"
              onClick={() => onRate(true)}
            >
              Got it (2)
            </Button>
          </Stack>
        )}
      </Box>
    </Box>
  );
}
