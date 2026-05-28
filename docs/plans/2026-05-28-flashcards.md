# Flashcard Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a learner-facing Flashcard page that draws a shuffled deck of 10/30/50 live questions, flips each card to reveal answer + explanation, lets the learner self-rate (Got it / Missed it), and shows a session summary.

**Architecture:** Pure frontend. A single client-component route `web/app/(learner)/flashcards/page.tsx` runs a `setup → review → summary` phase machine. All answer-derivation and deck-building logic lives in a pure, unit-tested module (`web/src/lib/flashcards.ts`). Data comes from the existing `GET /v1/questions` endpoint (returns full `payload` + `explanation` to any authenticated user). No API or DB changes.

**Tech Stack:** Next.js 16 (App Router, client component), React 19, MUI 7, `openapi-fetch` client (`@/api/client`), `bun test` for unit tests, Playwright for e2e.

---

### Task 1: Pure flashcard logic module

Answer-derivation, type-filtering, and deck-building as pure functions — the only part with branching logic, so the only part worth unit-testing.

**Files:**
- Create: `web/src/lib/flashcards.ts`
- Test: `web/src/lib/flashcards.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// web/src/lib/flashcards.test.ts
import { describe, it, expect } from "bun:test";
import {
  deriveBack,
  hasModelAnswer,
  filterByTypes,
  buildDeck,
  type FlashQuestion,
} from "./flashcards";

function q(partial: Partial<FlashQuestion>): FlashQuestion {
  return {
    id: "00000000-0000-0000-0000-000000000000",
    kind: "mc",
    prompt: "Q?",
    payload: {},
    explanation: null,
    ...partial,
  };
}

describe("deriveBack", () => {
  it("mc → option at correct_index", () => {
    const back = deriveBack(
      q({ kind: "mc", payload: { options: ["a", "b", "c"], correct_index: 1 } }),
    );
    expect(back.answer).toBe("b");
  });

  it("tf → True/False string", () => {
    expect(deriveBack(q({ kind: "tf", payload: { correct: true } })).answer).toBe("True");
    expect(deriveBack(q({ kind: "tf", payload: { correct: false } })).answer).toBe("False");
  });

  it("short → accepted joined", () => {
    const back = deriveBack(q({ kind: "short", payload: { accepted: ["x", "y"] } }));
    expect(back.answer).toBe("x, y");
  });

  it("essay/code → no crisp answer, explanation passed through", () => {
    const back = deriveBack(q({ kind: "essay", explanation: "discuss tradeoffs" }));
    expect(back.answer).toBeNull();
    expect(back.explanation).toBe("discuss tradeoffs");
  });

  it("malformed mc payload → null answer", () => {
    expect(deriveBack(q({ kind: "mc", payload: {} })).answer).toBeNull();
  });
});

describe("hasModelAnswer", () => {
  it("true when an answer exists", () => {
    expect(hasModelAnswer({ answer: "b", explanation: null })).toBe(true);
  });
  it("true when only explanation exists", () => {
    expect(hasModelAnswer({ answer: null, explanation: "because" })).toBe(true);
  });
  it("false when neither", () => {
    expect(hasModelAnswer({ answer: null, explanation: "  " })).toBe(false);
  });
});

describe("filterByTypes", () => {
  const items = [q({ kind: "mc" }), q({ kind: "tf" }), q({ kind: "essay" })];
  it("empty selection keeps all", () => {
    expect(filterByTypes(items, [])).toHaveLength(3);
  });
  it("keeps only selected kinds", () => {
    expect(filterByTypes(items, ["mc", "tf"]).map((x) => x.kind)).toEqual(["mc", "tf"]);
  });
});

describe("buildDeck", () => {
  const items = Array.from({ length: 100 }, (_, i) => q({ id: String(i) }));
  it("slices to count", () => {
    expect(buildDeck(items, 30)).toHaveLength(30);
  });
  it("returns all when fewer than count", () => {
    expect(buildDeck(items.slice(0, 5), 30)).toHaveLength(5);
  });
  it("is a permutation (no dropped/duplicated items) for full deck", () => {
    const ids = buildDeck(items, 100).map((x) => x.id).sort();
    expect(ids).toEqual(items.map((x) => x.id).sort());
  });
  it("uses injected rng deterministically", () => {
    const seq = [0.1, 0.9, 0.3];
    let i = 0;
    const rng = () => seq[i++ % seq.length];
    const a = buildDeck(items.slice(0, 3), 3, rng);
    i = 0;
    const b = buildDeck(items.slice(0, 3), 3, rng);
    expect(a.map((x) => x.id)).toEqual(b.map((x) => x.id));
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && mise exec -- bun test src/lib/flashcards.test.ts`
Expected: FAIL — `Cannot find module "./flashcards"`.

- [ ] **Step 3: Write minimal implementation**

```ts
// web/src/lib/flashcards.ts
export type Kind = "mc" | "tf" | "short" | "essay" | "code";

export interface FlashQuestion {
  id: string;
  kind: Kind;
  prompt: string;
  payload: Record<string, unknown>;
  explanation: string | null;
  code_snippet?: unknown;
}

export interface CardBack {
  answer: string | null;
  explanation: string | null;
}

export function deriveBack(q: FlashQuestion): CardBack {
  const p = q.payload ?? {};
  let answer: string | null = null;

  switch (q.kind) {
    case "mc": {
      const options = p.options as unknown;
      const idx = p.correct_index as unknown;
      if (Array.isArray(options) && typeof idx === "number") {
        answer = (options[idx] as string) ?? null;
      }
      break;
    }
    case "tf": {
      if (typeof p.correct === "boolean") answer = p.correct ? "True" : "False";
      break;
    }
    case "short": {
      const accepted = p.accepted as unknown;
      if (Array.isArray(accepted) && accepted.length > 0) {
        answer = (accepted as string[]).join(", ");
      }
      break;
    }
    case "essay":
    case "code":
      answer = null;
      break;
  }

  return { answer, explanation: q.explanation ?? null };
}

export function hasModelAnswer(back: CardBack): boolean {
  if (back.answer !== null && back.answer.trim() !== "") return true;
  return back.explanation !== null && back.explanation.trim() !== "";
}

export function filterByTypes<T extends { kind: Kind }>(items: T[], types: string[]): T[] {
  if (types.length === 0) return items;
  return items.filter((i) => types.includes(i.kind));
}

export function shuffle<T>(items: T[], rng: () => number = Math.random): T[] {
  const a = [...items];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

export function buildDeck<T>(items: T[], count: number, rng: () => number = Math.random): T[] {
  return shuffle(items, rng).slice(0, count);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && mise exec -- bun test src/lib/flashcards.test.ts`
Expected: PASS (all describe blocks green).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/flashcards.ts web/src/lib/flashcards.test.ts
git commit -m "feat: add flashcard deck + answer-derivation logic"
```

---

### Task 2: Flashcard review component

A single card with front (prompt) → flip → back (answer + explanation) and Got it / Missed it controls, including keyboard shortcuts. Self-contained and presentational — parent owns deck state.

**Files:**
- Create: `web/src/components/flashcards/FlashcardReview.tsx`

- [ ] **Step 1: Write the component**

```tsx
// web/src/components/flashcards/FlashcardReview.tsx
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
```

- [ ] **Step 2: Verify it type-checks**

Run: `cd web && mise exec -- bun run type-check`
Expected: PASS (no errors).

- [ ] **Step 3: Commit**

```bash
git add web/src/components/flashcards/FlashcardReview.tsx
git commit -m "feat: add flashcard review card component"
```

---

### Task 3: Flashcards page (phase machine)

The route that wires setup → review → summary, fetches questions, and tracks confidence.

**Files:**
- Create: `web/app/(learner)/flashcards/page.tsx`

- [ ] **Step 1: Write the page**

```tsx
// web/app/(learner)/flashcards/page.tsx
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
import {
  buildDeck,
  filterByTypes,
  type FlashQuestion,
} from "@/lib/flashcards";

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
      .then(({ data }: { data?: TagItem[] }) => setTags(Array.isArray(data) ? data : []))
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
      const params: Record<string, string | number> = { status: "live", limit: 200 };
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
        setNote(`Only ${filtered.length} live questions match — using all of them.`);
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
                  color={selectedTag === t.name ? "primary" : "default"}
                  variant={selectedTag === t.name ? "filled" : "outlined"}
                  sx={{ cursor: "pointer" }}
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
                variant={selectedTypes.includes(qt.value) ? "filled" : "outlined"}
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

        <Button variant="contained" size="large" disabled={loading} onClick={handleStart}>
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
        <Button variant="outlined" size="large" onClick={() => setPhase("setup")}>
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
        <Box sx={{ display: "flex", alignItems: "baseline", gap: 1.25, mb: 1.75 }}>
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
```

- [ ] **Step 2: Verify it type-checks**

Run: `cd web && mise exec -- bun run type-check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add web/app/\(learner\)/flashcards/page.tsx
git commit -m "feat: add flashcards page with setup/review/summary phases"
```

---

### Task 4: Wire sidebar + layout navigation

Make the page reachable from the learner sidebar.

**Files:**
- Modify: `web/src/components/Sidebar.tsx` (`ICON_MAP` ~line 38-44, `items` array ~line 61-72)
- Modify: `web/app/(learner)/layout.tsx` (`getRouteId` ~line 24-35, `routeMap` ~line 38-49)

- [ ] **Step 1: Add the icon import and ICON_MAP entry in Sidebar.tsx**

Add this import alongside the other `@mui/icons-material` imports (near line 14-24):

```tsx
import StyleOutlinedIcon from "@mui/icons-material/StyleOutlined";
```

Add to `ICON_MAP` (the object ending around line 44):

```tsx
  flashcards: <StyleOutlinedIcon fontSize="small" />,
```

- [ ] **Step 2: Add the nav item in Sidebar.tsx**

In the `items` array, add after the `quiz` ("Take quiz") entry:

```tsx
    { id: "flashcards", label: "Flashcards", icon: "flashcards", section: "Learn" },
```

- [ ] **Step 3: Add route mapping in layout.tsx**

In `getRouteId`, add before the final `return "library"`:

```tsx
    if (pathname.startsWith("/flashcards")) return "flashcards";
```

In `handleRouteChange`'s `routeMap`, add:

```tsx
      flashcards: "/flashcards",
```

- [ ] **Step 4: Verify type-check + lint**

Run: `cd web && mise exec -- bun run type-check && mise exec -- bun run lint`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/Sidebar.tsx web/app/\(learner\)/layout.tsx
git commit -m "feat: add flashcards link to learner sidebar"
```

---

### Task 5: Manual verification + full gate

Flashcards is interaction-heavy; verify in the browser, then run the pre-commit gate.

- [ ] **Step 1: Start the stack**

Run: `make dev`
Expected: API on :28080, web on :23000. (If a remote machine, `make dev API_HOST=harus-mini`.)

- [ ] **Step 2: Seed demo data if the bank is empty**

Run: `make db-seed`
Expected: questions/quizzes created.

- [ ] **Step 3: Exercise the feature in a browser**

Log in as a learner, click **Flashcards** in the sidebar. Verify:
- Setup: topic single-select, type multi-select, deck size 10/30/50.
- Start with no filters → a card appears with `n / total`.
- "Show answer" (and Space) reveals answer + explanation; essay/code show explanation only or "No model answer provided."
- Got it / Missed it (and keys 1/2, ←/→) advance.
- After the last card, summary shows "You knew X of N"; "Review missed" re-runs only missed cards and is disabled at 0; "New deck" returns to setup.
- Pick a type filter that has no matches → inline error, stays on setup.

- [ ] **Step 4: Run the pre-commit gate**

Run: `make check > /tmp/check.log 2>&1; echo exit=$?` then read `/tmp/check.log`.
Expected: exit=0 (fmt-check + lint + unit tests incl. `flashcards.test.ts` pass).

- [ ] **Step 5: Commit any fixes from verification**

```bash
git add -p
git commit -m "fix: address flashcard verification findings"
```

---

## Optional follow-up (not in this plan)

- Playwright e2e happy path (`web/e2e/flashcards.spec.ts`) using the project's
  established `addCookies({ name: "ame_token", value, url })` + `waitForLoadState`
  pattern. Deferred because make ci requires a seeded/clean DB and the UI logic is
  already covered by unit tests + manual verification.
- Persisting confidence / spaced repetition (needs backend) — out of scope.
