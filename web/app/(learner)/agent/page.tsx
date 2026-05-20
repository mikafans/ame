"use client";

import { useState, useEffect } from "react";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";

interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  createdAt: string;
  lastUsedAt?: string;
}

interface McpTool {
  name: string;
  description: string;
  inputSchema?: {
    required?: string[];
    properties?: Record<string, { description?: string; type?: string }>;
  };
}

interface ActivityEntry {
  id: string;
  toolName: string;
  agentId?: string;
  status: number;
  createdAt: string;
}

type TabId = "keys" | "tools" | "import" | "activity";

const SAMPLE_IMPORT = JSON.stringify(
  {
    title: "Sample quiz",
    questions: [
      {
        kind: "mc",
        prompt: "What is 2 + 2?",
        payload: {
          options: [{ text: "3" }, { text: "4" }, { text: "5" }],
          correct_index: 1,
        },
      },
    ],
  },
  null,
  2,
);

export default function AgentPage() {
  const { token, user } = useAuth();
  const [tab, setTab] = useState<TabId>("keys");

  const isInstructor = user?.role === "instructor" || user?.role === "admin";

  if (!isInstructor) {
    return (
      <div
        style={{ padding: "48px 36px", color: "var(--muted)", fontSize: 14 }}
      >
        Agent integration is available to instructors and admins only.
      </div>
    );
  }

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      {/* Header */}
      <div style={{ marginBottom: 22 }}>
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
          Programmatic surface · OpenAPI 3.1 · MCP-compatible
        </div>
        <h1
          style={{
            margin: 0,
            fontSize: 26,
            fontWeight: 600,
            color: "var(--text)",
          }}
        >
          Agent integration
        </h1>
      </div>

      {/* Explainer card */}
      <div
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          padding: "22px 28px",
          marginBottom: 22,
          display: "grid",
          gridTemplateColumns: "1.2fr 1fr",
          gap: 24,
        }}
      >
        <div>
          <div
            style={{
              fontSize: 15,
              fontWeight: 600,
              color: "var(--text)",
              marginBottom: 8,
            }}
          >
            Quizzes, attempts, and rubrics are first-class API objects.
          </div>
          <p
            style={{
              color: "var(--text-2)",
              fontSize: 13.5,
              lineHeight: 1.55,
              margin: 0,
            }}
          >
            Every screen a learner or instructor sees is backed by the same REST
            surface that agents use. No scraping, no duplicate state.
          </p>
          <div style={{ marginTop: 16, display: "flex", gap: 24 }}>
            {[
              { k: "Auth", v: "Bearer + scopes" },
              { k: "Rate limit", v: "120 / min" },
            ].map(({ k, v }) => (
              <div key={k}>
                <div
                  style={{
                    fontFamily: "var(--mono)",
                    fontSize: 10,
                    color: "var(--muted)",
                    textTransform: "uppercase",
                    letterSpacing: 1.2,
                    marginBottom: 2,
                  }}
                >
                  {k}
                </div>
                <div
                  style={{
                    fontSize: 15,
                    fontWeight: 600,
                    color: "var(--text)",
                  }}
                >
                  {v}
                </div>
              </div>
            ))}
          </div>
        </div>
        <div
          style={{
            background: "var(--surface-2)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            padding: 16,
          }}
        >
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 8,
            }}
          >
            Hello, world
          </div>
          <pre
            style={{
              margin: 0,
              fontFamily: "var(--mono)",
              fontSize: 12,
              color: "var(--text-2)",
              lineHeight: 1.6,
            }}
          >{`curl https://api.harus.app/v1/quizzes \\
  -H "Authorization: Bearer hk_live_…"

→ 200 OK · quizzes[]`}</pre>
        </div>
      </div>

      {/* Tabs */}
      <div
        style={{
          display: "flex",
          gap: 4,
          borderBottom: "1px solid var(--border)",
          marginBottom: 18,
        }}
      >
        {(
          [
            { id: "keys", label: "API keys" },
            { id: "tools", label: "MCP tools" },
            { id: "import", label: "Import demo" },
            { id: "activity", label: "Recent activity" },
          ] as { id: TabId; label: string }[]
        ).map((t) => {
          const active = tab === t.id;
          return (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              style={{
                background: "transparent",
                border: "none",
                cursor: "pointer",
                padding: "10px 14px",
                fontSize: 13,
                fontWeight: active ? 600 : 500,
                color: active ? "var(--text)" : "var(--muted)",
                borderBottom: `2px solid ${active ? "var(--accent)" : "transparent"}`,
                marginBottom: -1,
              }}
            >
              {t.label}
            </button>
          );
        })}
      </div>

      {tab === "keys" && <KeysTab token={token} />}
      {tab === "tools" && <ToolsTab token={token} />}
      {tab === "import" && <ImportTab token={token} />}
      {tab === "activity" && <ActivityTab token={token} />}
    </div>
  );
}

// ── API Keys tab ──────────────────────────────────────────────────────────────

function KeysTab({ token }: { token: string | undefined }) {
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [newKeyName, setNewKeyName] = useState("");
  const [newKeyScopes, setNewKeyScopes] = useState("quiz.read");
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [bootstrapping, setBootstrapping] = useState(false);
  const [bootstrapResult, setBootstrapResult] = useState<string | null>(null);

  function loadKeys() {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET("/v1/me/keys")
      .then(({ data }: { data?: { keys: ApiKey[] } }) => {
        if (data?.keys) setKeys(data.keys);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    loadKeys();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token]);

  async function createKey() {
    if (!token) return;
    setCreating(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (makeClient(token) as any).POST("/v1/me/keys", {
        body: {
          name: newKeyName || "New key",
          scopes: newKeyScopes
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
        },
      });
      if (data?.apiKey) {
        setCreatedSecret(data.apiKey);
        setNewKeyName("");
        loadKeys();
      }
    } catch {
      // noop
    } finally {
      setCreating(false);
    }
  }

  async function revokeKey(id: string) {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (makeClient(token) as any).DELETE(`/v1/me/keys/${id}`);
    loadKeys();
  }

  async function rotateKey(id: string) {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const { data } = await (makeClient(token) as any).POST(
      `/v1/me/keys/${id}/rotate`,
    );
    if (data?.apiKey) setCreatedSecret(data.apiKey);
    loadKeys();
  }

  async function bootstrapAgent() {
    if (!token) return;
    setBootstrapping(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (makeClient(token) as any).POST(
        "/v1/agents/register",
        {
          body: {
            displayName: "My agent",
            scopes: ["quiz.read", "quiz.write", "stats.read"],
          },
        },
      );
      if (data?.apiKey) {
        setBootstrapResult(data.apiKey);
        try {
          await navigator.clipboard.writeText(data.apiKey);
        } catch {
          // ignore clipboard error
        }
      }
    } catch {
      setBootstrapResult("Error registering agent");
    } finally {
      setBootstrapping(false);
    }
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1.3fr 1fr", gap: 18 }}>
      {/* Keys list */}
      <div
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "14px 20px",
            borderBottom: "1px solid var(--border)",
            fontSize: 15,
            fontWeight: 600,
            color: "var(--text)",
          }}
        >
          API keys
        </div>

        {/* Create form */}
        <div
          style={{
            padding: "14px 20px",
            borderBottom: "1px solid var(--border)",
            display: "flex",
            gap: 8,
          }}
        >
          <input
            value={newKeyName}
            onChange={(e) => setNewKeyName(e.target.value)}
            placeholder="Key name"
            style={{ ...inputStyle, flex: 1 }}
          />
          <input
            value={newKeyScopes}
            onChange={(e) => setNewKeyScopes(e.target.value)}
            placeholder="scopes (comma-sep)"
            style={{ ...inputStyle, flex: 1 }}
          />
          <button onClick={createKey} disabled={creating} style={btnAccent}>
            {creating ? "…" : "Create"}
          </button>
        </div>

        {loading ? (
          <div
            style={{
              padding: "24px 20px",
              color: "var(--muted)",
              fontSize: 13,
            }}
          >
            Loading…
          </div>
        ) : keys.length === 0 ? (
          <div
            style={{
              padding: "24px 20px",
              color: "var(--muted)",
              fontSize: 13,
            }}
          >
            No API keys yet.
          </div>
        ) : (
          keys.map((k, i) => (
            <div
              key={k.id}
              style={{
                padding: "16px 20px",
                borderBottom:
                  i < keys.length - 1 ? "1px solid var(--border)" : "none",
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <div>
                  <div
                    style={{
                      fontSize: 13.5,
                      fontWeight: 600,
                      color: "var(--text)",
                    }}
                  >
                    {k.name}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 12,
                      color: "var(--text-2)",
                      marginTop: 4,
                    }}
                  >
                    {k.prefix}
                  </div>
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <button onClick={() => rotateKey(k.id)} style={btnGhost}>
                    Rotate
                  </button>
                  <button
                    onClick={() => revokeKey(k.id)}
                    style={{
                      ...btnGhost,
                      color: "#ef4444",
                      borderColor: "#ef4444",
                    }}
                  >
                    Revoke
                  </button>
                </div>
              </div>
              <div
                style={{
                  marginTop: 10,
                  display: "flex",
                  gap: 6,
                  flexWrap: "wrap",
                }}
              >
                {k.scopes.map((s) => (
                  <span
                    key={s}
                    style={{
                      fontFamily: "var(--mono)",
                      fontSize: 10,
                      padding: "2px 7px",
                      background: "var(--surface-2)",
                      border: "1px solid var(--border)",
                      borderRadius: 3,
                      color: s === "*" ? "#f59e0b" : "var(--text-2)",
                    }}
                  >
                    {s}
                  </span>
                ))}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Right panel: auth info + bootstrap */}
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        <div
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            padding: 20,
          }}
        >
          <div style={sectionLabel}>Authentication</div>
          <div
            style={{
              fontSize: 15,
              fontWeight: 600,
              color: "var(--text)",
              marginBottom: 8,
            }}
          >
            Header-based bearer token
          </div>
          <p
            style={{
              color: "var(--text-2)",
              fontSize: 13,
              lineHeight: 1.6,
              margin: 0,
            }}
          >
            Send the key in an <code style={codeStyle}>Authorization</code>{" "}
            header. Scopes are checked per endpoint.
          </p>
          <div
            style={{
              marginTop: 14,
              background: "var(--surface-2)",
              border: "1px solid var(--border)",
              borderRadius: 6,
              padding: "10px 14px",
            }}
          >
            <pre
              style={{
                margin: 0,
                fontFamily: "var(--mono)",
                fontSize: 11.5,
                color: "var(--text-2)",
                lineHeight: 1.6,
              }}
            >
              {`GET /v1/quizzes
Authorization: Bearer hk_live_…`}
            </pre>
          </div>
          <div
            style={{
              marginTop: 16,
              paddingTop: 14,
              borderTop: "1px solid var(--border)",
            }}
          >
            <div style={sectionLabel}>Scope reference</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {[
                "quiz.read",
                "quiz.write",
                "attempt.read",
                "stats.read",
                "feedback.write",
                "plan.write",
              ].map((s) => (
                <div
                  key={s}
                  style={{
                    display: "flex",
                    gap: 8,
                    fontSize: 12,
                    color: "var(--text-2)",
                  }}
                >
                  <span style={{ color: "var(--accent)" }}>✓</span>
                  <code style={codeStyle}>{s}</code>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Bootstrap helper */}
        <div
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            padding: 20,
          }}
        >
          <div style={sectionLabel}>Bootstrap agent</div>
          <p
            style={{
              color: "var(--text-2)",
              fontSize: 13,
              lineHeight: 1.6,
              margin: "0 0 14px",
            }}
          >
            Mint an agent user instantly. The API key is shown once — copy it
            before dismissing.
          </p>
          {bootstrapResult ? (
            <div>
              <div
                style={{
                  padding: "10px 12px",
                  background: "var(--surface-2)",
                  border: "1px solid var(--border)",
                  borderRadius: 4,
                  fontFamily: "var(--mono)",
                  fontSize: 12,
                  color: "var(--text)",
                  wordBreak: "break-all",
                  marginBottom: 8,
                }}
              >
                {bootstrapResult}
              </div>
              <button
                onClick={() => {
                  navigator.clipboard
                    .writeText(bootstrapResult)
                    .catch(() => {});
                }}
                style={btnGhost}
              >
                Copy key
              </button>
            </div>
          ) : (
            <button
              onClick={bootstrapAgent}
              disabled={bootstrapping}
              style={{ ...btnAccent, opacity: bootstrapping ? 0.7 : 1 }}
            >
              {bootstrapping ? "Registering…" : "Register agent"}
            </button>
          )}
        </div>
      </div>

      {/* Secret shown once modal */}
      {createdSecret && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.6)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 100,
          }}
        >
          <div
            style={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: 8,
              padding: 28,
              width: 480,
            }}
          >
            <div
              style={{
                fontSize: 16,
                fontWeight: 600,
                color: "var(--text)",
                marginBottom: 10,
              }}
            >
              API key created
            </div>
            <p
              style={{ color: "var(--text-2)", fontSize: 13, lineHeight: 1.6 }}
            >
              Copy this key now — it will not be shown again.
            </p>
            <div
              style={{
                padding: "12px 14px",
                background: "var(--surface-2)",
                border: "1px solid var(--accent)",
                borderRadius: 4,
                fontFamily: "var(--mono)",
                fontSize: 13,
                color: "var(--accent)",
                wordBreak: "break-all",
                marginBottom: 14,
              }}
            >
              {createdSecret}
            </div>
            <div
              style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}
            >
              <button
                onClick={() => {
                  navigator.clipboard.writeText(createdSecret).catch(() => {});
                }}
                style={btnGhost}
              >
                Copy
              </button>
              <button onClick={() => setCreatedSecret(null)} style={btnAccent}>
                Done — I&apos;ve copied it
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ── MCP Tools tab ─────────────────────────────────────────────────────────────

function ToolsTab({ token }: { token: string | undefined }) {
  const [tools, setTools] = useState<McpTool[]>([]);
  const [loading, setLoading] = useState(true);
  const [active, setActive] = useState<string | null>(null);

  useEffect(() => {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET("/v1/agents/mcp.json")
      .then(({ data }: { data?: { tools: McpTool[] } }) => {
        if (data?.tools) {
          setTools(data.tools);
          if (data.tools.length) setActive(data.tools[0].name);
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [token]);

  const tool = tools.find((t) => t.name === active) ?? null;

  if (loading) {
    return (
      <div
        style={{
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Loading MCP manifest…
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "280px 1fr", gap: 18 }}>
      {/* Tool list */}
      <div
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "14px 18px",
            borderBottom: "1px solid var(--border)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div style={{ fontSize: 15, fontWeight: 600, color: "var(--text)" }}>
            MCP descriptors
          </div>
          <span
            style={{
              fontFamily: "var(--mono)",
              fontSize: 11,
              color: "var(--accent)",
            }}
          >
            {tools.length}
          </span>
        </div>
        {tools.map((t) => {
          const sel = active === t.name;
          return (
            <button
              key={t.name}
              onClick={() => setActive(t.name)}
              style={{
                width: "100%",
                padding: "12px 18px",
                background: sel
                  ? "var(--accent-dim, rgba(0,200,100,0.08))"
                  : "transparent",
                border: "none",
                borderLeft: `2px solid ${sel ? "var(--accent)" : "transparent"}`,
                borderBottom: "1px solid var(--border)",
                textAlign: "left",
                cursor: "pointer",
              }}
            >
              <div
                style={{
                  fontFamily: "var(--mono)",
                  fontSize: 13,
                  fontWeight: 600,
                  color: sel ? "var(--accent)" : "var(--text)",
                }}
              >
                {t.name}
              </div>
              <div
                style={{
                  fontSize: 12,
                  color: "var(--muted)",
                  marginTop: 3,
                  lineHeight: 1.4,
                }}
              >
                {t.description}
              </div>
            </button>
          );
        })}
      </div>

      {/* Tool detail */}
      {tool ? (
        <div
          style={{
            background: "var(--surface)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            overflow: "hidden",
          }}
        >
          <div
            style={{
              padding: "18px 22px",
              borderBottom: "1px solid var(--border)",
            }}
          >
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--accent)",
                letterSpacing: 0.5,
              }}
            >
              {tool.name}
            </div>
            <div
              style={{
                fontSize: 20,
                fontWeight: 600,
                color: "var(--text)",
                marginTop: 4,
              }}
            >
              {tool.description}
            </div>
          </div>
          <div style={{ padding: 22 }}>
            <div style={sectionLabel}>MCP descriptor</div>
            <pre
              style={{
                background: "var(--surface-2)",
                border: "1px solid var(--border)",
                borderRadius: 6,
                padding: "12px 14px",
                fontFamily: "var(--mono)",
                fontSize: 12,
                color: "var(--text-2)",
                lineHeight: 1.65,
                overflowX: "auto",
                margin: 0,
              }}
            >
              {JSON.stringify(tool, null, 2)}
            </pre>
          </div>
        </div>
      ) : (
        <div style={{ color: "var(--muted)", fontSize: 13 }}>
          Select a tool to view its descriptor.
        </div>
      )}
    </div>
  );
}

// ── Import demo tab ───────────────────────────────────────────────────────────

function ImportTab({ token }: { token: string | undefined }) {
  const [text, setText] = useState(SAMPLE_IMPORT);
  const [response, setResponse] = useState<{
    ok: boolean;
    status: number;
    body: unknown;
    latencyMs: number;
  } | null>(null);
  const [running, setRunning] = useState(false);

  async function send() {
    if (!token) return;
    setRunning(true);
    const t0 = Date.now();
    try {
      let body: unknown;
      try {
        body = JSON.parse(text);
      } catch {
        setResponse({
          ok: false,
          status: 0,
          body: { error: "JSON parse error" },
          latencyMs: 0,
        });
        return;
      }
      /* eslint-disable @typescript-eslint/no-explicit-any */
      const client = makeClient(token) as any;
      /* eslint-enable @typescript-eslint/no-explicit-any */
      const {
        data,
        error,
        response: res,
      } = await client.POST("/v1/quizzes", { body });
      const latencyMs = Date.now() - t0;
      setResponse({
        ok: !error,
        status: res?.status ?? (error ? 400 : 200),
        body: data ?? error,
        latencyMs,
      });
    } catch {
      setResponse({
        ok: false,
        status: 0,
        body: { error: "Network error" },
        latencyMs: 0,
      });
    } finally {
      setRunning(false);
    }
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1.1fr 1fr", gap: 18 }}>
      <div
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "14px 20px",
            borderBottom: "1px solid var(--border)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div>
            <div
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--accent)",
              }}
            >
              quiz.import
            </div>
            <div
              style={{
                fontSize: 16,
                fontWeight: 600,
                color: "var(--text)",
                marginTop: 2,
              }}
            >
              Try the import endpoint
            </div>
          </div>
        </div>
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          spellCheck={false}
          style={{
            width: "100%",
            minHeight: 340,
            background: "var(--surface-2)",
            border: "none",
            borderTop: "1px solid var(--border)",
            color: "var(--text)",
            fontFamily: "var(--mono)",
            fontSize: 12.5,
            lineHeight: 1.6,
            padding: "14px 18px",
            outline: "none",
            resize: "vertical",
            boxSizing: "border-box",
          }}
        />
        <div
          style={{
            padding: "12px 20px",
            borderTop: "1px solid var(--border)",
            background: "var(--surface)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span
            style={{
              fontFamily: "var(--mono)",
              fontSize: 11,
              color: "var(--muted)",
            }}
          >
            POST /v1/quizzes · Bearer hk_live_…
          </span>
          <button onClick={send} disabled={running} style={btnAccent}>
            {running ? "Sending…" : "Send request →"}
          </button>
        </div>
      </div>

      <div
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "14px 20px",
            borderBottom: "1px solid var(--border)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div style={{ fontSize: 16, fontWeight: 600, color: "var(--text)" }}>
            Response
          </div>
          {response ? (
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: response.ok ? "var(--accent)" : "#ef4444",
                padding: "2px 8px",
                background: response.ok
                  ? "var(--accent-dim, rgba(0,200,100,0.1))"
                  : "rgba(239,68,68,0.1)",
                borderRadius: 3,
              }}
            >
              {response.status} · {response.latencyMs}ms
            </span>
          ) : (
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--muted)",
              }}
            >
              awaiting request
            </span>
          )}
        </div>
        {response ? (
          <div style={{ padding: 20 }}>
            <pre
              style={{
                background: "var(--surface-2)",
                border: "1px solid var(--border)",
                borderRadius: 6,
                padding: "12px 14px",
                fontFamily: "var(--mono)",
                fontSize: 12,
                color: "var(--text-2)",
                lineHeight: 1.65,
                overflowX: "auto",
                margin: 0,
              }}
            >
              {JSON.stringify(response.body, null, 2)}
            </pre>
          </div>
        ) : (
          <div
            style={{
              padding: "48px 20px",
              textAlign: "center",
              color: "var(--muted)",
              fontFamily: "var(--mono)",
              fontSize: 12,
            }}
          >
            Click Send request to see the response
          </div>
        )}
      </div>
    </div>
  );
}

// ── Activity tab ──────────────────────────────────────────────────────────────

function ActivityTab({ token }: { token: string | undefined }) {
  const [entries, setEntries] = useState<ActivityEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET("/v1/agents/activity", { params: { query: { limit: 50 } } })
      .then(({ data }: { data?: { entries: ActivityEntry[] } }) => {
        if (data?.entries) setEntries(data.entries);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [token]);

  return (
    <div
      style={{
        background: "var(--surface)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          padding: "14px 20px",
          borderBottom: "1px solid var(--border)",
          fontSize: 16,
          fontWeight: 600,
          color: "var(--text)",
        }}
      >
        Recent activity
      </div>
      {loading ? (
        <div
          style={{ padding: "24px 20px", color: "var(--muted)", fontSize: 13 }}
        >
          Loading…
        </div>
      ) : entries.length === 0 ? (
        <div
          style={{ padding: "24px 20px", color: "var(--muted)", fontSize: 13 }}
        >
          No agent activity yet.
        </div>
      ) : (
        <div style={{ fontFamily: "var(--mono)" }}>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "160px 1fr 80px",
              padding: "8px 20px",
              borderBottom: "1px solid var(--border)",
              background: "var(--surface-2)",
              fontSize: 10,
              letterSpacing: 1.1,
              textTransform: "uppercase",
              color: "var(--muted)",
            }}
          >
            <span>Time</span>
            <span>Tool</span>
            <span>Status</span>
          </div>
          {entries.map((e, i) => (
            <div
              key={e.id}
              style={{
                display: "grid",
                gridTemplateColumns: "160px 1fr 80px",
                padding: "12px 20px",
                borderBottom:
                  i < entries.length - 1 ? "1px solid var(--border)" : "none",
                fontSize: 12,
                alignItems: "center",
              }}
            >
              <span style={{ color: "var(--muted)", letterSpacing: 0.5 }}>
                {new Date(e.createdAt).toLocaleTimeString()}
              </span>
              <span style={{ color: "var(--accent)" }}>{e.toolName}</span>
              <span
                style={{
                  color: e.status < 400 ? "var(--accent)" : "#ef4444",
                  fontWeight: 600,
                }}
              >
                {e.status}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── shared styles ─────────────────────────────────────────────────────────────

const sectionLabel: React.CSSProperties = {
  fontFamily: "var(--mono)",
  fontSize: 10,
  letterSpacing: 1.3,
  textTransform: "uppercase",
  color: "var(--muted)",
  marginBottom: 10,
};

const inputStyle: React.CSSProperties = {
  background: "var(--surface-2)",
  border: "1px solid var(--border)",
  borderRadius: 4,
  padding: "7px 10px",
  color: "var(--text)",
  fontFamily: "var(--sans, sans-serif)",
  fontSize: 13,
  outline: "none",
  boxSizing: "border-box",
};

const btnAccent: React.CSSProperties = {
  padding: "7px 14px",
  background: "var(--accent)",
  border: "none",
  borderRadius: 4,
  color: "#000",
  fontWeight: 600,
  fontSize: 13,
  cursor: "pointer",
  flexShrink: 0,
};

const btnGhost: React.CSSProperties = {
  padding: "6px 12px",
  background: "transparent",
  border: "1px solid var(--border)",
  borderRadius: 4,
  color: "var(--text-2)",
  fontSize: 12,
  cursor: "pointer",
};

const codeStyle: React.CSSProperties = {
  fontFamily: "var(--mono)",
  fontSize: 12,
  background: "var(--surface-2)",
  border: "1px solid var(--border)",
  borderRadius: 3,
  padding: "1px 5px",
  color: "var(--text)",
};
