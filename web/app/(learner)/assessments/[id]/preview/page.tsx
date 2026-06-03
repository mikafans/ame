"use client";

import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import Chip from "@mui/material/Chip";
import CircularProgress from "@mui/material/CircularProgress";
import { useColorMode } from "@/components/ThemeRegistry";
import { tagColor } from "@/lib/tagColor";

const KIND_LABEL: Record<string, string> = {
  mc: "MC",
  tf: "T/F",
  short: "Short",
  essay: "Essay",
  code: "Code",
};

interface PreviewQuestion {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  orderIndex: number;
}

interface AssessmentDetail {
  id: string;
  title: string;
  mode?: string;
  status?: string;
  course?: string;
  objectives?: string[];
  questions: PreviewQuestion[];
}

export default function AssessmentPreviewPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const { mode } = useColorMode();
  const isDark = mode === "dark";
  const { user } = useAuth();
  const router = useRouter();
  const [assessment, setAssessment] = useState<AssessmentDetail | null>(null);
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    api
      .GET("/v1/assessments/{id}", { params: { path: { id } } })
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      .then(({ data: d }: { data?: any }) => {
        if (d) setAssessment(d);
      })
      .catch(console.error);
  }, [id]);

  const handleStart = async () => {
    if (!assessment) return;
    setStarting(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (api as any).POST("/v1/sessions", {
        body: { assessmentId: id, count: assessment.questions.length },
      });
      const sessionId = data?.sessionId ?? data?.session_id;
      if (sessionId) router.push(`/sessions/${sessionId}`);
    } finally {
      setStarting(false);
    }
  };

  if (!assessment) {
    return (
      <Box
        sx={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <CircularProgress />
      </Box>
    );
  }

  const totalPoints = assessment.questions.reduce((s, q) => s + q.points, 0);
  const kindCounts = assessment.questions.reduce(
    (acc, q) => ({ ...acc, [q.kind]: (acc[q.kind] ?? 0) + 1 }),
    {} as Record<string, number>,
  );
  const kindSummary = Object.entries(kindCounts)
    .map(([k, n]) => `${n} ${KIND_LABEL[k] ?? k}`)
    .join(" · ");

  const sorted = [...assessment.questions].sort(
    (a, b) => a.orderIndex - b.orderIndex,
  );

  return (
    <Box sx={{ pt: 5, px: 5, pb: 8, maxWidth: 860 }}>
      {/* Breadcrumb */}
      <Box sx={{ display: "flex", gap: 1, alignItems: "center", mb: 2 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: "monospace",
            letterSpacing: 1.1,
            textTransform: "uppercase",
            cursor: "pointer",
            textDecoration: "underline",
          }}
          onClick={() => router.push("/explore")}
        >
          Explore
        </Typography>
        <Typography variant="caption" color="text.secondary">
          ›
        </Typography>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: "monospace",
            letterSpacing: 1.1,
            textTransform: "uppercase",
          }}
        >
          {assessment.title}
        </Typography>
      </Box>

      {/* Header */}
      <Box sx={{ mb: 3 }}>
        {assessment.course && (
          <Chip
            label={assessment.course}
            size="small"
            variant="outlined"
            sx={{ mb: 1.25, ...tagColor(assessment.course, isDark) }}
          />
        )}
        <Stack
          direction="row"
          spacing={1.5}
          sx={{ alignItems: "center", mb: 2 }}
        >
          <Typography variant="h4" sx={{ fontWeight: 500 }}>
            {assessment.title}
          </Typography>
          {assessment.status && (
            <Chip
              label={assessment.status.toUpperCase()}
              size="small"
              variant="outlined"
              color={
                assessment.status === "active" || assessment.status === "live"
                  ? "success"
                  : assessment.status === "draft"
                    ? "warning"
                    : "default"
              }
            />
          )}
        </Stack>
        {assessment.objectives && assessment.objectives.length > 0 && (
          <Box component="ul" sx={{ m: 0, pl: 2.25, color: "text.secondary" }}>
            {assessment.objectives.map((obj, i) => (
              <Typography
                key={i}
                component="li"
                variant="body2"
                sx={{ lineHeight: 1.7 }}
              >
                {obj}
              </Typography>
            ))}
          </Box>
        )}
      </Box>

      {/* Stat strip */}
      <Stack direction="row" spacing={2.5} sx={{ mb: 3 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: "monospace",
            letterSpacing: 1.1,
            textTransform: "uppercase",
          }}
        >
          {assessment.questions.length} questions
        </Typography>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: "monospace",
            letterSpacing: 1.1,
            textTransform: "uppercase",
          }}
        >
          {totalPoints} pts
        </Typography>
        {kindSummary && (
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              fontFamily: "monospace",
              letterSpacing: 1.1,
              textTransform: "uppercase",
            }}
          >
            {kindSummary}
          </Typography>
        )}
      </Stack>

      {/* Question list */}
      <Card variant="outlined" sx={{ mb: 4 }}>
        {sorted.map((q, i) => (
          <Box
            key={q.id}
            sx={{
              px: "22px",
              py: 2,
              borderBottom: i < sorted.length - 1 ? 1 : 0,
              borderColor: "divider",
              display: "grid",
              gridTemplateColumns: "28px 1fr auto",
              gap: 1.75,
              alignItems: "start",
            }}
          >
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ fontFamily: "monospace", pt: 0.25 }}
            >
              {i + 1}
            </Typography>
            <Typography
              variant="body2"
              sx={{
                color: "text.primary",
                lineHeight: 1.5,
                display: "-webkit-box",
                WebkitLineClamp: 2,
                WebkitBoxOrient: "vertical",
                overflow: "hidden",
              }}
            >
              {q.prompt}
            </Typography>
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ fontFamily: "monospace", whiteSpace: "nowrap", pt: 0.25 }}
            >
              {q.points} pt{q.points !== 1 ? "s" : ""}
            </Typography>
          </Box>
        ))}
      </Card>

      {/* Footer */}
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          pt: 3,
          borderTop: 1,
          borderColor: "divider",
        }}
      >
        <Button variant="outlined" onClick={() => router.push("/explore")}>
          Back to Explore
        </Button>
        <Button
          variant="contained"
          onClick={handleStart}
          disabled={
            starting ||
            assessment.status === "draft" ||
            assessment.questions.length === 0
          }
        >
          {starting
            ? "Starting…"
            : assessment.status === "draft"
              ? "Cannot start draft"
              : assessment.questions.length === 0
                ? "No questions available"
                : assessment.mode === "graded"
                  ? "Start exam"
                  : "Start assessment"}
        </Button>
      </Box>
    </Box>
  );
}
