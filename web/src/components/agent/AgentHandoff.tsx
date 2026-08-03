"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { Check, Copy, KeyRound, LoaderCircle, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/hooks/useAuth";
import { copyToClipboard } from "@/utils/clipboard";

type Delegation = {
  id: string;
  goal: string;
  expiresAt: string;
  createdAt?: string;
  revokedAt?: string | null;
};

type DelegationResponse = {
  delegations?: Delegation[];
};

type CreateDelegationResponse = {
  delegation?: Delegation;
  handoff?: string | Record<string, unknown>;
  token?: string;
  id?: string;
  goal?: string;
  expiresAt?: string;
};

const DEFAULT_EXPIRY_MINUTES = 60;

function apiUrl(path: string) {
  const base =
    process.env.NEXT_PUBLIC_API_URL ??
    (typeof window === "undefined"
      ? "http://localhost:28080"
      : `http://${window.location.hostname}:28080`);
  return `${base.replace(/\/$/, "")}${path}`;
}

function errorMessage(response: Response, fallback: string) {
  return response
    .json()
    .then((body) => body?.error?.message ?? body?.message ?? fallback)
    .catch(() => fallback);
}

function toHandoffText(response: CreateDelegationResponse) {
  if (typeof response.handoff === "string") return response.handoff;
  if (response.handoff) return JSON.stringify(response.handoff, null, 2);
  if (!response.token) return "";

  return [
    "AME delegated course-authoring handoff",
    "",
    `Authorization: Bearer ${response.token}`,
    `Goal: ${response.goal ?? response.delegation?.goal ?? "Not specified"}`,
    `Expires: ${response.expiresAt ?? response.delegation?.expiresAt ?? "See AME"}`,
    "",
    "Read /public/skill.json, then create, validate, review, and publish the course. Return the learner's AME learning URL when it is ready.",
  ].join("\n");
}

function formatExpiry(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.valueOf())
    ? value
    : date.toLocaleString(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
      });
}

export function AgentHandoff() {
  const { user, loading: authLoading } = useAuth();
  const [delegations, setDelegations] = useState<Delegation[]>([]);
  const [goal, setGoal] = useState("");
  const [expiresInMinutes, setExpiresInMinutes] = useState(
    DEFAULT_EXPIRY_MINUTES,
  );
  const [handoff, setHandoff] = useState("");
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [revokingId, setRevokingId] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const activeDelegations = delegations.filter(
    (delegation) =>
      !delegation.revokedAt &&
      new Date(delegation.expiresAt).valueOf() > Date.now(),
  );

  const loadDelegations = useCallback(async () => {
    setLoading(true);
    try {
      const response = await fetch(apiUrl("/api/v1/agent-delegations"), {
        credentials: "include",
      });
      if (!response.ok) {
        setMessage(
          await errorMessage(response, "Could not load agent access."),
        );
        return;
      }
      const body = (await response.json()) as DelegationResponse | Delegation[];
      setDelegations(Array.isArray(body) ? body : (body.delegations ?? []));
    } catch {
      setMessage("Could not reach AME to load agent access.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (user) void loadDelegations();
    else setDelegations([]);
  }, [loadDelegations, user]);

  async function createDelegation(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setCreating(true);
    setMessage(null);
    setHandoff("");
    try {
      const response = await fetch(apiUrl("/api/v1/agent-delegations"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ goal, expiresInMinutes }),
      });
      if (!response.ok) {
        setMessage(
          await errorMessage(response, "Could not create agent access."),
        );
        return;
      }
      const body = (await response.json()) as CreateDelegationResponse;
      const createdHandoff = toHandoffText(body);
      if (!createdHandoff) {
        setMessage("AME created access but did not return a handoff to copy.");
        await loadDelegations();
        return;
      }
      setHandoff(createdHandoff);
      setGoal("");
      setMessage(
        "Copy the handoff now. For safety, it will not appear in your access history.",
      );
      await loadDelegations();
    } catch {
      setMessage("Could not reach AME to create agent access.");
    } finally {
      setCreating(false);
    }
  }

  async function copyHandoff() {
    const copied = await copyToClipboard(handoff);
    setMessage(
      copied
        ? "Handoff copied. Send it directly to the agent you trust."
        : "Could not copy automatically. Select the handoff and copy it manually.",
    );
  }

  async function revokeDelegation(id: string) {
    setRevokingId(id);
    setMessage(null);
    try {
      const response = await fetch(apiUrl(`/api/v1/agent-delegations/${id}`), {
        method: "DELETE",
        credentials: "include",
      });
      if (!response.ok) {
        setMessage(
          await errorMessage(response, "Could not revoke agent access."),
        );
        return;
      }
      setDelegations((current) => current.filter((item) => item.id !== id));
      setMessage("Agent access revoked.");
    } catch {
      setMessage("Could not reach AME to revoke agent access.");
    } finally {
      setRevokingId(null);
    }
  }

  if (authLoading) {
    return <div aria-hidden className="min-h-72 animate-pulse bg-muted/40" />;
  }

  if (!user) {
    return (
      <section className="border border-border bg-card p-5 sm:p-6">
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Private handoff
        </p>
        <h2 className="mt-3 text-2xl font-semibold tracking-[-0.03em]">
          Sign in before you give an agent access.
        </h2>
        <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
          AME gives an agent a short-lived, revocable course-authoring
          capability for your learning—not your full account session.
        </p>
        <Button asChild className="mt-5 rounded-full">
          <Link href="/login?returnTo=/agent">Sign in to create a handoff</Link>
        </Button>
      </section>
    );
  }

  return (
    <section
      className="border border-border bg-card p-5 sm:p-6"
      data-testid="agent-handoff"
    >
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
            Private handoff
          </p>
          <h2 className="mt-3 text-2xl font-semibold tracking-[-0.03em]">
            Give an agent limited course-authoring access.
          </h2>
          <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
            It can prepare a source-grounded course for your goal. It cannot
            sign in as you, read your notes, or perform learning activity for
            you. You can revoke access at any time.
          </p>
        </div>
        <KeyRound className="size-5 shrink-0 text-primary" aria-hidden />
      </div>

      <form
        className="mt-6 grid gap-4 border-t border-border pt-6"
        onSubmit={createDelegation}
      >
        <label
          className="grid gap-1.5 text-sm font-medium"
          htmlFor="agent-goal"
        >
          What should the agent prepare?
          <textarea
            id="agent-goal"
            value={goal}
            onChange={(event) => setGoal(event.target.value)}
            placeholder="For example: Build a practical Flink course for debugging stateful streaming jobs."
            required
            rows={3}
            className="w-full resize-y rounded-lg border border-input bg-background px-3 py-2 font-normal outline-none transition placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/20"
          />
        </label>
        <label
          className="grid max-w-xs gap-1.5 text-sm font-medium"
          htmlFor="agent-expiry"
        >
          Access expires after
          <select
            id="agent-expiry"
            value={expiresInMinutes}
            onChange={(event) =>
              setExpiresInMinutes(Number(event.target.value))
            }
            className="h-10 rounded-lg border border-input bg-background px-3 font-normal outline-none focus:border-ring focus:ring-3 focus:ring-ring/20"
          >
            <option value={30}>30 minutes</option>
            <option value={60}>1 hour</option>
            <option value={240}>4 hours</option>
            <option value={1440}>24 hours</option>
          </select>
        </label>
        <div className="flex flex-wrap items-center gap-3">
          <Button className="rounded-full" disabled={creating} type="submit">
            {creating ? <LoaderCircle className="size-4 animate-spin" /> : null}
            Create secure handoff
          </Button>
          <p className="text-xs leading-5 text-muted-foreground">
            The secret is shown once, never in a URL, and is not saved in this
            page after you dismiss it.
          </p>
        </div>
      </form>

      {handoff && (
        <div className="mt-6 border border-primary/40 bg-primary/5 p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h3 className="font-semibold">Copy this handoff now</h3>
              <p className="mt-1 text-sm text-muted-foreground">
                Send it directly to the agent you trust. Do not put it in a
                shared document or link.
              </p>
            </div>
            <div className="flex gap-2">
              <Button
                onClick={() => void copyHandoff()}
                size="sm"
                type="button"
              >
                <Copy className="size-4" /> Copy handoff
              </Button>
              <Button
                onClick={() => setHandoff("")}
                size="sm"
                type="button"
                variant="outline"
              >
                Dismiss secret
              </Button>
            </div>
          </div>
          <textarea
            aria-label="One-time agent handoff"
            className="mt-4 min-h-40 w-full resize-y border border-border bg-background p-3 font-mono text-xs leading-5 text-foreground"
            readOnly
            spellCheck={false}
            value={handoff}
          />
        </div>
      )}

      {message && (
        <p
          aria-live="polite"
          className="mt-4 text-sm leading-6 text-muted-foreground"
        >
          {message}
        </p>
      )}

      <section
        className="mt-8 border-t border-border pt-6"
        aria-labelledby="active-agent-access"
      >
        <div className="flex items-center justify-between gap-3">
          <h3 id="active-agent-access" className="font-semibold">
            Active agent access
          </h3>
          {loading && (
            <LoaderCircle
              className="size-4 animate-spin text-muted-foreground"
              aria-label="Loading agent access"
            />
          )}
        </div>
        {!loading && activeDelegations.length === 0 && (
          <p className="mt-3 text-sm text-muted-foreground">
            No agent has access yet.
          </p>
        )}
        <ul className="mt-3 grid gap-3">
          {activeDelegations.map((delegation) => (
            <li
              key={delegation.id}
              className="flex flex-wrap items-center justify-between gap-4 border border-border bg-background p-4"
            >
              <div className="min-w-0">
                <p className="font-medium">{delegation.goal}</p>
                <p className="mt-1 text-xs text-muted-foreground">
                  Expires {formatExpiry(delegation.expiresAt)}
                </p>
              </div>
              <Button
                disabled={revokingId === delegation.id}
                onClick={() => void revokeDelegation(delegation.id)}
                size="sm"
                type="button"
                variant="outline"
              >
                {revokingId === delegation.id ? (
                  <LoaderCircle className="size-4 animate-spin" />
                ) : (
                  <Trash2 className="size-4" />
                )}
                Revoke
              </Button>
            </li>
          ))}
        </ul>
      </section>
      <p className="mt-6 flex gap-2 text-xs leading-5 text-muted-foreground">
        <Check className="mt-0.5 size-3.5 shrink-0 text-primary" aria-hidden />
        The agent contract tells the agent how to validate and publish; this
        handoff controls only whose course it may prepare.
      </p>
    </section>
  );
}
