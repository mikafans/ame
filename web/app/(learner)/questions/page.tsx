"use client";
import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { useColorMode } from "@/components/ThemeRegistry";
import { PageShell } from "@/components/PageShell";
import { HighlightedCode } from "@/components/HighlightedCode";
import { Button } from "@/components/ui/button";
import { CheckCircle2, Search, X } from "lucide-react";
interface Question {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  status: string;
  tags: string[];
}
interface QuestionDetail extends Question {
  payload: Record<string, unknown>;
  explanation: string | null;
}
interface Tag {
  id: string;
  name: string;
}
const KIND_LABELS: Record<string, string> = {
  all: "All",
  mc: "MC",
  tf: "T/F",
  short: "Short",
  essay: "Essay",
  code: "Code",
};
export default function QuestionsPage() {
  const { mode } = useColorMode();
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [kind, setKind] = useState("all");
  const [tagId, setTagId] = useState<string | null>(null);
  const [status, setStatus] = useState("all");
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(25);
  const [loading, setLoading] = useState(false);
  const [questions, setQuestions] = useState<Question[]>([]);
  const [total, setTotal] = useState(0);
  const [tags, setTags] = useState<Tag[]>([]);
  const [preview, setPreview] = useState<Question | null>(null);
  const [detail, setDetail] = useState<QuestionDetail | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [requests, setRequests] = useState<
    Record<string, "saving" | "requested">
  >({});
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      setPage(1);
    }, 300);
    return () => clearTimeout(timer);
  }, [search]);
  useEffect(() => {
    (api as any)
      .GET("/v1/tags")
      .then(({ data }: { data?: Tag[] }) =>
        setTags(
          Array.isArray(data)
            ? [...data].sort((a, b) => a.name.localeCompare(b.name))
            : [],
        ),
      )
      .catch(() => {});
  }, []);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const query: Record<string, unknown> = { page, pageSize };
      if (debouncedSearch) query.search = debouncedSearch;
      if (kind !== "all") query.kind = kind;
      if (tagId) query.tag = tags.find((t) => t.id === tagId)?.name;
      if (status !== "all") query.status = status;
      const { data } = await (api as any).GET("/v1/questions", {
        params: { query },
      });
      setQuestions(data?.questions ?? []);
      setTotal(data?.total ?? 0);
    } catch {
      setQuestions([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [page, pageSize, debouncedSearch, kind, tagId, status, tags]);
  useEffect(() => {
    load();
  }, [load]);
  const open = async (q: Question) => {
    setPreview(q);
    setDetail(null);
    setPreviewLoading(true);
    try {
      const { data } = await (api as any).GET("/v1/questions/{id}", {
        params: { path: { id: q.id } },
      });
      if (data) setDetail({ ...q, ...data });
    } finally {
      setPreviewLoading(false);
    }
  };
  const request = async (id: string) => {
    if (requests[id] === "saving") return;
    setRequests((p) => ({ ...p, [id]: "saving" }));
    try {
      await (api as any).POST("/v1/deep-dives", {
        body: { questionId: id, reason: "Marked from question bank" },
      });
      setRequests((p) => ({ ...p, [id]: "requested" }));
    } catch {
      setRequests((p) => {
        const n = { ...p };
        delete n[id];
        return n;
      });
    }
  };
  const pages = Math.max(1, Math.ceil(total / pageSize));
  return (
    <PageShell
      kicker="Library"
      title="Question Bank"
      subtitle="Browse, search, and review all questions in your organization"
    >
      <div className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-center">
        <label className="relative flex-1">
          <Search
            size={16}
            className="absolute left-3 top-2.5 text-muted-foreground"
          />
          <input
            className="h-9 w-full rounded-md border border-input bg-background pl-9 pr-3 text-sm outline-none focus:ring-2 focus:ring-ring"
            placeholder="Search questions..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
        <div className="flex overflow-x-auto rounded-md border border-input">
          {Object.entries(KIND_LABELS).map(([value, label]) => (
            <button
              key={value}
              className={`whitespace-nowrap px-3 py-2 text-xs ${kind === value ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted"}`}
              onClick={() => {
                setKind(value);
                setPage(1);
              }}
            >
              {label}
            </button>
          ))}
        </div>
        <select
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={tagId ?? ""}
          onChange={(e) => {
            setTagId(e.target.value || null);
            setPage(1);
          }}
        >
          <option value="">All tags</option>
          {tags.map((tag) => (
            <option key={tag.id} value={tag.id}>
              {tag.name}
            </option>
          ))}
        </select>
        <select
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={status}
          onChange={(e) => {
            setStatus(e.target.value);
            setPage(1);
          }}
        >
          <option value="all">All status</option>
          <option value="draft">Draft</option>
          <option value="live">Live</option>
          <option value="archived">Archived</option>
        </select>
      </div>
      <div className="overflow-x-auto rounded-lg border border-border">
        <table className="w-full min-w-[600px] text-sm">
          <thead className="border-b border-border bg-muted/40 text-left text-xs uppercase tracking-wide text-muted-foreground">
            <tr>
              <th className="w-20 px-4 py-3">Kind</th>
              <th className="px-4 py-3">Prompt</th>
              <th className="w-52 px-4 py-3">Tags</th>
              <th className="w-16 px-4 py-3 text-right">Pts</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <tr key={i}>
                  <td className="px-4 py-4" colSpan={4}>
                    <div className="h-4 animate-pulse rounded bg-muted" />
                  </td>
                </tr>
              ))
            ) : questions.length === 0 ? (
              <tr>
                <td
                  colSpan={4}
                  className="px-4 py-10 text-center text-muted-foreground"
                >
                  No questions found.
                </td>
              </tr>
            ) : (
              questions.map((q) => (
                <tr
                  key={q.id}
                  className="cursor-pointer hover:bg-muted/40"
                  onClick={() => open(q)}
                >
                  <td className="px-4 py-4">
                    <span className="rounded-full border px-2 py-1 text-xs">
                      {KIND_LABELS[q.kind] || q.kind}
                    </span>
                  </td>
                  <td className="max-w-[420px] px-4 py-4">
                    <span className="line-clamp-2">{q.prompt}</span>
                  </td>
                  <td className="px-4 py-4">
                    <div className="flex flex-wrap gap-1">
                      {q.tags.slice(0, 3).map((tag) => (
                        <span
                          key={tag}
                          className="rounded-full border px-2 py-0.5 text-xs text-muted-foreground"
                        >
                          {tag}
                        </span>
                      ))}
                      {q.tags.length > 3 && (
                        <span className="rounded-full border px-2 py-0.5 text-xs">
                          +{q.tags.length - 3}
                        </span>
                      )}
                    </div>
                  </td>
                  <td className="px-4 py-4 text-right">{q.points}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 py-4 text-sm text-muted-foreground">
        <span>
          {total === 0
            ? "—"
            : `${(page - 1) * pageSize + 1}–${Math.min(page * pageSize, total)} of ${total}`}
        </span>
        <div className="flex items-center gap-2">
          <select
            className="rounded border border-input bg-background px-2 py-1 text-xs"
            value={pageSize}
            onChange={(e) => {
              setPageSize(Number(e.target.value));
              setPage(1);
            }}
          >
            <option value={25}>25 / page</option>
            <option value={50}>50 / page</option>
            <option value={100}>100 / page</option>
          </select>
          <Button
            size="sm"
            variant="outline"
            disabled={page <= 1}
            onClick={() => setPage(page - 1)}
          >
            Previous
          </Button>
          <span>
            {page} / {pages}
          </span>
          <Button
            size="sm"
            variant="outline"
            disabled={page >= pages}
            onClick={() => setPage(page + 1)}
          >
            Next
          </Button>
        </div>
      </div>
      {preview && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
          role="dialog"
          aria-modal="true"
        >
          <div className="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-xl border border-border bg-background shadow-xl">
            <header className="flex items-center gap-3 border-b border-border px-6 py-4">
              <span className="rounded-full border px-2 py-1 text-xs">
                {KIND_LABELS[preview.kind] || preview.kind}
              </span>
              <span className="text-sm text-muted-foreground">
                {preview.points} pt{preview.points === 1 ? "" : "s"} ·{" "}
                {preview.status}
              </span>
              <button
                className="ml-auto text-muted-foreground hover:text-foreground"
                onClick={() => {
                  setPreview(null);
                  setDetail(null);
                }}
              >
                <X size={18} />
              </button>
            </header>
            <div className="space-y-5 p-6">
              {previewLoading || !detail ? (
                <div className="flex justify-center py-10">
                  <span className="size-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
                </div>
              ) : (
                <>
                  <p className="whitespace-pre-wrap">{detail.prompt}</p>
                  <AnswerBlock detail={detail} />
                  {detail.explanation && (
                    <div className="border-t border-border pt-4">
                      <p className="mb-1 text-xs uppercase tracking-wide text-muted-foreground">
                        Explanation
                      </p>
                      <p className="whitespace-pre-wrap text-sm">
                        {detail.explanation}
                      </p>
                    </div>
                  )}
                  <div className="flex flex-wrap gap-1">
                    {preview.tags.map((tag) => (
                      <span
                        key={tag}
                        className="rounded-full border px-2 py-0.5 text-xs text-muted-foreground"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                  <div className="flex justify-end">
                    <Button
                      size="sm"
                      variant={
                        requests[preview.id] === "requested"
                          ? "outline"
                          : "default"
                      }
                      disabled={requests[preview.id] === "saving"}
                      onClick={() => request(preview.id)}
                    >
                      {requests[preview.id] === "saving"
                        ? "Requesting"
                        : requests[preview.id] === "requested"
                          ? "Deep dive requested"
                          : "Request deep dive"}
                    </Button>
                  </div>
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </PageShell>
  );
}
const MCQ_LABELS = ["A", "B", "C", "D", "E", "F"];
function AnswerBlock({ detail }: { detail: QuestionDetail }) {
  const p = detail.payload;
  if (detail.kind === "mc") {
    const options = (p.options as string[]) ?? [];
    const correct = p.correct_index as number;
    return (
      <div className="space-y-2">
        {options.map((opt, i) => (
          <div
            key={i}
            className={`flex items-center gap-3 rounded-md border px-3 py-2 ${i === correct ? "border-emerald-500 bg-emerald-500/10" : "border-border"}`}
          >
            <b className="text-muted-foreground">{MCQ_LABELS[i] ?? i + 1}</b>
            <span className="flex-1">{opt}</span>
            {i === correct && (
              <CheckCircle2 size={16} className="text-emerald-500" />
            )}
          </div>
        ))}
      </div>
    );
  }
  if (detail.kind === "tf") {
    const correct = p.correct as boolean;
    return (
      <div className="flex gap-2">
        {[true, false].map((v) => (
          <span
            key={String(v)}
            className={`rounded-full border px-3 py-1 text-sm ${v === correct ? "border-emerald-500 bg-emerald-500/10 text-emerald-600" : "border-border"}`}
          >
            {v ? "True" : "False"}
          </span>
        ))}
      </div>
    );
  }
  if (detail.kind === "short")
    return (
      <div>
        <p className="mb-2 text-xs uppercase tracking-wide text-muted-foreground">
          Accepted answers
        </p>
        <div className="flex flex-wrap gap-1">
          {((p.accepted as string[]) ?? []).map((a, i) => (
            <span
              key={i}
              className="rounded-full border border-emerald-500/50 px-2 py-1 text-xs"
            >
              {a}
            </span>
          ))}
        </div>
      </div>
    );
  if (detail.kind === "essay")
    return (
      <p className="text-sm text-muted-foreground">
        Open response{p.min_words ? ` · min ${p.min_words} words` : ""}
        {p.rubric ? ` · ${p.rubric}` : ""}
      </p>
    );
  if (detail.kind === "code")
    return (
      <div>
        <p className="mb-2 text-xs uppercase tracking-wide text-muted-foreground">
          Code · {String(p.language ?? "python")} ·{" "}
          {((p.tests as unknown[]) ?? []).length} tests
        </p>
        {typeof p.starter === "string" && p.starter.length > 0 && (
          <HighlightedCode
            code={String(p.starter)}
            language={String(p.language ?? "python")}
          />
        )}
      </div>
    );
  return null;
}
