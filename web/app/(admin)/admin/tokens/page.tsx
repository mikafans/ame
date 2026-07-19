"use client";
import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { formatDateTime } from "@/utils/format";
import { Button } from "@/components/ui/button";
interface TokenEntry {
  id: string;
  name: string;
  ownerId: string;
  ownerEmail: string | null;
  ownerDisplayName?: string | null;
  ownerRole: string;
  status: string;
  scopes: string[];
  lastUsedAt: string | null;
  revokedAt: string | null;
  expiresAt: string;
  createdAt: string;
}
export default function TokensAuditPage() {
  const [tokens, setTokens] = useState<TokenEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const [rows, setRows] = useState(10);
  const [q, setQ] = useState("");
  const [status, setStatus] = useState("");
  const [role, setRole] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<TokenEntry | null>(null);
  const [dialog, setDialog] = useState<"revoke" | "scopes" | null>(null);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const { data, error } = await api.GET("/v1/admin/tokens", {
        params: {
          query: {
            page: page + 1,
            pageSize: rows,
            q: q || undefined,
            status: status || undefined,
            role: role || undefined,
          },
        },
      });
      if (error)
        setError(
          "Failed to load tokens: " + errorMessage(error, "Unknown error"),
        );
      else if (data) {
        setTokens(data.tokens as TokenEntry[]);
        setTotal(data.total);
      }
    } catch {
      setError("An unexpected error occurred while fetching tokens.");
    } finally {
      setLoading(false);
    }
  }, [page, rows, q, status, role]);
  useEffect(() => {
    load();
  }, [load]);
  const revoke = async () => {
    if (!selected) return;
    try {
      const { error } = await api.DELETE("/v1/admin/tokens/{id}", {
        params: { path: { id: selected.id } },
      });
      if (error) setError(errorMessage(error, "Could not revoke token."));
      else {
        setDialog(null);
        load();
      }
    } catch {
      setError("Network or unexpected server error.");
    }
  };
  const pages = Math.max(1, Math.ceil(total / rows));
  return (
    <main className="px-6 py-12 sm:px-12">
      <header className="mb-8">
        <h1 className="text-3xl font-bold">API Tokens</h1>
        <p className="mt-2 text-muted-foreground">
          Audit platform API tokens, scopes, usage and expiration.
        </p>
      </header>
      <div className="mb-5 flex flex-wrap gap-2">
        <input
          className="h-9 min-w-64 flex-1 rounded-md border border-input bg-background px-3 text-sm"
          placeholder="Search tokens..."
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setPage(0);
          }}
        />
        <select
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={status}
          onChange={(e) => {
            setStatus(e.target.value);
            setPage(0);
          }}
        >
          <option value="">All statuses</option>
          <option value="active">Active</option>
          <option value="revoked">Revoked</option>
          <option value="expired">Expired</option>
        </select>
        <select
          className="h-9 rounded-md border border-input bg-background px-3 text-sm"
          value={role}
          onChange={(e) => {
            setRole(e.target.value);
            setPage(0);
          }}
        >
          <option value="">All roles</option>
          <option value="admin">Admin</option>
          <option value="agent">Agent</option>
          <option value="user">User</option>
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
        <table className="w-full min-w-[900px] text-sm">
          <thead className="border-b border-border bg-muted/40 text-left text-xs uppercase tracking-wide text-muted-foreground">
            <tr>
              <th className="px-4 py-3">Token</th>
              <th className="px-4 py-3">Owner</th>
              <th className="px-4 py-3">Scopes</th>
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3">Expires</th>
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
                  Loading tokens…
                </td>
              </tr>
            ) : (
              tokens.map((token) => (
                <tr key={token.id}>
                  <td className="px-4 py-4">
                    <p className="font-medium">{token.name}</p>
                    <p className="font-mono text-xs text-muted-foreground">
                      {token.id}
                    </p>
                  </td>
                  <td className="px-4 py-4">
                    <p>
                      {token.ownerDisplayName ||
                        token.ownerEmail ||
                        token.ownerId}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {token.ownerRole}
                    </p>
                  </td>
                  <td className="max-w-64 px-4 py-4">
                    <div className="flex flex-wrap gap-1">
                      {token.scopes.map((scope) => (
                        <span
                          key={scope}
                          className="rounded-full border px-2 py-0.5 text-xs"
                        >
                          {scope}
                        </span>
                      ))}
                    </div>
                  </td>
                  <td className="px-4 py-4">
                    <span
                      className={`rounded-full border px-2 py-1 text-xs ${token.status === "active" ? "border-emerald-500/50 text-emerald-600" : "text-muted-foreground"}`}
                    >
                      {token.status}
                    </span>
                  </td>
                  <td className="px-4 py-4 text-xs text-muted-foreground">
                    {formatDateTime(token.expiresAt)}
                  </td>
                  <td className="px-4 py-4 text-right">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => {
                        setSelected(token);
                        setDialog("scopes");
                      }}
                    >
                      Scopes
                    </Button>
                    {token.status === "active" && (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => {
                          setSelected(token);
                          setDialog("revoke");
                        }}
                      >
                        Revoke
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
          disabled={page === 0}
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
      {dialog && selected && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <div className="w-full max-w-md rounded-xl border border-border bg-background p-6 shadow-xl">
            <h2 className="text-lg font-semibold">
              {dialog === "revoke" ? "Revoke token?" : "Token scopes"}
            </h2>
            {dialog === "revoke" ? (
              <p className="mt-3 text-sm text-muted-foreground">
                This will immediately invalidate{" "}
                <strong>{selected.name}</strong>.
              </p>
            ) : (
              <div className="mt-4 flex flex-wrap gap-2">
                {selected.scopes.map((scope) => (
                  <span
                    key={scope}
                    className="rounded-full border px-2 py-1 text-xs"
                  >
                    {scope}
                  </span>
                ))}
              </div>
            )}
            <div className="mt-6 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setDialog(null)}>
                Close
              </Button>
              {dialog === "revoke" && (
                <Button variant="destructive" onClick={revoke}>
                  Revoke
                </Button>
              )}
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
