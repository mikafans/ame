"use client";

import { useEffect, useState } from "react";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Alert from "@mui/material/Alert";
import { api } from "@/api/client";
import { FlashcardReview } from "@/components/flashcards/FlashcardReview";
import { buildDeck, filterByTypes, type FlashQuestion } from "@/lib/flashcards";
import { vibrantTagColor } from "@/lib/tagColor";

interface TagItem {
  name: string;
}

const QUESTION_TYPES = [
  { value: "mc", label: "Multiple choice" },
  { value: "tf", label: "True / false" },
  { value: "short", label: "Short answer" },
  { value: "essay", label: "Essay" },
  { value: "code", label: "Code" },
];

const DECK_SIZES = [10, 30, 50];

type Phase = "setup" | "review" | "summary";

export default function FlashcardsPage() {
  const [phase, setPhase] = useState<Phase>("setup");

  const [tags, setTags] = useState<TagItem[] | null>(null);
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [deckSize, setDeckSize] = useState(10);

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);

  const [deck, setDeck] = useState<FlashQuestion[]>([]);
  const [cursor, setCursor] = useState(0);
  const [missed, setMissed] = useState<FlashQuestion[]>([]);
  const [knewCount, setKnewCount] = useState(0);

  useEffect(() => {
    api
      .GET("/v1/tags" as never)
      .then(({ data }: { data?: TagItem[] }) =>
        setTags(Array.isArray(data) ? data : []),
      )
      .catch(() => setTags([]));
  }, []);

  function toggleType(value: string) {
    setSelectedTypes((prev) =>
      prev.includes(value) ? prev.filter((t) => t !== value) : [...prev, value],
    );
  }

  function startReview(questions: FlashQuestion[]) {
    setDeck(questions);
    setCursor(0);
    setMissed([]);
    setKnewCount(0);
    setPhase("review");
  }

  async function handleStart() {
    setError(null);
    setNote(null);
    setLoading(true);
    try {
      const params: Record<string, string | number> = {
        status: "live",
        limit: 200,
      };
      if (selectedTag) params.tag = selectedTag;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data, error: apiErr } = await (api as any).GET("/v1/questions", {
        params: { query: params },
      });
      if (apiErr) {
        setError("Could not load questions.");
        return;
      }
      const all = ((data?.questions ?? []) as FlashQuestion[]) ?? [];
      const filtered = filterByTypes(all, selectedTypes);
      if (filtered.length === 0) {
        setError("No live questions match those filters.");
        return;
      }
      if (filtered.length < deckSize) {
        setNote(
          `Only ${filtered.length} live questions match — using all of them.`,
        );
      }
      startReview(buildDeck(filtered, deckSize));
    } catch {
      setError("Could not reach the API.");
    } finally {
      setLoading(false);
    }
  }

  function handleRate(knew: boolean) {
    const current = deck[cursor];
    if (knew) {
      setKnewCount((n) => n + 1);
    } else {
      setMissed((m) => [...m, current]);
    }
    if (cursor + 1 >= deck.length) {
      setPhase("summary");
    } else {
      setCursor((c) => c + 1);
    }
  }

  // Skip-for-now: re-queue the current card to the end of the deck without
  // counting it. No-op on the last remaining card (nothing left to defer past).
  function handleSkip() {
    setDeck((d) => {
      if (d.length <= 1 || cursor >= d.length) return d;
      const next = [...d];
      const [card] = next.splice(cursor, 1);
      next.push(card);
      return next;
    });
  }

  function reviewMissed() {
    if (missed.length === 0) return;
    startReview(buildDeck(missed, missed.length));
  }

  // ---- Setup phase ----
  if (phase === "setup") {
    return (
      <Box sx={{ p: "28px 36px 56px", maxWidth: 640 }}>
        <Kicker>Flashcards</Kicker>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 4 }}>
          Review deck
        </Typography>

        <SetupBlock label="Topic" kicker="Filter by tag (optional)">
          {tags === null ? (
            <Typography variant="body2" color="text.secondary">
              Loading tags…
            </Typography>
          ) : (
            <Stack direction="row" sx={{ flexWrap: "wrap", gap: 1 }}>
              <Chip
                label="All topics"
                size="small"
                onClick={() => setSelectedTag(null)}
                color={selectedTag === null ? "primary" : "default"}
                variant={selectedTag === null ? "filled" : "outlined"}
                sx={{ cursor: "pointer" }}
              />
              {tags.map((t) => (
                <Chip
                  key={t.name}
                  label={t.name}
                  size="small"
                  onClick={() => setSelectedTag(t.name)}
                  sx={{
                    cursor: "pointer",
                    ...(selectedTag === t.name
                      ? vibrantTagColor(t.name)
                      : { variant: "outlined" }),
                  }}
                />
              ))}
            </Stack>
          )}
        </SetupBlock>

        <SetupBlock label="Question types" kicker="Leave empty for all types">
          <Stack direction="row" sx={{ flexWrap: "wrap", gap: 1 }}>
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

        <SetupBlock label="Deck size" kicker="How many cards">
          <Stack direction="row" spacing={1}>
            {DECK_SIZES.map((n) => (
              <Chip
                key={n}
                label={n}
                size="small"
                onClick={() => setDeckSize(n)}
                color={deckSize === n ? "primary" : "default"}
                variant={deckSize === n ? "filled" : "outlined"}
                sx={{ cursor: "pointer", fontFamily: "monospace" }}
              />
            ))}
          </Stack>
        </SetupBlock>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        <Button
          variant="contained"
          size="large"
          disabled={loading}
          onClick={handleStart}
        >
          {loading ? "Loading…" : "Start deck →"}
        </Button>
      </Box>
    );
  }

  // ---- Review phase ----
  if (phase === "review") {
    return (
      <Box sx={{ p: "28px 36px 56px" }}>
        {note && (
          <Alert severity="info" sx={{ mb: 2, maxWidth: 680 }}>
            {note}
          </Alert>
        )}
        <FlashcardReview
          question={deck[cursor]}
          index={cursor}
          total={deck.length}
          onRate={handleRate}
          onSkip={handleSkip}
        />
      </Box>
    );
  }

  // ---- Summary phase ----
  return (
    <Box sx={{ p: "28px 36px 56px", maxWidth: 640 }}>
      <Kicker>Deck complete</Kicker>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>
        You knew {knewCount} of {deck.length}
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mb: 4 }}>
        {missed.length === 0
          ? "Clean sweep — nothing missed."
          : `${missed.length} to review again.`}
      </Typography>
      <Stack direction="row" spacing={1.5}>
        <Button
          variant="contained"
          size="large"
          onClick={reviewMissed}
          disabled={missed.length === 0}
        >
          Review missed ({missed.length})
        </Button>
        <Button
          variant="outlined"
          size="large"
          onClick={() => setPhase("setup")}
        >
          New deck
        </Button>
      </Stack>
    </Box>
  );
}

function Kicker({ children }: { children: React.ReactNode }) {
  return (
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
      {children}
    </Typography>
  );
}

function SetupBlock({
  label,
  kicker,
  children,
}: {
  label: string;
  kicker?: string;
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
        </Box>
        {children}
      </CardContent>
    </Card>
  );
}
