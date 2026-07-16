"use client";

import { useEffect, useState } from "react";
import {
  Copy,
  Download,
  ExternalLink,
  KeyRound,
  Plus,
  RotateCcw,
  Trash2,
} from "lucide-react";
import { PageShell } from "@/components/PageShell";
import { HighlightedCode } from "@/components/HighlightedCode";
import { Button } from "@/components/ui/button";
import { api } from "@/api/client";
import { copyToClipboard } from "@/utils/clipboard";
import { formatTime } from "@/utils/format";
import { CLIENT_PY } from "@/generated/clientSource";

type Token = {
  id: string;
  name: string;
  scopes: string[];
  createdAt: string;
  lastUsedAt?: string | null;
};
type Agent = {
  id: string;
  label: string;
  scopes?: string[];
  focusTags?: string[];
  tokens?: Token[];
  createdAt: string;
  currentGoal?: string | null;
  nextTarget?: string | null;
};
type Tool = { name: string; description: string; inputSchema?: unknown };
type Activity = {
  id: string;
  toolName: string;
  agentId?: string;
  agentName?: string;
  status: number;
  ts: string;
};
const scopes = [
  { value: "assessment.read", label: "Read assessments and questions" },
  { value: "assessment.write", label: "Create and update assessments" },
  { value: "attempt.read", label: "Read attempt details" },
  { value: "stats.read", label: "Read learning metrics" },
];

export default function AgentPage() {
  const [tab, setTab] = useState("keys");
  return (
    <PageShell
      kicker="Programmatic surface · OpenAPI 3.1 · MCP-compatible"
      title="Agent integration"
      subtitle="Give agents a narrow, auditable interface to the same assessment model you use in the app."
      action={
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            onClick={() => window.open("/llms.txt", "_blank")}
          >
            llms.txt
          </Button>
          <Button
            variant="outline"
            onClick={() => window.open("/openapi.yaml", "_blank")}
          >
            OpenAPI
          </Button>
          <Button
            variant="outline"
            onClick={() => window.open("/skill.json", "_blank")}
          >
            MCP manifest
          </Button>
        </div>
      }
    >
      <section className="mb-7 grid gap-5 rounded-xl border border-border bg-card p-6 lg:grid-cols-[1.2fr_1fr]">
        <div>
          <span className="inline-flex rounded-full border border-primary/30 bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary">
            Two interfaces, one model
          </span>
          <h2 className="mt-4 max-w-xl text-xl font-semibold tracking-tight">
            Assessments, attempts, and rubrics are first-class API objects.
          </h2>
          <p className="mt-3 max-w-xl text-sm leading-6 text-muted-foreground">
            Every screen is backed by the same REST surface that agents use. A
            grading agent reads an attempt with one call; an authoring agent
            imports an assessment with another.
          </p>
        </div>
        <div className="rounded-lg bg-muted/50 p-4">
          <p className="mb-2 font-mono text-xs uppercase tracking-widest text-muted-foreground">
            Start here
          </p>
          <code className="block whitespace-pre-wrap break-words text-xs leading-6">{`curl $API/v1/assessments \\\n  -H "Authorization: Bearer <agent-key>"`}</code>
        </div>
      </section>
      <nav
        className="mb-5 flex overflow-x-auto border-b border-border"
        aria-label="Agent integration sections"
      >
        {[
          ["keys", "API keys"],
          ["tools", "MCP tools"],
          ["import", "Import demo"],
          ["activity", "Recent activity"],
        ].map(([value, label]) => (
          <button
            key={value}
            type="button"
            aria-selected={tab === value}
            onClick={() => setTab(value)}
            className={`whitespace-nowrap border-b-2 px-4 py-3 text-sm font-medium ${tab === value ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground"}`}
          >
            {label}
          </button>
        ))}
      </nav>
      {tab === "keys" && <KeysSection />}
      {tab === "tools" && <ToolsSection />}
      {tab === "import" && <ImportSection />}
      {tab === "activity" && <ActivitySection />}
    </PageShell>
  );
}

function KeysSection() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [label, setLabel] = useState("");
  const [focus, setFocus] = useState("");
  const [selectedScopes, setSelectedScopes] = useState(["assessment.read"]);
  const [secret, setSecret] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const load = () => {
    setLoading(true);
    (api as any)
      .GET("/v1/me/agents")
      .then(({ data }: { data?: { agents: Agent[] } }) =>
        setAgents(data?.agents ?? []),
      )
      .catch(() => setError("Could not load agents"))
      .finally(() => setLoading(false));
  };
  useEffect(load, []);
  async function create() {
    if (!label.trim()) return;
    setCreating(true);
    setError(null);
    try {
      const { data, error: requestError } = await (api as any).POST(
        "/v1/me/agents",
        {
          body: {
            label,
            scopes: selectedScopes,
            focusTags: focus
              .split(",")
              .map((v) => v.trim())
              .filter(Boolean),
          },
        },
      );
      if (requestError) throw new Error("Could not create agent");
      if (data?.apiKey) setSecret(data.apiKey);
      setLabel("");
      setFocus("");
      load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not create agent");
    } finally {
      setCreating(false);
    }
  }
  async function revoke(agentId: string) {
    if (!window.confirm("Revoke this agent and its tokens?")) return;
    await (api as any).DELETE("/v1/me/agents/{id}", {
      params: { path: { id: agentId } },
    });
    load();
  }
  return (
    <div className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
      {secret && (
        <div className="lg:col-span-2 rounded-xl border border-amber-500/40 bg-amber-500/10 p-5">
          <p className="font-semibold text-amber-800 dark:text-amber-200">
            Copy this secret now
          </p>
          <p className="mt-1 text-sm text-amber-800/80 dark:text-amber-200/80">
            It will not be shown again.
          </p>
          <div className="mt-3 flex gap-2">
            <code className="min-w-0 flex-1 overflow-auto rounded-lg bg-background p-3 text-xs">
              {secret}
            </code>
            <Button variant="outline" onClick={() => copyToClipboard(secret)}>
              <Copy /> Copy
            </Button>
            <Button variant="ghost" onClick={() => setSecret(null)}>
              Dismiss
            </Button>
          </div>
        </div>
      )}
      <section className="rounded-xl border border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-5 py-4">
          <div>
            <h2 className="font-semibold">Agent sub-accounts</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Separate keys with narrow scopes and visible activity.
            </p>
          </div>
          <span className="rounded-full border px-2.5 py-1 text-xs">
            {agents.length}
          </span>
        </div>
        {loading ? (
          <p className="p-6 text-sm text-muted-foreground">Loading agents…</p>
        ) : agents.length === 0 ? (
          <p className="p-8 text-center text-sm text-muted-foreground">
            No agent keys yet. Create one to connect an assistant.
          </p>
        ) : (
          <div className="divide-y divide-border">
            {agents.map((agent) => (
              <div key={agent.id} className="p-5">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex min-w-0 gap-3">
                    <div className="grid size-10 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary">
                      <KeyRound className="size-5" />
                    </div>
                    <div className="min-w-0">
                      <h3 className="font-medium">{agent.label}</h3>
                      <p className="mt-1 text-xs text-muted-foreground">
                        Created {formatTime(agent.createdAt)} ·{" "}
                        {(agent.tokens ?? []).length} token
                        {(agent.tokens ?? []).length === 1 ? "" : "s"}
                      </p>
                      {agent.focusTags?.length ? (
                        <div className="mt-2 flex flex-wrap gap-1.5">
                          {agent.focusTags.map((tag) => (
                            <span
                              key={tag}
                              className="rounded-full border border-sky-500/30 bg-sky-500/10 px-2 py-0.5 text-xs text-sky-700 dark:text-sky-300"
                            >
                              {tag}
                            </span>
                          ))}
                        </div>
                      ) : null}
                    </div>
                  </div>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => revoke(agent.id)}
                  >
                    <Trash2 /> Revoke
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
      <section className="rounded-xl border border-border bg-card p-5">
        <div className="mb-5">
          <h2 className="font-semibold">Create an agent</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Start with read-only access. Add write access only when the workflow
            needs it.
          </p>
        </div>
        <div className="space-y-4">
          <Field label="Agent label">
            <input
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="e.g. Grading assistant"
            />
          </Field>
          <Field label="Focus topics">
            <input
              value={focus}
              onChange={(e) => setFocus(e.target.value)}
              placeholder="rust, algorithms, logic"
            />
          </Field>
          <fieldset>
            <legend className="mb-2 text-sm font-medium">Capabilities</legend>
            <div className="space-y-2">
              {scopes.map((scope) => (
                <label
                  key={scope.value}
                  className="flex cursor-pointer gap-3 rounded-lg border border-border p-3 text-sm hover:bg-muted/40"
                >
                  <input
                    type="checkbox"
                    checked={selectedScopes.includes(scope.value)}
                    onChange={() =>
                      setSelectedScopes((current) =>
                        current.includes(scope.value)
                          ? current.filter((value) => value !== scope.value)
                          : [...current, scope.value],
                      )
                    }
                  />
                  <span>
                    <span className="block font-mono text-xs">
                      {scope.value}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {scope.label}
                    </span>
                  </span>
                </label>
              ))}
            </div>
          </fieldset>
          {error && (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          )}
          <Button
            className="w-full"
            onClick={create}
            disabled={creating || !label.trim()}
          >
            <Plus />
            {creating ? "Creating…" : "Create agent"}
          </Button>
        </div>
      </section>
    </div>
  );
}

function ToolsSection() {
  const [tools, setTools] = useState<Tool[]>([]);
  const [active, setActive] = useState<string | null>(null);
  useEffect(() => {
    (api as any)
      .GET("/skill.json")
      .then(({ data }: { data?: { tools: Tool[] } }) => {
        setTools(data?.tools ?? []);
        setActive(data?.tools?.[0]?.name ?? null);
      })
      .catch(console.error);
  }, []);
  const tool = tools.find((item) => item.name === active);
  return (
    <div className="grid gap-5 lg:grid-cols-[280px_1fr]">
      {" "}
      <section className="overflow-hidden rounded-xl border border-border bg-card">
        <div className="border-b border-border px-4 py-3">
          <h2 className="font-semibold">MCP tools</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            {tools.length} discoverable tools
          </p>
        </div>
        {tools.map((item) => (
          <button
            key={item.name}
            type="button"
            onClick={() => setActive(item.name)}
            className={`w-full border-b border-border px-4 py-3 text-left ${item.name === active ? "bg-primary/10" : "hover:bg-muted/40"}`}
          >
            <span className="block font-mono text-xs font-semibold">
              {item.name}
            </span>
            <span className="mt-1 block line-clamp-2 text-xs text-muted-foreground">
              {item.description}
            </span>
          </button>
        ))}
      </section>
      {tool ? (
        <section className="rounded-xl border border-border bg-card p-6">
          <p className="font-mono text-xs text-primary">{tool.name}</p>
          <h2 className="mt-2 text-lg font-semibold">{tool.description}</h2>
          <div className="mt-6">
            <p className="mb-2 text-xs font-semibold uppercase tracking-widest text-muted-foreground">
              Descriptor
            </p>
            <HighlightedCode
              code={JSON.stringify(tool, null, 2)}
              language="json"
            />
          </div>
        </section>
      ) : (
        <p className="text-sm text-muted-foreground">Loading tools…</p>
      )}
    </div>
  );
}

function ImportSection() {
  return (
    <section className="rounded-xl border border-border bg-card p-6">
      <div className="flex items-start gap-3">
        <div className="grid size-10 place-items-center rounded-lg bg-primary/10 text-primary">
          <Download className="size-5" />
        </div>
        <div>
          <h2 className="font-semibold">Import through the API</h2>
          <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
            Use an agent key to import assessments. The browser does not
            silently create content on your behalf; this keeps ownership and
            audit history explicit.
          </p>
        </div>
      </div>
      <div className="mt-6 rounded-lg bg-muted/50 p-4">
        <HighlightedCode
          code={`curl -X POST $API/v1/agents/run \\\n  -H "Authorization: Bearer <agent-key>" \\\n  -H "Content-Type: application/json" \\\n  -d '{"tool":"assessment.create","params":{...}}'`}
          language="bash"
        />
      </div>
      <a
        className="mt-4 inline-flex items-center gap-1 text-sm text-primary hover:underline"
        href="/openapi.yaml"
        target="_blank"
        rel="noreferrer"
      >
        Read the OpenAPI contract <ExternalLink className="size-3.5" />
      </a>
    </section>
  );
}

function ActivitySection() {
  const [entries, setEntries] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    (api as any)
      .GET("/v1/agents/activity", { params: { query: { limit: 50 } } })
      .then(({ data }: { data?: { items: Activity[] } }) =>
        setEntries(data?.items ?? []),
      )
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);
  return (
    <section className="rounded-xl border border-border bg-card">
      <div className="flex items-center justify-between border-b border-border px-5 py-4">
        <div>
          <h2 className="font-semibold">Recent activity</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            A lightweight audit trail for agent calls.
          </p>
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={() => window.location.reload()}
        >
          <RotateCcw /> Refresh
        </Button>
      </div>
      {loading ? (
        <p className="p-6 text-sm text-muted-foreground">Loading activity…</p>
      ) : entries.length === 0 ? (
        <p className="p-8 text-center text-sm text-muted-foreground">
          No agent activity yet.
        </p>
      ) : (
        <div className="divide-y divide-border">
          {entries.map((entry) => (
            <div
              key={entry.id}
              className="grid gap-2 px-5 py-4 text-sm sm:grid-cols-[150px_1fr_90px] sm:items-center"
            >
              <span className="font-mono text-xs text-muted-foreground">
                {formatTime(entry.ts)}
              </span>
              <span>
                <span className="block font-mono text-xs">
                  {entry.toolName}
                </span>
                <span className="text-xs text-muted-foreground">
                  {entry.agentName ?? "System"}
                </span>
              </span>
              <span
                className={
                  entry.status < 400
                    ? "text-emerald-600 dark:text-emerald-300"
                    : "text-destructive"
                }
              >
                {entry.status}
              </span>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="grid gap-1.5 text-sm font-medium">
      <span>{label}</span>
      {children}
    </label>
  );
}
