"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import {
  Check,
  Edit3,
  Eye,
  ExternalLink,
  LoaderCircle,
  Play,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { PageShell } from "@/components/PageShell";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { useAuth } from "@/hooks/useAuth";
import { formatDate } from "@/utils/format";

type AssessmentMode = "all" | "practice" | "graded";

function StatusTag({ status }: { status: string }) {
  const styles: Record<string, string> = {
    active:
      "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
    published:
      "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
    draft:
      "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
    archived:
      "border-slate-400/40 bg-slate-500/10 text-slate-600 dark:text-slate-300",
  };
  return (
    <span
      className={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${styles[status] ?? "border-border bg-muted text-muted-foreground"}`}
    >
      {status.replaceAll("_", " ")}
    </span>
  );
}

function TopicTag({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex rounded-full border border-sky-500/30 bg-sky-500/10 px-2 py-1 text-xs text-sky-700 dark:text-sky-300">
      {children}
    </span>
  );
}

export default function ExplorePage() {
  const router = useRouter();
  const { user, loading: authLoading } = useAuth();
  const [searchText, setSearchText] = useState("");
  const [search, setSearch] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [mode, setMode] = useState<AssessmentMode>("all");
  const [facets, setFacets] = useState<{
    tags: string[];
    counts: { practice: number; graded: number };
  } | null>(null);
  const [items, setItems] = useState<any[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [starting, setStarting] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);

  const fetchFacets = useCallback(async () => {
    if (authLoading || !user) return;
    try {
      const { data } = await api.GET("/v1/explore/facets");
      if (data) setFacets(data as any);
    } catch (error) {
      console.error("Failed to fetch facets:", error);
    }
  }, [authLoading, user]);

  const fetchItems = useCallback(
    async (cursor: string | null = null) => {
      if (authLoading || !user) return;
      setLoading(true);
      try {
        const { data } = await api.GET("/v1/explore", {
          params: {
            query: {
              search: search || undefined,
              tags: selectedTags.join(",") || undefined,
              mode: mode === "all" ? undefined : mode,
              kind: "active",
              limit: 50,
              after: cursor ?? undefined,
            } as any,
          },
        });
        if (data) {
          setItems((previous) =>
            cursor ? [...previous, ...data.items] : data.items,
          );
          setNextCursor(data.nextCursor ?? null);
        }
      } catch (error) {
        console.error("Failed to fetch items:", error);
      } finally {
        setLoading(false);
      }
    },
    [search, selectedTags, mode, authLoading, user],
  );

  useEffect(() => {
    fetchFacets();
  }, [fetchFacets]);

  useEffect(() => {
    if (authLoading || !user) return;
    setItems([]);
    setNextCursor(null);
    fetchItems();
  }, [search, selectedTags, mode, fetchItems, authLoading, user]);

  async function startAssessment(assessmentId: string) {
    setStarting(assessmentId);
    setStartError(null);
    try {
      const { data, error } = await api.POST("/v1/sessions", {
        body: { assessmentId },
      });
      if (error) {
        setStartError(errorMessage(error, "Failed to start assessment"));
        return;
      }
      if (data?.sessionId) router.push(`/sessions/${data.sessionId}`);
    } catch (error) {
      console.error(error);
      setStartError("Unexpected error — check the console");
    } finally {
      setStarting(null);
    }
  }

  const practiceCount = facets?.counts?.practice ?? 0;
  const gradedCount = facets?.counts?.graded ?? 0;

  return (
    <PageShell
      kicker="Assessments"
      title="Explore"
      subtitle="Browse and search practice assessments and graded exams"
    >
      <div className="flex flex-col gap-6">
        {startError && (
          <div
            role="alert"
            className="flex items-center justify-between rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive"
          >
            <span>{startError}</span>
            <button
              type="button"
              aria-label="Dismiss error"
              onClick={() => setStartError(null)}
              className="font-medium underline"
            >
              Dismiss
            </button>
          </div>
        )}

        <section className="rounded-xl border border-border bg-card p-5">
          <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center gap-3">
              <span className="text-sm font-semibold text-muted-foreground">
                Filter by type:
              </span>
              {(
                [
                  ["all", "All"],
                  ["practice", `Practice (${practiceCount})`],
                  ["graded", `Exam (${gradedCount})`],
                ] as const
              ).map(([value, label]) => (
                <button
                  key={value}
                  type="button"
                  aria-pressed={mode === value}
                  onClick={() => setMode(value)}
                  className={`rounded-lg border px-3 py-1.5 text-sm outline-none transition focus-visible:ring-2 focus-visible:ring-ring ${mode === value ? "border-primary bg-primary text-primary-foreground" : "border-border bg-background hover:bg-muted"}`}
                >
                  {label}
                </button>
              ))}
            </div>

            <div className="grid gap-3 md:grid-cols-[1fr_1.35fr_auto] md:items-end">
              <label
                className="grid gap-1.5 text-sm font-medium"
                htmlFor="explore-search"
              >
                Search title
                <input
                  id="explore-search"
                  value={searchText}
                  onChange={(event) => setSearchText(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") setSearch(searchText);
                  }}
                  className="h-10 rounded-lg border border-input bg-background px-3 font-normal outline-none focus:border-ring focus:ring-3 focus:ring-ring/20"
                />
              </label>
              <label
                className="grid gap-1.5 text-sm font-medium"
                htmlFor="explore-tags"
              >
                Topics
                <div
                  id="explore-tags"
                  aria-label="Filter by topics"
                  className="flex min-h-20 flex-wrap content-start gap-2 rounded-lg border border-input bg-background p-2 outline-none focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/20"
                >
                  {(facets?.tags ?? []).length === 0 ? (
                    <span className="px-1 py-1 text-xs font-normal text-muted-foreground">
                      No topics available yet
                    </span>
                  ) : (
                    (facets?.tags ?? []).map((tag) => {
                      const selected = selectedTags.includes(tag);
                      return (
                        <button
                          key={tag}
                          type="button"
                          aria-label={`Topic ${tag}`}
                          aria-pressed={selected}
                          onClick={() =>
                            setSelectedTags((current) =>
                              selected
                                ? current.filter((item) => item !== tag)
                                : [...current, tag],
                            )
                          }
                          className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-1 text-xs font-normal transition focus-visible:ring-2 focus-visible:ring-ring ${selected ? "border-sky-500/50 bg-sky-500/15 text-sky-700 dark:text-sky-200" : "border-border bg-muted/50 text-muted-foreground hover:bg-muted hover:text-foreground"}`}
                        >
                          {selected && <Check className="size-3" />}
                          {tag}
                        </button>
                      );
                    })
                  )}
                </div>
              </label>
              <Button
                type="button"
                onClick={() => setSearch(searchText)}
                className="h-10"
              >
                Apply filters
              </Button>
            </div>
          </div>
        </section>

        <div className="overflow-x-auto rounded-xl border border-border bg-card">
          <table className="w-full min-w-[720px] text-left text-sm">
            <thead className="border-b border-border bg-muted/40 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-4 py-3">Title</th>
                <th className="px-4 py-3">Type</th>
                <th className="px-4 py-3">Learning objectives</th>
                <th className="px-4 py-3">Status</th>
                <th className="px-4 py-3">Created at</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {loading && items.length === 0 ? (
                <tr>
                  <td
                    colSpan={6}
                    className="px-4 py-12 text-center text-muted-foreground"
                  >
                    <LoaderCircle className="mx-auto mb-2 size-7 animate-spin" />
                    Loading assessments...
                  </td>
                </tr>
              ) : items.length === 0 ? (
                <tr>
                  <td
                    colSpan={6}
                    className="px-4 py-12 text-center text-muted-foreground"
                  >
                    No assessments or exams found matching your criteria.
                  </td>
                </tr>
              ) : (
                items.map((item) => {
                  const isDraft = item.status === "draft";
                  const isGraded = item.mode === "graded";
                  return (
                    <tr key={item.id} className="transition hover:bg-muted/30">
                      <td className="px-4 py-3 font-medium">{item.title}</td>
                      <td className="px-4 py-3">
                        <span
                          className={`inline-flex rounded-full border px-2 py-1 text-xs font-medium ${isGraded ? "border-violet-500/30 bg-violet-500/10 text-violet-700 dark:text-violet-300" : "border-blue-500/30 bg-blue-500/10 text-blue-700 dark:text-blue-300"}`}
                        >
                          {isGraded ? "Exam" : "Practice"}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex flex-wrap gap-1">
                          {(item.tags as string[]).map((tag) => (
                            <TopicTag key={tag}>{tag}</TopicTag>
                          ))}
                        </div>
                      </td>
                      <td className="px-4 py-3">
                        <StatusTag status={item.status ?? "unknown"} />
                      </td>
                      <td className="px-4 py-3 text-muted-foreground">
                        {formatDate(item.createdAt)}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex justify-end gap-2">
                          {isDraft ? (
                            <Button asChild size="sm" variant="outline">
                              <Link href={`/author/${item.id}`}>
                                <Edit3 className="size-3.5" />
                                Edit
                              </Link>
                            </Button>
                          ) : isGraded ? (
                            <Button asChild size="sm">
                              <Link href={`/assessments/${item.id}/preview`}>
                                <ExternalLink className="size-3.5" />
                                Open
                              </Link>
                            </Button>
                          ) : (
                            <>
                              <Button asChild size="sm" variant="outline">
                                <Link href={`/assessments/${item.id}/preview`}>
                                  <Eye className="size-3.5" />
                                  Preview
                                </Link>
                              </Button>
                              <Button
                                size="sm"
                                disabled={starting === item.id}
                                onClick={() => startAssessment(item.id)}
                              >
                                <Play className="size-3.5" />
                                {starting === item.id
                                  ? "Starting…"
                                  : item.completed
                                    ? "Re-take"
                                    : "Start"}
                              </Button>
                            </>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
          {nextCursor && (
            <div className="border-t border-border p-4 text-center">
              <Button
                variant="outline"
                size="sm"
                onClick={() => fetchItems(nextCursor)}
                disabled={loading}
              >
                {loading && <LoaderCircle className="size-4 animate-spin" />}
                {loading ? "Loading more..." : "Load more"}
              </Button>
            </div>
          )}
        </div>
      </div>
    </PageShell>
  );
}
