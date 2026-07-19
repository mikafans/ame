"use client";

import { useEffect, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { PageShell } from "@/components/PageShell";
import { api } from "@/api/client";

interface TagItem {
  name: string;
}

const QUESTION_TYPES = [
  { value: "mc", label: "Multiple choice" },
  { value: "tf", label: "True or false" },
  { value: "short", label: "Short answer" },
  { value: "essay", label: "Essay" },
  { value: "code", label: "Code" },
];

export default function PracticePage() {
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
      .then(({ data }: { data?: TagItem[] }) =>
        setTags(Array.isArray(data) ? data : []),
      )
      .catch(() => setTags([]));
  }, []);

  function toggleTag(name: string) {
    setSelectedTags((previous) =>
      previous.includes(name)
        ? previous.filter((tag) => tag !== name)
        : [...previous, name],
    );
  }

  function toggleType(value: string) {
    setSelectedTypes((previous) =>
      previous.includes(value)
        ? previous.filter((type) => type !== value)
        : [...previous, value],
    );
  }

  async function handleStart(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const { data, error: apiError } = await (api as any).POST(
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
      if (apiError) {
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
    <PageShell
      kicker="Session setup"
      title="Practice"
      subtitle="Build a focused practice session from your live question bank."
      maxWidth={640}
    >
      <form onSubmit={handleStart} className="space-y-5">
        <SetupBlock label="Topics" kicker="Filter by topic">
          {tags === null ? (
            <p className="text-sm text-muted-foreground">Loading topics…</p>
          ) : tags.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No topics yet — all questions will be included.
            </p>
          ) : (
            <div className="flex max-h-40 flex-wrap gap-2 overflow-y-auto">
              {tags.map((tag) => (
                <Choice
                  key={tag.name}
                  selected={selectedTags.includes(tag.name)}
                  onClick={() => toggleTag(tag.name)}
                >
                  {tag.name}
                </Choice>
              ))}
            </div>
          )}
        </SetupBlock>

        <SetupBlock
          label="Question types"
          kicker="Leave empty for all types"
          hint="Mix and match"
        >
          <div className="flex flex-wrap gap-2">
            {QUESTION_TYPES.map((questionType) => (
              <Choice
                key={questionType.value}
                selected={selectedTypes.includes(questionType.value)}
                onClick={() => toggleType(questionType.value)}
              >
                {questionType.label}
              </Choice>
            ))}
          </div>
        </SetupBlock>

        <SetupBlock label="Questions" kicker="How many">
          <div className="flex flex-wrap items-center gap-2">
            {[5, 10, 20, 50].map((value) => (
              <Choice
                key={value}
                selected={count === value}
                onClick={() => setCount(value)}
                mono
              >
                {value}
              </Choice>
            ))}
            <input
              aria-label="Custom question count"
              type="number"
              min={1}
              max={200}
              value={count}
              onChange={(event) =>
                setCount(Number.parseInt(event.target.value, 10) || 10)
              }
              className="h-9 w-16 rounded-lg border border-input bg-background px-2 text-center font-mono text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/20"
            />
          </div>
        </SetupBlock>

        <SetupBlock label="Time limit" kicker="Minutes (optional)">
          <div className="flex flex-wrap gap-2">
            {[10, 20, 30, 60].map((value) => (
              <Choice
                key={value}
                selected={duration === value}
                onClick={() => setDuration(duration === value ? "" : value)}
                mono
              >
                {value}m
              </Choice>
            ))}
            <Choice selected={duration === ""} onClick={() => setDuration("")}>
              No limit
            </Choice>
          </div>
        </SetupBlock>

        {error && (
          <div
            role="alert"
            className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
          >
            {error}
          </div>
        )}
        <Button type="submit" size="lg" disabled={loading}>
          {loading ? "Starting…" : "Start session →"}
        </Button>
      </form>
    </PageShell>
  );
}

function Choice({
  children,
  selected,
  onClick,
  mono = false,
}: {
  children: React.ReactNode;
  selected: boolean;
  onClick: () => void;
  mono?: boolean;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onClick}
      className={`rounded-lg border px-3 py-2 text-sm outline-none transition focus-visible:ring-2 focus-visible:ring-ring ${mono ? "font-mono" : ""} ${selected ? "border-primary bg-primary text-primary-foreground" : "border-border bg-background hover:bg-muted"}`}
    >
      {children}
    </button>
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
    <section className="rounded-xl border border-border bg-card p-5">
      <div className="mb-4 flex items-baseline gap-3">
        <h2 className="text-sm font-semibold">{label}</h2>
        {kicker && (
          <span className="font-mono text-xs tracking-wide text-muted-foreground">
            {kicker}
          </span>
        )}
        {hint && (
          <span className="ml-auto text-xs text-muted-foreground">{hint}</span>
        )}
      </div>
      {children}
    </section>
  );
}
