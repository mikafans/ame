"use client";
import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { formatDateTime } from "@/utils/format";
import { Button } from "@/components/ui/button";
interface AssessmentEntry {
  id: string;
  title: string;
  description?: string | null;
  status: string;
  mode: string;
  createdBy: string;
  createdByEmail?: string | null;
  objectives: string[];
  createdAt: string;
  deletedAt?: string | null;
}
interface PreviewQuestion {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  orderIndex: number;
  payload: unknown;
}
interface AssessmentDetail extends AssessmentEntry {
  questions: PreviewQuestion[];
}
const KIND_LABEL: Record<string, string> = {
  mc: "MC",
  tf: "T/F",
  short: "Short",
  essay: "Essay",
  code: "Code",
};
function QuestionAnswer({ kind, payload }: { kind: string; payload: unknown }) {
  const p = (payload ?? {}) as Record<string, unknown>;
  if (kind === "mc") {
    const options = Array.isArray(p.options) ? (p.options as string[]) : [];
    const correct = typeof p.correct_index === "number" ? p.correct_index : -1;
    return (
      <div className="mt-2 space-y-1">
        {options.map((option, i) => (
          <p
            key={i}
            className={`text-sm ${i === correct ? "font-semibold text-emerald-600" : "text-muted-foreground"}`}
          >
            {String.fromCharCode(65 + i)}. {option}
            {i === correct && " · CORRECT"}
          </p>
        ))}
      </div>
    );
  }
  if (kind === "tf")
    return (
      <p className="mt-2 text-sm font-semibold text-emerald-600">
        Answer:{" "}
        {p.correct === true ? "True" : p.correct === false ? "False" : "—"}
      </p>
    );
  if (kind === "short")
    return (
      <div className="mt-2 flex flex-wrap gap-1">
        {(Array.isArray(p.accepted) ? (p.accepted as string[]) : []).map(
          (a, i) => (
            <span
              key={i}
              className="rounded-full border border-emerald-500/50 px-2 py-0.5 text-xs"
            >
              {a}
            </span>
          ),
        )}
      </div>
    );
  if (kind === "code" && typeof p.exemplar === "string")
    return (
      <pre className="mt-2 overflow-auto rounded bg-muted p-3 text-xs">
        {p.exemplar}
      </pre>
    );
  if (kind === "essay")
    return (
      <p className="mt-2 text-sm italic text-muted-foreground">
        {typeof p.rubric === "string"
          ? `Rubric: ${p.rubric}`
          : "Manually graded — no fixed answer."}
      </p>
    );
  return null;
}
export default function AdminAssessmentsPage() {
  const [items, setItems] = useState<AssessmentEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const [rows, setRows] = useState(10);
  const [search, setSearch] = useState("");
  const [mode, setMode] = useState("all");
  const [status, setStatus] = useState("all");
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<AssessmentEntry | null>(null);
  const [detail, setDetail] = useState<AssessmentDetail | null>(null);
  const [dialog, setDialog] = useState<"delete" | "restore" | null>(null);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const { data, error } = await api.GET("/v1/admin/assessments", {
        params: {
          query: {
            page: page + 1,
            pageSize: rows,
            q: search || undefined,
            mode: mode === "all" ? undefined : mode,
            status: status === "all" ? undefined : status,
          },
        },
      });
      if (error)
        setError(
          "Failed to load assessments: " + errorMessage(error, "Unknown error"),
        );
      else if (data) {
        setItems(data.assessments as AssessmentEntry[]);
        setTotal(data.total);
      }
    } catch {
      setError("An unexpected error occurred while fetching assessments.");
    } finally {
      setLoading(false);
    }
  }, [page, rows, search, mode, status]);
  useEffect(() => {
    load();
  }, [load]);
  const preview = async (item: AssessmentEntry) => {
    setSelected(item);
    setDetail(null);
    try {
      const { data, error } = await api.GET("/v1/admin/assessments/{id}", {
        params: { path: { id: item.id } },
      });
      if (error) setError(errorMessage(error, "Could not load assessment."));
      else if (data) setDetail(data as AssessmentDetail);
    } catch {
      setError("Could not load assessment.");
    }
  };
  const mutate = async () => {
    if (!selected || !dialog) return;
    try {
      const result =
        dialog === "delete"
          ? await api.DELETE("/v1/admin/assessments/{id}", {
              params: { path: { id: selected.id } },
            })
          : await api.POST("/v1/admin/assessments/{id}/restore", {
              params: { path: { id: selected.id } },
            });
      if (result.error)
        setError(errorMessage(result.error, "Could not update assessment."));
      else {
        setDialog(null);
        setSelected(null);
        await load();
      }
    } catch {
      setError("Network or unexpected server error.");
    }
  };
  const pages = Math.max(1, Math.ceil(total / rows));
  return (
    <main className="px-6 py-12 sm:px-12">
      <header className="mb-8">
        <h1 className="text-3xl font-bold">Assessment Moderation</h1>
        <p className="mt-2 text-muted-foreground">
          Review, inspect, deactivate and restore platform assessments.
        </p>
      </header>
      <div className="mb-5 flex flex-wrap gap-2">
        <input
          className="h-9 min-w-64 flex-1 rounded-md border border-input bg-background px-3 text-sm"
          placeholder="Search assessments..."
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setPage(0);
          }}
        />
        <select
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={mode}
          onChange={(e) => {
            setMode(e.target.value);
            setPage(0);
          }}
        >
          <option value="all">All modes</option>
          <option value="practice">Practice</option>
          <option value="graded">Graded</option>
        </select>
        <select
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={status}
          onChange={(e) => {
            setStatus(e.target.value);
            setPage(0);
          }}
        >
          <option value="all">All statuses</option>
          <option value="draft">Draft</option>
          <option value="active">Active</option>
          <option value="archived">Archived</option>
        </select>
      </div>
      {error && (
        <div
          role="alert"
          className="mb-4 rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          {error}
        </div>
      )}
      <div className="overflow-x-auto rounded-lg border border-border">
        <table className="w-full min-w-[850px] text-sm">
          <thead className="border-b border-border bg-muted/40 text-left text-xs uppercase tracking-wide text-muted-foreground">
            <tr>
              <th className="px-4 py-3">Assessment</th>
              <th className="px-4 py-3">Mode</th>
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3">Creator</th>
              <th className="px-4 py-3">Created</th>
              <th className="px-4 py-3 text-right">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {loading ? (
              <tr>
                <td
                  colSpan={6}
                  className="px-4 py-10 text-center text-muted-foreground"
                >
                  Loading assessments…
                </td>
              </tr>
            ) : (
              items.map((item) => (
                <tr key={item.id}>
                  <td className="px-4 py-4">
                    <p className="font-medium">{item.title}</p>
                    <p className="text-xs text-muted-foreground">
                      {item.description}
                    </p>
                  </td>
                  <td className="px-4 py-4">{item.mode}</td>
                  <td className="px-4 py-4">
                    <span className="rounded-full border px-2 py-1 text-xs">
                      {item.status}
                    </span>
                  </td>
                  <td className="px-4 py-4 text-muted-foreground">
                    {item.createdByEmail || item.createdBy}
                  </td>
                  <td className="px-4 py-4 text-xs text-muted-foreground">
                    {formatDateTime(item.createdAt)}
                  </td>
                  <td className="px-4 py-4 text-right">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => preview(item)}
                    >
                      Preview
                    </Button>
                    {item.status === "archived" || item.deletedAt ? (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => {
                          setSelected(item);
                          setDialog("restore");
                        }}
                      >
                        Restore
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="destructive"
                        onClick={() => {
                          setSelected(item);
                          setDialog("delete");
                        }}
                      >
                        Deactivate
                      </Button>
                    )}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      <div className="flex justify-end gap-3 py-4 text-sm text-muted-foreground">
        <select
          className="rounded border border-input bg-background px-2 py-1"
          value={rows}
          onChange={(e) => {
            setRows(Number(e.target.value));
            setPage(0);
          }}
        >
          <option value={10}>10 / page</option>
          <option value={25}>25 / page</option>
          <option value={50}>50 / page</option>
        </select>
        <span>
          {total ? page * rows + 1 : 0}–{Math.min((page + 1) * rows, total)} of{" "}
          {total}
        </span>
        <Button
          size="sm"
          variant="ghost"
          disabled={!page}
          onClick={() => setPage(page - 1)}
        >
          Previous
        </Button>
        <Button
          size="sm"
          variant="ghost"
          disabled={page + 1 >= pages}
          onClick={() => setPage(page + 1)}
        >
          Next
        </Button>
      </div>
      {selected && (detail || dialog) && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="max-h-[85vh] w-full max-w-3xl overflow-auto rounded-xl border border-border bg-background p-6 shadow-xl">
            {dialog ? (
              <>
                <h2 className="text-lg font-semibold">
                  {dialog === "delete"
                    ? "Deactivate assessment?"
                    : "Restore assessment?"}
                </h2>
                <p className="mt-3 text-sm text-muted-foreground">
                  {selected.title}
                </p>
                <div className="mt-6 flex justify-end gap-2">
                  <Button variant="ghost" onClick={() => setDialog(null)}>
                    Cancel
                  </Button>
                  <Button
                    variant={dialog === "delete" ? "destructive" : "default"}
                    onClick={mutate}
                  >
                    {dialog === "delete" ? "Deactivate" : "Restore"}
                  </Button>
                </div>
              </>
            ) : (
              <>
                <div className="flex justify-between">
                  <h2 className="text-lg font-semibold">{detail?.title}</h2>
                  <Button
                    variant="ghost"
                    onClick={() => {
                      setSelected(null);
                      setDetail(null);
                    }}
                  >
                    Close
                  </Button>
                </div>
                {detail?.objectives?.length ? (
                  <ul className="my-4 list-disc pl-5 text-sm text-muted-foreground">
                    {detail.objectives.map((objective, i) => (
                      <li key={i}>{objective}</li>
                    ))}
                  </ul>
                ) : null}
                <div className="space-y-4">
                  {detail?.questions?.map((question, i) => (
                    <article
                      key={question.id}
                      className="rounded border border-border p-4"
                    >
                      <p className="text-xs uppercase text-muted-foreground">
                        {i + 1} · {KIND_LABEL[question.kind] || question.kind} ·{" "}
                        {question.points} pts
                      </p>
                      <p className="mt-2">{question.prompt}</p>
                      <QuestionAnswer
                        kind={question.kind}
                        payload={question.payload}
                      />
                    </article>
                  ))}
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </main>
  );
}
