"use client";
import { useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { PageShell } from "@/components/PageShell";
import { formatDate } from "@/utils/format";
import { Button } from "@/components/ui/button";
interface DeepDive {
  id: string;
  questionId: string;
  status: string;
  reason?: string | null;
  bodyMarkdown?: string | null;
  createdAt: string;
  updatedAt: string;
  category?: string | null;
  questionPrompt: string;
  questionKind: string;
  questionTags: string[];
  assessmentTitle?: string | null;
  course?: string | null;
}
const STATUSES = [
  "active",
  "published",
  "requested",
  "drafting",
  "needs_revision",
  "archived",
];
export default function DeepDivesPage() {
  const router = useRouter();
  const [status, setStatus] = useState("active");
  const [category, setCategory] = useState("");
  const [tag, setTag] = useState("");
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<DeepDive[]>([]);
  const [all, setAll] = useState<DeepDive[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  useEffect(() => {
    (api as any)
      .GET("/v1/deep-dives", { params: { query: {} } })
      .then(({ data }: any) => setAll(data?.deepDives ?? []))
      .catch(() => {});
  }, []);
  useEffect(() => {
    setLoading(true);
    const query: Record<string, string> = {};
    if (status !== "active") query.status = status;
    if (category) query.category = category;
    if (search) query.search = search;
    (api as any)
      .GET("/v1/deep-dives", { params: { query } })
      .then(({ data }: any) => setItems(data?.deepDives ?? []))
      .catch(() => setError("Could not load deep dives."))
      .finally(() => setLoading(false));
  }, [status, category, search]);
  const categories = useMemo(
    () =>
      [...new Set(all.flatMap((d) => (d.category ? [d.category] : [])))].sort(),
    [all],
  );
  const tags = useMemo(
    () => [...new Set(all.flatMap((d) => d.questionTags ?? []))].sort(),
    [all],
  );
  const visible = tag
    ? items.filter((d) => d.questionTags?.includes(tag))
    : items;
  const exportKb = async () => {
    setExporting(true);
    try {
      const base =
        process.env.NEXT_PUBLIC_API_URL ??
        `http://${window.location.hostname}:28080`;
      const response = await fetch(`${base}/v1/deep-dives/export`, {
        credentials: "include",
      });
      if (!response.ok) throw new Error();
      const blob = await response.blob();
      const link = document.createElement("a");
      link.href = URL.createObjectURL(blob);
      link.download = "ame-deep-dives-export.zip";
      link.click();
      URL.revokeObjectURL(link.href);
    } catch {
      setError("Failed to export Obsidian KB. Please try again.");
    } finally {
      setExporting(false);
    }
  };
  return (
    <PageShell
      kicker="Deep dives"
      title="Knowledge Base & Study Notes"
      subtitle="Track hard questions, read agent explanations, and organize your personal study space."
      maxWidth={1200}
    >
      <div className="grid gap-6 lg:grid-cols-[250px_1fr]">
        <aside className="space-y-4 rounded-lg border border-border p-5 lg:sticky lg:top-6 lg:self-start">
          <Button className="w-full" onClick={exportKb} disabled={exporting}>
            {exporting ? "Exporting…" : "Export KB (Obsidian)"}
          </Button>
          <label className="grid gap-1 text-xs text-muted-foreground">
            Search
            <input
              className="h-9 rounded border border-input bg-background px-3 text-sm"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search notes…"
            />
          </label>
          <label className="grid gap-1 text-xs text-muted-foreground">
            Status
            <select
              className="h-9 rounded border border-input bg-background px-2 text-sm"
              value={status}
              onChange={(e) => setStatus(e.target.value)}
            >
              {STATUSES.map((s) => (
                <option key={s} value={s}>
                  {s.replace("_", " ")}
                </option>
              ))}
            </select>
          </label>
          <label className="grid gap-1 text-xs text-muted-foreground">
            Category
            <select
              className="h-9 rounded border border-input bg-background px-2 text-sm"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
            >
              <option value="">All categories</option>
              {categories.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
          <label className="grid gap-1 text-xs text-muted-foreground">
            Tag
            <select
              className="h-9 rounded border border-input bg-background px-2 text-sm"
              value={tag}
              onChange={(e) => setTag(e.target.value)}
            >
              <option value="">All tags</option>
              {tags.map((t) => (
                <option key={t} value={t}>
                  {t}
                </option>
              ))}
            </select>
          </label>
        </aside>
        <section>
          {error && (
            <div
              role="alert"
              className="mb-4 rounded border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
            >
              {error}
            </div>
          )}
          <p className="mb-4 text-xs text-muted-foreground">
            {loading
              ? "Loading"
              : `${visible.length} item${visible.length === 1 ? "" : "s"}`}
          </p>
          <div className="space-y-3">
            {visible.map((d) => (
              <button
                key={d.id}
                className="block w-full rounded-lg border border-border p-5 text-left transition-colors hover:bg-muted/40"
                onClick={() => router.push(`/deep-dives/${d.id}`)}
              >
                <div className="mb-2 flex flex-wrap items-center gap-2">
                  <span className="rounded-full border px-2 py-0.5 text-xs">
                    {d.status}
                  </span>
                  {d.category && (
                    <span className="rounded-full border px-2 py-0.5 text-xs text-muted-foreground">
                      {d.category}
                    </span>
                  )}
                  <span className="ml-auto text-xs text-muted-foreground">
                    {formatDate(d.updatedAt)}
                  </span>
                </div>
                <h2 className="font-medium">{d.questionPrompt}</h2>
                <p className="mt-2 line-clamp-2 text-sm text-muted-foreground">
                  {d.bodyMarkdown || d.reason || "No explanation yet."}
                </p>
                <div className="mt-3 flex flex-wrap gap-1">
                  {d.questionTags?.map((t) => (
                    <span
                      key={t}
                      className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground"
                    >
                      {t}
                    </span>
                  ))}
                </div>
              </button>
            ))}
            {!loading && visible.length === 0 && (
              <div className="rounded-lg border border-dashed border-border p-12 text-center text-sm text-muted-foreground">
                No deep dives match these filters.
              </div>
            )}
          </div>
        </section>
      </div>
    </PageShell>
  );
}
