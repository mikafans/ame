"use client";

import { useState, useEffect, FormEvent } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";

interface Tag {
  name: string;
}

const QUESTION_TYPES = [
  { value: "mcq", label: "Multiple choice" },
  { value: "free_text", label: "Free text" },
  { value: "cloze", label: "Fill-in-the-blank" },
];

export default function PracticePage() {
  const { token } = useAuth();
  const router = useRouter();
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [count, setCount] = useState(10);
  const [duration, setDuration] = useState<number | "">(20);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) return;
    makeClient(token)
      .GET("/tags" as never)
      .then(({ data }: { data?: { tags: Tag[] } }) => {
        if (data?.tags) setTags(data.tags);
      })
      .catch(console.error);
  }, [token]);

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
    if (!token) return;
    setError(null);
    setLoading(true);
    try {
      const client = makeClient(token);
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
    <div style={{ padding: "28px 36px 56px", maxWidth: 640 }}>
      <div
        style={{
          fontFamily: "var(--mono)",
          fontSize: 10,
          letterSpacing: 1.3,
          textTransform: "uppercase",
          color: "var(--muted)",
          marginBottom: 6,
        }}
      >
        Session setup
      </div>
      <h1
        style={{
          margin: "0 0 32px",
          fontSize: 24,
          fontWeight: 600,
          color: "var(--text)",
        }}
      >
        Practice
      </h1>

      <form onSubmit={handleStart}>
        {/* Tags */}
        <SetupBlock label="Topics" kicker="Filter by tag">
          {tags.length === 0 ? (
            <span style={{ color: "var(--muted)", fontSize: 13 }}>
              Loading tags…
            </span>
          ) : (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
              {tags.map((t) => {
                const active = selectedTags.includes(t.name);
                return (
                  <button
                    key={t.name}
                    type="button"
                    onClick={() => toggleTag(t.name)}
                    style={{
                      padding: "5px 12px",
                      background: active
                        ? "var(--accent-dim)"
                        : "var(--surface-2)",
                      border: `1px solid ${active ? "var(--accent-line)" : "var(--border)"}`,
                      borderRadius: 4,
                      color: active ? "var(--accent)" : "var(--text-2)",
                      cursor: "pointer",
                      fontSize: 12,
                      fontFamily: "var(--mono)",
                    }}
                  >
                    {t.name}
                  </button>
                );
              })}
            </div>
          )}
        </SetupBlock>

        {/* Question types */}
        <SetupBlock
          label="Question types"
          kicker="Leave empty for all types"
          hint="Mix and match"
        >
          <div style={{ display: "flex", gap: 8 }}>
            {QUESTION_TYPES.map((qt) => {
              const active = selectedTypes.includes(qt.value);
              return (
                <button
                  key={qt.value}
                  type="button"
                  onClick={() => toggleType(qt.value)}
                  style={{
                    padding: "6px 14px",
                    background: active
                      ? "var(--accent-dim)"
                      : "var(--surface-2)",
                    border: `1px solid ${active ? "var(--accent-line)" : "var(--border)"}`,
                    borderRadius: 4,
                    color: active ? "var(--accent)" : "var(--text-2)",
                    cursor: "pointer",
                    fontSize: 12,
                  }}
                >
                  {qt.label}
                </button>
              );
            })}
          </div>
        </SetupBlock>

        {/* Count */}
        <SetupBlock label="Questions" kicker="How many">
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {[5, 10, 20, 50].map((n) => (
              <button
                key={n}
                type="button"
                onClick={() => setCount(n)}
                style={{
                  padding: "6px 14px",
                  background:
                    count === n ? "var(--accent-dim)" : "var(--surface-2)",
                  border: `1px solid ${count === n ? "var(--accent-line)" : "var(--border)"}`,
                  borderRadius: 4,
                  color: count === n ? "var(--accent)" : "var(--text-2)",
                  cursor: "pointer",
                  fontSize: 13,
                  fontFamily: "var(--mono)",
                }}
              >
                {n}
              </button>
            ))}
            <input
              type="number"
              min={1}
              max={200}
              value={count}
              onChange={(e) => setCount(parseInt(e.target.value) || 10)}
              style={{
                width: 64,
                padding: "6px 10px",
                background: "var(--surface-2)",
                border: "1px solid var(--border)",
                borderRadius: 4,
                color: "var(--text)",
                fontSize: 13,
                fontFamily: "var(--mono)",
                textAlign: "center",
              }}
            />
          </div>
        </SetupBlock>

        {/* Duration */}
        <SetupBlock label="Time limit" kicker="Minutes (optional)">
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {[10, 20, 30, 60].map((n) => (
              <button
                key={n}
                type="button"
                onClick={() => setDuration(duration === n ? "" : n)}
                style={{
                  padding: "6px 14px",
                  background:
                    duration === n ? "var(--accent-dim)" : "var(--surface-2)",
                  border: `1px solid ${duration === n ? "var(--accent-line)" : "var(--border)"}`,
                  borderRadius: 4,
                  color: duration === n ? "var(--accent)" : "var(--text-2)",
                  cursor: "pointer",
                  fontSize: 13,
                  fontFamily: "var(--mono)",
                }}
              >
                {n}m
              </button>
            ))}
            <button
              type="button"
              onClick={() => setDuration("")}
              style={{
                padding: "6px 14px",
                background:
                  duration === "" ? "var(--accent-dim)" : "var(--surface-2)",
                border: `1px solid ${duration === "" ? "var(--accent-line)" : "var(--border)"}`,
                borderRadius: 4,
                color: duration === "" ? "var(--accent)" : "var(--muted)",
                cursor: "pointer",
                fontSize: 12,
              }}
            >
              No limit
            </button>
          </div>
        </SetupBlock>

        {error && (
          <div
            style={{
              padding: "10px 14px",
              background: "var(--red-dim)",
              border: "1px solid var(--red)",
              borderRadius: 4,
              color: "var(--red)",
              fontSize: 13,
              marginBottom: 16,
            }}
          >
            {error}
          </div>
        )}

        <button
          type="submit"
          disabled={loading}
          style={{
            padding: "11px 28px",
            background: "var(--accent)",
            border: "none",
            borderRadius: 4,
            color: "#000",
            fontWeight: 700,
            fontSize: 14,
            cursor: loading ? "not-allowed" : "pointer",
            opacity: loading ? 0.7 : 1,
          }}
        >
          {loading ? "Starting…" : "Start session →"}
        </button>
      </form>
    </div>
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
    <div
      style={{
        marginBottom: 28,
        padding: "18px 20px",
        background: "var(--surface)",
        border: "1px solid var(--border)",
        borderRadius: 6,
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          gap: 10,
          marginBottom: 14,
        }}
      >
        <span style={{ fontWeight: 600, fontSize: 14, color: "var(--text)" }}>
          {label}
        </span>
        {kicker && (
          <span
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10.5,
              letterSpacing: 0.8,
              color: "var(--muted)",
            }}
          >
            {kicker}
          </span>
        )}
        {hint && (
          <span
            style={{ marginLeft: "auto", fontSize: 11, color: "var(--muted)" }}
          >
            {hint}
          </span>
        )}
      </div>
      {children}
    </div>
  );
}
