"use client";
import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { formatDateTime } from "@/utils/format";
import { Button } from "@/components/ui/button";
interface AuditLogEntry {
  id: string;
  actorUserId?: string | null;
  actorEmail?: string | null;
  actorName?: string | null;
  action: string;
  targetType?: string | null;
  targetId?: string | null;
  targetEmail?: string | null;
  targetName?: string | null;
  metadata: any;
  createdAt: string;
}
export default function AuditLogsPage() {
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const [rows, setRows] = useState(10);
  const [filters, setFilters] = useState({
    action: "",
    actorId: "",
    targetId: "",
  });
  const [inputs, setInputs] = useState(filters);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<AuditLogEntry | null>(null);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const { data, error } = await api.GET("/v1/admin/audit", {
        params: {
          query: {
            limit: rows,
            offset: page * rows,
            action: filters.action || undefined,
            actorId: filters.actorId || undefined,
            targetId: filters.targetId || undefined,
          },
        },
      });
      if (error)
        setError(
          "Failed to load audit logs: " + errorMessage(error, "Unknown error"),
        );
      else if (data) {
        setLogs(data.logs as AuditLogEntry[]);
        setTotal(data.total);
      }
    } catch {
      setError("An unexpected error occurred while fetching audit logs.");
    } finally {
      setLoading(false);
    }
  }, [page, rows, filters]);
  useEffect(() => {
    load();
  }, [load]);
  const label = (value: string) =>
    value
      .split(/[._]/)
      .map((word) => word[0]?.toUpperCase() + word.slice(1))
      .join(" ");
  const tone = (value: string) =>
    value.includes("fail") ||
    value.includes("disable") ||
    value.includes("reject")
      ? "text-red-600"
      : value.includes("create") || value.includes("enable")
        ? "text-emerald-600"
        : value.includes("update") || value.includes("patch")
          ? "text-amber-600"
          : "text-muted-foreground";
  const pages = Math.max(1, Math.ceil(total / rows));
  return (
    <main className="px-6 py-12 sm:px-12">
      <header className="mb-8">
        <h1 className="text-3xl font-bold">Audit Logs</h1>
        <p className="mt-2 text-muted-foreground">
          Track and inspect write actions and security events on the platform.
        </p>
      </header>
      <form
        className="mb-5 grid gap-3 rounded-lg border border-border p-5 sm:grid-cols-4"
        onSubmit={(e) => {
          e.preventDefault();
          setPage(0);
          setFilters(inputs);
        }}
      >
        <input
          className="h-9 rounded border border-input bg-background px-3 text-sm"
          placeholder="Action"
          value={inputs.action}
          onChange={(e) => setInputs({ ...inputs, action: e.target.value })}
        />
        <input
          className="h-9 rounded border border-input bg-background px-3 text-sm"
          placeholder="Actor User ID"
          value={inputs.actorId}
          onChange={(e) => setInputs({ ...inputs, actorId: e.target.value })}
        />
        <input
          className="h-9 rounded border border-input bg-background px-3 text-sm"
          placeholder="Target ID"
          value={inputs.targetId}
          onChange={(e) => setInputs({ ...inputs, targetId: e.target.value })}
        />
        <div className="flex gap-2">
          <Button type="submit">Filter</Button>
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              setInputs({ action: "", actorId: "", targetId: "" });
              setFilters({ action: "", actorId: "", targetId: "" });
              setPage(0);
            }}
          >
            Clear
          </Button>
        </div>
      </form>
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
              <th className="px-4 py-3">Action</th>
              <th className="px-4 py-3">Actor</th>
              <th className="px-4 py-3">Target</th>
              <th className="px-4 py-3">Created</th>
              <th className="px-4 py-3 text-right">Details</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {loading ? (
              <tr>
                <td
                  colSpan={5}
                  className="px-4 py-10 text-center text-muted-foreground"
                >
                  Loading audit logs…
                </td>
              </tr>
            ) : (
              logs.map((log) => (
                <tr key={log.id}>
                  <td className={`px-4 py-4 font-medium ${tone(log.action)}`}>
                    {label(log.action)}
                  </td>
                  <td className="px-4 py-4">
                    {log.actorName ||
                      log.actorEmail ||
                      log.actorUserId ||
                      "System"}
                  </td>
                  <td className="px-4 py-4 text-muted-foreground">
                    {log.targetName || log.targetEmail || log.targetId || "—"}
                  </td>
                  <td className="px-4 py-4 text-xs text-muted-foreground">
                    {formatDateTime(log.createdAt)}
                  </td>
                  <td className="px-4 py-4 text-right">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setSelected(log)}
                    >
                      View
                    </Button>
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
      {selected && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="max-h-[80vh] w-full max-w-2xl overflow-auto rounded-xl border border-border bg-background p-6 shadow-xl">
            <h2 className="text-lg font-semibold">{label(selected.action)}</h2>
            <pre className="mt-4 overflow-auto rounded-md bg-muted p-4 text-xs">
              {JSON.stringify(selected.metadata, null, 2)}
            </pre>
            <div className="mt-6 flex justify-end">
              <Button variant="outline" onClick={() => setSelected(null)}>
                Close
              </Button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
