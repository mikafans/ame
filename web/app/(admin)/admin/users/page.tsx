"use client";
import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { formatDateTime } from "@/utils/format";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "@/components/ui/button";
interface UserType {
  id: string;
  email?: string | null;
  displayName: string;
  role: "admin" | "learner";
  plan: "free" | "premium";
  status: "active" | "deactivated";
  createdAt: string;
}
export default function ManageUsersPage() {
  const { user: currentUser } = useAuth();
  const [users, setUsers] = useState<UserType[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const [rows, setRows] = useState(10);
  const [query, setQuery] = useState("");
  const [input, setInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<UserType | null>(null);
  const [dialog, setDialog] = useState<"role" | "plan" | "status" | null>(null);
  const [targetRole, setTargetRole] = useState("learner");
  const [targetPlan, setTargetPlan] = useState<"free" | "premium">("free");
  const [confirm, setConfirm] = useState(false);
  const [saving, setSaving] = useState(false);
  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const { data, error } = await api.GET("/api/v1/admin/users", {
        params: {
          query: { limit: rows, offset: page * rows, q: query || undefined },
        },
      });
      if (error)
        setError(
          "Failed to load users: " + errorMessage(error, "Unknown error"),
        );
      else if (data) {
        setUsers(data.users as UserType[]);
        setTotal(data.total);
      }
    } catch {
      setError("An unexpected error occurred while fetching users.");
    } finally {
      setLoading(false);
    }
  }, [page, rows, query]);
  useEffect(() => {
    load();
  }, [load]);
  const open = (u: UserType, kind: "role" | "plan" | "status") => {
    setSelected(u);
    setDialog(kind);
    setTargetRole(u.role);
    setTargetPlan(u.plan);
    setConfirm(false);
  };
  const save = async () => {
    if (!selected || !dialog) return;
    if (dialog === "status" && selected.status === "active" && !confirm) return;
    setSaving(true);
    try {
      const body =
        dialog === "role"
          ? { role: targetRole }
          : dialog === "plan"
            ? { plan: targetPlan }
            : {
                status: selected.status === "active" ? "deactivated" : "active",
              };
      const { error } = await api.PATCH("/api/v1/admin/users/{id}", {
        params: { path: { id: selected.id } },
        body,
      });
      if (error) setError(errorMessage(error, "Could not update user."));
      else {
        setDialog(null);
        await load();
      }
    } catch {
      setError("Network or unexpected server error.");
    } finally {
      setSaving(false);
    }
  };
  const pages = Math.max(1, Math.ceil(total / rows));
  return (
    <main className="px-6 py-12 sm:px-12">
      <header className="mb-8">
        <h1 className="text-3xl font-bold">Manage Users</h1>
        <p className="mt-2 text-muted-foreground">
          Paginate, search, and update roles, VIP plans, or account status.
        </p>
      </header>
      <form
        className="mb-5 flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          setPage(0);
          setQuery(input);
        }}
      >
        <input
          className="h-9 max-w-md flex-1 rounded-md border border-input bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-ring"
          placeholder="Search by display name or email..."
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <Button type="submit">Search</Button>
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
        <table className="w-full min-w-[760px] text-sm">
          <thead className="border-b border-border bg-muted/40 text-left text-xs uppercase tracking-wide text-muted-foreground">
            <tr>
              <th className="px-4 py-3">User</th>
              <th className="px-4 py-3">Plan</th>
              <th className="px-4 py-3">Role</th>
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3">Joined</th>
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
                  Loading user accounts…
                </td>
              </tr>
            ) : users.length === 0 ? (
              <tr>
                <td
                  colSpan={6}
                  className="px-4 py-10 text-center text-muted-foreground"
                >
                  No users found.
                </td>
              </tr>
            ) : (
              users.map((u) => (
                <tr
                  key={u.id}
                  className={u.status === "deactivated" ? "opacity-60" : ""}
                >
                  <td className="px-4 py-4">
                    <div className="flex items-center gap-3">
                      <span className="flex size-9 items-center justify-center rounded-full border border-border bg-muted font-semibold">
                        {u.displayName
                          .split(" ")
                          .map((n) => n[0])
                          .join("")
                          .slice(0, 2)
                          .toUpperCase()}
                      </span>
                      <div>
                        <p className="font-medium">
                          {u.displayName}
                          {currentUser?.id === u.id && (
                            <span className="ml-1 text-xs text-primary">
                              (You)
                            </span>
                          )}
                        </p>
                        <p className="text-xs text-muted-foreground">
                          {u.email}
                        </p>
                      </div>
                    </div>
                  </td>
                  <td className="px-4 py-4">
                    <span className="rounded-full border px-2 py-1 text-xs uppercase">
                      {u.plan === "premium" ? "VIP / Premium" : "Free"}
                    </span>
                  </td>
                  <td className="px-4 py-4">
                    <span className="rounded-full border px-2 py-1 text-xs uppercase">
                      {u.role}
                    </span>
                  </td>
                  <td className="px-4 py-4">
                    {u.status === "deactivated" ? (
                      <span className="text-red-500">Disabled</span>
                    ) : (
                      <span className="text-emerald-600">Active</span>
                    )}
                  </td>
                  <td className="px-4 py-4 text-xs text-muted-foreground">
                    {formatDateTime(u.createdAt)}
                  </td>
                  <td className="px-4 py-4 text-right">
                    <div className="flex justify-end gap-1">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => open(u, "role")}
                      >
                        Role
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => open(u, "plan")}
                      >
                        Plan
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => open(u, "status")}
                      >
                        {u.status === "deactivated" ? "Enable" : "Disable"}
                      </Button>
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      <div className="flex items-center justify-end gap-3 py-4 text-sm text-muted-foreground">
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
        <span>
          {page + 1} / {pages}
        </span>
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
          <div className="w-full max-w-sm rounded-xl border border-border bg-background p-6 shadow-xl">
            <h2 className="text-lg font-semibold">
              {dialog === "role"
                ? "Change role"
                : dialog === "plan"
                  ? "Change service plan"
                  : selected.status === "deactivated"
                    ? "Enable user"
                    : "Disable user"}
            </h2>
            {dialog === "role" && (
              <select
                className="mt-5 h-9 w-full rounded border border-input bg-background px-3"
                value={targetRole}
                onChange={(e) => setTargetRole(e.target.value)}
              >
                <option value="learner">Learner</option>
                <option value="admin">Admin</option>
              </select>
            )}
            {dialog === "plan" && (
              <select
                className="mt-5 h-9 w-full rounded border border-input bg-background px-3"
                value={targetPlan}
                onChange={(e) =>
                  setTargetPlan(e.target.value as "free" | "premium")
                }
              >
                <option value="free">Free</option>
                <option value="premium">VIP / Premium</option>
              </select>
            )}
            {dialog === "status" && selected.status === "active" && (
              <label className="mt-5 flex gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={confirm}
                  onChange={(e) => setConfirm(e.target.checked)}
                />
                I understand this disables the account.
              </label>
            )}
            <div className="mt-6 flex justify-end gap-2">
              <Button variant="ghost" onClick={() => setDialog(null)}>
                Cancel
              </Button>
              <Button
                onClick={save}
                disabled={
                  saving ||
                  (dialog === "status" &&
                    selected.status === "active" &&
                    !confirm)
                }
              >
                {saving ? "Saving…" : "Save"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}
