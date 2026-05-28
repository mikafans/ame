"use client";

import { useState, useEffect, FormEvent } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Alert from "@mui/material/Alert";

interface TagItem {
  name: string;
}

const QUESTION_TYPES = [
  { value: "mc", label: "Multiple choice" },
  { value: "tf", label: "True or false" },
  { value: "short", label: "Short answer" },
  { value: "essay", label: "Essay" },
];

export default function PracticePage() {
  const { user } = useAuth();
  const router = useRouter();
  const [tags, setTags] = useState<TagItem[] | null>(null);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [count, setCount] = useState(10);
  const [duration, setDuration] = useState<number | "">(20);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .GET("/v1/tags" as never)
      .then(({ data }: { data?: TagItem[] }) => {
        setTags(Array.isArray(data) ? data : []);
      })
      .catch(() => setTags([]));
  }, []);

  function toggleTag(name: string) {
    setSelectedTags((prev) =>
      prev.includes(name) ? prev.filter((t) => t !== name) : [...prev, name],
    );
  }

  function toggleType(value: string) {
    setSelectedTypes((prev) =>
      prev.includes(value) ? prev.filter((t) => t !== value) : [...prev, value],
    );
  }

  async function handleStart(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const client = api;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data, error: apiErr } = await (client as any).POST(
        "/v1/sessions",
        {
          body: {
            tags: selectedTags,
            types: selectedTypes.length ? selectedTypes : undefined,
            count,
            duration: duration !== "" ? duration : undefined,
          },
        },
      );
      if (apiErr) {
        setError(
          "Failed to start session. Make sure you have live questions for the selected filters.",
        );
        return;
      }
      router.push(`/sessions/${(data as { sessionId: string }).sessionId}`);
    } catch {
      setError("Could not reach the API.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <Box sx={{ p: "28px 36px 56px", maxWidth: 640 }}>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{
          fontFamily: "monospace",
          letterSpacing: 1.3,
          textTransform: "uppercase",
          display: "block",
          mb: 0.75,
        }}
      >
        Session setup
      </Typography>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 4 }}>
        Practice
      </Typography>

      <Box component="form" onSubmit={handleStart}>
        {/* Tags */}
        <SetupBlock label="Topics" kicker="Filter by tag">
          {tags === null ? (
            <Typography variant="body2" color="text.secondary">
              Loading tags…
            </Typography>
          ) : tags.length === 0 ? (
            <Typography variant="body2" color="text.secondary">
              No tags yet — all questions will be included.
            </Typography>
          ) : (
            <Stack direction="row" sx={{ flexWrap: "wrap", gap: 1 }}>
              {tags.map((t) => (
                <Chip
                  key={t.name}
                  label={t.name}
                  size="small"
                  onClick={() => toggleTag(t.name)}
                  color={selectedTags.includes(t.name) ? "primary" : "default"}
                  variant={
                    selectedTags.includes(t.name) ? "filled" : "outlined"
                  }
                  sx={{ cursor: "pointer" }}
                />
              ))}
            </Stack>
          )}
        </SetupBlock>

        {/* Question types */}
        <SetupBlock
          label="Question types"
          kicker="Leave empty for all types"
          hint="Mix and match"
        >
          <Stack direction="row" spacing={1}>
            {QUESTION_TYPES.map((qt) => (
              <Chip
                key={qt.value}
                label={qt.label}
                size="small"
                onClick={() => toggleType(qt.value)}
                color={selectedTypes.includes(qt.value) ? "primary" : "default"}
                variant={
                  selectedTypes.includes(qt.value) ? "filled" : "outlined"
                }
                sx={{ cursor: "pointer" }}
              />
            ))}
          </Stack>
        </SetupBlock>

        {/* Count */}
        <SetupBlock label="Questions" kicker="How many">
          <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
            {[5, 10, 20, 50].map((n) => (
              <Chip
                key={n}
                label={n}
                size="small"
                onClick={() => setCount(n)}
                color={count === n ? "primary" : "default"}
                variant={count === n ? "filled" : "outlined"}
                sx={{ cursor: "pointer", fontFamily: "monospace" }}
              />
            ))}
            <Box
              component="input"
              type="number"
              min={1}
              max={200}
              value={count}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                setCount(parseInt(e.target.value) || 10)
              }
              sx={{
                width: 64,
                p: "6px 10px",
                border: 1,
                borderColor: "divider",
                borderRadius: 1,
                fontFamily: "monospace",
                fontSize: 13,
                textAlign: "center",
                bgcolor: "background.paper",
                color: "text.primary",
                outline: "none",
              }}
            />
          </Stack>
        </SetupBlock>

        {/* Duration */}
        <SetupBlock label="Time limit" kicker="Minutes (optional)">
          <Stack direction="row" spacing={1}>
            {[10, 20, 30, 60].map((n) => (
              <Chip
                key={n}
                label={`${n}m`}
                size="small"
                onClick={() => setDuration(duration === n ? "" : n)}
                color={duration === n ? "primary" : "default"}
                variant={duration === n ? "filled" : "outlined"}
                sx={{ cursor: "pointer", fontFamily: "monospace" }}
              />
            ))}
            <Chip
              label="No limit"
              size="small"
              onClick={() => setDuration("")}
              color={duration === "" ? "primary" : "default"}
              variant={duration === "" ? "filled" : "outlined"}
              sx={{ cursor: "pointer" }}
            />
          </Stack>
        </SetupBlock>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        <Button
          type="submit"
          variant="contained"
          size="large"
          disabled={loading}
        >
          {loading ? "Starting…" : "Start session →"}
        </Button>
      </Box>
    </Box>
  );
}

function SetupBlock({
  label,
  kicker,
  hint,
  children,
}: {
  label: string;
  kicker?: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <Card variant="outlined" sx={{ mb: 3.5 }}>
      <CardContent>
        <Box
          sx={{ display: "flex", alignItems: "baseline", gap: 1.25, mb: 1.75 }}
        >
          <Typography variant="body2" sx={{ fontWeight: 600 }}>
            {label}
          </Typography>
          {kicker && (
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ fontFamily: "monospace", letterSpacing: 0.8 }}
            >
              {kicker}
            </Typography>
          )}
          {hint && (
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ ml: "auto" }}
            >
              {hint}
            </Typography>
          )}
        </Box>
        {children}
      </CardContent>
    </Card>
  );
}
