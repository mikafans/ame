"use client";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { PageShell } from "@/components/PageShell";
import { FlashcardReview } from "@/components/flashcards/FlashcardReview";
import { Button } from "@/components/ui/button";
import { buildDeck, filterByTypes, type FlashQuestion } from "@/lib/flashcards";
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
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [deckSize, setDeckSize] = useState(10);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [deck, setDeck] = useState<FlashQuestion[]>([]);
  const [cursor, setCursor] = useState(0);
  const [missed, setMissed] = useState<FlashQuestion[]>([]);
  const [knewCount, setKnewCount] = useState(0);
  const [skipped, setSkipped] = useState<FlashQuestion[]>([]);
  useEffect(() => {
    api
      .GET("/v1/tags" as never)
      .then(({ data }: { data?: TagItem[] }) =>
        setTags(Array.isArray(data) ? data : []),
      )
      .catch(() => setTags([]));
  }, []);
  const toggleType = (v: string) =>
    setSelectedTypes((p) =>
      p.includes(v) ? p.filter((x) => x !== v) : [...p, v],
    );
  const startReview = (questions: FlashQuestion[]) => {
    setDeck(questions);
    setCursor(0);
    setMissed([]);
    setKnewCount(0);
    setSkipped([]);
    setPhase("review");
  };
  async function handleStart() {
    setError(null);
    setNote(null);
    setLoading(true);
    try {
      let all: FlashQuestion[] = [];
      if (selectedTags.length) {
        const results = await Promise.all(
          selectedTags.map(async (tag) => {
            const { data } = await (api as any).GET("/v1/questions", {
              params: { query: { status: "live", limit: 200, tag } },
            });
            return (data?.questions ?? []) as FlashQuestion[];
          }),
        );
        const seen = new Set<string>();
        results.flat().forEach((q) => {
          if (!seen.has(q.id)) {
            seen.add(q.id);
            all.push(q);
          }
        });
      } else {
        const { data } = await (api as any).GET("/v1/questions", {
          params: { query: { status: "live", limit: 200 } },
        });
        all = (data?.questions ?? []) as FlashQuestion[];
      }
      const filtered = filterByTypes(all, selectedTypes);
      if (!filtered.length) {
        setError("No live questions match those filters.");
        return;
      }
      if (filtered.length < deckSize)
        setNote(
          `Only ${filtered.length} live questions match — using all of them.`,
        );
      startReview(buildDeck(filtered, deckSize));
    } catch {
      setError("Could not reach the API.");
    } finally {
      setLoading(false);
    }
  }
  function handleRate(knew: boolean) {
    const current = deck[cursor];
    if (knew) setKnewCount((n) => n + 1);
    else setMissed((m) => [...m, current]);
    if (cursor + 1 < deck.length) setCursor((c) => c + 1);
    else if (skipped.length) {
      setDeck(skipped);
      setCursor(0);
      setSkipped([]);
    } else setPhase("summary");
  }
  function handleSkip() {
    const current = deck[cursor];
    const next = [...skipped, current];
    if (cursor + 1 < deck.length) {
      setSkipped(next);
      setCursor((c) => c + 1);
    } else if (next.length) {
      setDeck(next);
      setCursor(0);
      setSkipped([]);
    } else setPhase("summary");
  }
  const reviewMissed = () =>
    missed.length && startReview(buildDeck(missed, missed.length));
  if (phase === "setup")
    return (
      <PageShell
        kicker="Flashcards"
        title="Review deck"
        subtitle="Turn live questions into a quick recall deck."
        maxWidth={640}
      >
        <SetupBlock label="Topics" kicker="Filter by topics (optional)">
          {tags === null ? (
            <p className="text-sm text-muted-foreground">Loading topics…</p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {tags.map((tag) => (
                <button
                  key={tag.name}
                  onClick={() =>
                    setSelectedTags((p) =>
                      p.includes(tag.name)
                        ? p.filter((x) => x !== tag.name)
                        : [...p, tag.name],
                    )
                  }
                  className={`rounded-full border px-3 py-1.5 text-xs ${selectedTags.includes(tag.name) ? "border-primary bg-primary text-primary-foreground" : "border-border text-muted-foreground"}`}
                >
                  {tag.name}
                </button>
              ))}
            </div>
          )}
        </SetupBlock>
        <SetupBlock label="Question types" kicker="Leave empty for all types">
          <div className="flex flex-wrap gap-2">
            {QUESTION_TYPES.map((qt) => (
              <button
                key={qt.value}
                onClick={() => toggleType(qt.value)}
                className={`rounded-full border px-3 py-1.5 text-xs ${selectedTypes.includes(qt.value) ? "border-primary bg-primary text-primary-foreground" : "border-border text-muted-foreground"}`}
              >
                {qt.label}
              </button>
            ))}
          </div>
        </SetupBlock>
        <SetupBlock label="Deck size" kicker="How many cards">
          <div className="flex gap-2">
            {DECK_SIZES.map((n) => (
              <button
                key={n}
                onClick={() => setDeckSize(n)}
                className={`rounded-full border px-3 py-1.5 font-mono text-xs ${deckSize === n ? "border-primary bg-primary text-primary-foreground" : "border-border text-muted-foreground"}`}
              >
                {n}
              </button>
            ))}
          </div>
        </SetupBlock>
        {error && (
          <div
            role="alert"
            className="mb-4 rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
          >
            {error}
          </div>
        )}
        {note && <p className="mb-4 text-sm text-muted-foreground">{note}</p>}
        <Button size="lg" disabled={loading} onClick={handleStart}>
          {loading ? "Loading…" : "Start deck →"}
        </Button>
      </PageShell>
    );
  if (phase === "review")
    return (
      <div className="px-4 pb-16 pt-10 sm:px-12">
        {note && (
          <div className="mb-4 max-w-[680px] rounded-md border border-blue-500/40 bg-blue-500/10 px-4 py-3 text-sm text-blue-700 dark:text-blue-300">
            {note}
          </div>
        )}
        <FlashcardReview
          question={deck[cursor]}
          index={cursor}
          total={deck.length}
          onRate={handleRate}
          onSkip={handleSkip}
          onFinish={() => setPhase("summary")}
        />
      </div>
    );
  return (
    <PageShell kicker="Deck complete" title="Review summary" maxWidth={640}>
      <h2 className="mb-1 text-xl font-semibold">
        You knew {knewCount} of {knewCount + missed.length}
      </h2>
      <p className="mb-6 text-sm text-muted-foreground">
        {knewCount + missed.length === 0
          ? "No cards rated in this session."
          : missed.length === 0
            ? "Clean sweep — nothing missed."
            : `${missed.length} to review again.`}
      </p>
      <div className="flex flex-wrap gap-3">
        <Button size="lg" onClick={reviewMissed} disabled={!missed.length}>
          Review missed ({missed.length})
        </Button>
        <Button size="lg" variant="outline" onClick={() => setPhase("setup")}>
          New deck
        </Button>
      </div>
    </PageShell>
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
    <section className="mb-6 rounded-lg border border-border p-5">
      <div className="mb-4 flex items-baseline gap-3">
        <h2 className="text-sm font-semibold">{label}</h2>
        {kicker && (
          <span className="font-mono text-xs tracking-wide text-muted-foreground">
            {kicker}
          </span>
        )}
      </div>
      {children}
    </section>
  );
}
