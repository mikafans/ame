"use client";

import React, { useState, useEffect } from "react";
import { api } from "@/api/client";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import Paper from "@mui/material/Paper";
import Chip from "@mui/material/Chip";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import CircularProgress from "@mui/material/CircularProgress";

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
  const [tab, setTab] = useState<TabId>("keys");

  return (
    <Box sx={{ p: "28px 36px 56px" }}>
      {/* Header */}
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          mb: 2.75,
        }}
      >
        <Box>
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              fontFamily: "monospace",
              letterSpacing: 1.3,
              textTransform: "uppercase",
              display: "block",
              mb: 0.75,
            }}
          >
            Programmatic surface · OpenAPI 3.1 · MCP-compatible
          </Typography>
          <Typography variant="h4" sx={{ fontWeight: 500 }}>
            Agent integration
          </Typography>
        </Box>
        <Stack direction="row" spacing={1}>
          <Button variant="outlined" size="small">
            OpenAPI
          </Button>
          <Button variant="outlined" size="small">
            MCP manifest
          </Button>
          <Button variant="contained" size="small">
            + New API key
          </Button>
        </Stack>
      </Box>

      {/* Intro card */}
      <Card
        variant="outlined"
        sx={{
          mb: 3.5,
          display: "grid",
          gridTemplateColumns: "1.2fr 1fr",
          overflow: "hidden",
        }}
      >
        <Box sx={{ p: 3.5 }}>
          <Chip
            label="Two interfaces, one model"
            color="primary"
            size="small"
            sx={{ mb: 1.5 }}
          />
          <Typography
            variant="h5"
            sx={{ fontWeight: 500, mb: 1, letterSpacing: -0.3 }}
          >
            Quizzes, attempts, and rubrics are first-class API objects.
          </Typography>
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ lineHeight: 1.55, mb: 2.75, maxWidth: 540 }}
          >
            Every screen you see is backed by the same REST surface that agents
            use. A grading agent reads an attempt with one call; an authoring
            agent imports a new quiz with another. No scraping, no duplicate
            state.
          </Typography>
          <Stack direction="row" spacing={2.75}>
            <KV2 k="Endpoints" v="34" />
            <KV2 k="Auth" v="Bearer + scopes" />
            <KV2 k="Rate limit" v="120 / min" />
            <KV2 k="SDKs" v="ts · py · go" />
          </Stack>
        </Box>
        <Box
          sx={{
            bgcolor: "action.hover",
            borderLeft: 1,
            borderColor: "divider",
            p: "22px 26px",
          }}
        >
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              fontFamily: "monospace",
              letterSpacing: 1.3,
              textTransform: "uppercase",
              display: "block",
              mb: 1.25,
            }}
          >
            Hello, world
          </Typography>
          <CodeBlock
            label="curl"
            lines={[
              `curl https://api.ame-platform.app/v1/quizzes \\`,
              `  -H "Authorization: Bearer hk_live_3fY9…ax2P" \\`,
              `  -H "Content-Type: application/json"`,
              ``,
              `→ 200 OK · 24 quizzes`,
            ]}
            dim={[4]}
          />
        </Box>
      </Card>

      {/* Tabs */}
      <Tabs
        value={tab}
        onChange={(_, v) => setTab(v as TabId)}
        sx={{ mb: 2.25, borderBottom: 1, borderColor: "divider" }}
      >
        <Tab value="keys" label="API keys" sx={{ textTransform: "none" }} />
        <Tab value="tools" label="MCP tools" sx={{ textTransform: "none" }} />
        <Tab
          value="import"
          label="Import demo"
          sx={{ textTransform: "none" }}
        />
        <Tab
          value="activity"
          label="Recent activity"
          sx={{ textTransform: "none" }}
        />
      </Tabs>

      {tab === "keys" && <KeysTab />}
      {tab === "tools" && <ToolsTab />}
      {tab === "import" && <ImportTab />}
      {tab === "activity" && <ActivityTab />}
    </Box>
  );
}

function KV2({ k, v }: { k: string; v: string }) {
  return (
    <Box>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{
          fontFamily: "monospace",
          letterSpacing: 1.2,
          textTransform: "uppercase",
          display: "block",
          mb: 0.25,
        }}
      >
        {k}
      </Typography>
      <Typography variant="h6" sx={{ fontWeight: 500 }}>
        {v}
      </Typography>
    </Box>
  );
}

function CodeBlock({
  label,
  lines,
  dim = [],
  style,
}: {
  label?: string;
  lines: string[];
  dim?: number[];
  style?: React.CSSProperties;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(lines.join("\n")).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Paper
      variant="outlined"
      style={style}
      sx={{ overflow: "hidden", borderRadius: 1 }}
    >
      {label && (
        <Box
          sx={{
            px: 1.5,
            py: 0.75,
            borderBottom: 1,
            borderColor: "divider",
            bgcolor: "background.paper",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: "monospace", letterSpacing: 1 }}
          >
            {label}
          </Typography>
          <Button
            size="small"
            variant="text"
            onClick={handleCopy}
            sx={{ minWidth: 0, p: 0.25 }}
          >
            {copied ? "✓" : "⎘"}
          </Button>
        </Box>
      )}
      <Box
        component="pre"
        sx={{
          m: 0,
          p: "12px 14px",
          fontFamily: "monospace",
          fontSize: 12,
          lineHeight: 1.65,
          color: "text.secondary",
          whiteSpace: "pre-wrap",
          wordBreak: "break-word",
        }}
      >
        {lines.map((l, i) => (
          <Box
            key={i}
            component="div"
            sx={{ color: dim.includes(i) ? "primary.main" : undefined }}
          >
            {l || " "}
          </Box>
        ))}
      </Box>
    </Paper>
  );
}

// ── API Keys tab ──────────────────────────────────────────────────────────────

function KeysTab() {
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [loading, setLoading] = useState(true);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [creating, setCreating] = useState(false);
  const [newKeyName, setNewKeyName] = useState("");
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [newKeyScopes, setNewKeyScopes] = useState("quiz.read");
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);

  function loadKeys() {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (api as any)
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
  }, []);

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  async function createKey() {
    setCreating(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (api as any).POST("/v1/me/keys", {
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

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  async function revokeKey(id: string) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (api as any).DELETE(`/v1/me/keys/${id}`);
    loadKeys();
  }

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  async function rotateKey(id: string) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const { data } = await (api as any).POST(`/v1/me/keys/${id}/rotate`);
    if (data?.apiKey) setCreatedSecret(data.apiKey);
    loadKeys();
  }

  return (
    <Box sx={{ display: "grid", gridTemplateColumns: "1.4fr 1fr", gap: 2.25 }}>
      {/* Keys list */}
      <Card variant="outlined">
        <Box
          sx={{
            px: 2.5,
            py: 1.75,
            borderBottom: 1,
            borderColor: "divider",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Typography variant="subtitle1" sx={{ fontWeight: 500 }}>
            API keys
          </Typography>
          <Button size="small" variant="outlined">
            Create
          </Button>
        </Box>

        {loading ? (
          <Box sx={{ p: "24px 20px", color: "text.secondary", fontSize: 13 }}>
            Loading…
          </Box>
        ) : keys.length === 0 ? (
          <Box sx={{ p: "24px 20px" }}>
            <Typography variant="body2" color="text.secondary">
              No API keys yet.
            </Typography>
          </Box>
        ) : (
          keys.map((k, i) => (
            <Box
              key={k.id}
              sx={{
                px: 2.5,
                py: 2,
                borderBottom: i < keys.length - 1 ? 1 : 0,
                borderColor: "divider",
              }}
            >
              <Box
                sx={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                }}
              >
                <Box>
                  <Typography variant="body2" sx={{ fontWeight: 600 }}>
                    {k.name}
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{
                      fontFamily: "monospace",
                      letterSpacing: 0.4,
                      display: "block",
                      mt: 0.5,
                    }}
                  >
                    {k.prefix}
                  </Typography>
                </Box>
                <Stack direction="row" spacing={0.75}>
                  <Button size="small" variant="text">
                    Copy
                  </Button>
                  <Button size="small" variant="text">
                    Rotate
                  </Button>
                  <Button size="small" variant="text" color="error">
                    Revoke
                  </Button>
                </Stack>
              </Box>
              <Box
                sx={{ mt: 1.25, display: "flex", gap: 2, alignItems: "center" }}
              >
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ fontFamily: "monospace", letterSpacing: 0.5 }}
                >
                  Created {k.createdAt} · Last used {k.lastUsedAt || "—"}
                </Typography>
                <Stack direction="row" spacing={0.5} sx={{ ml: "auto" }}>
                  {k.scopes.map((s) => (
                    <Chip
                      key={s}
                      label={s}
                      size="small"
                      color={s === "*" ? "warning" : "default"}
                      variant="outlined"
                    />
                  ))}
                </Stack>
              </Box>
            </Box>
          ))
        )}
      </Card>

      {/* Right panel: auth info */}
      <Card variant="outlined" sx={{ p: 2.5 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: "monospace",
            letterSpacing: 1.3,
            textTransform: "uppercase",
            display: "block",
            mb: 1.25,
          }}
        >
          Authentication
        </Typography>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Header-based bearer token
        </Typography>
        <Typography
          variant="body2"
          color="text.secondary"
          sx={{ lineHeight: 1.6, mb: 0 }}
        >
          Send the key in an{" "}
          <Box component="code" sx={codeStyle}>
            Authorization
          </Box>{" "}
          header. Scopes are checked per endpoint.
        </Typography>
        <CodeBlock
          label="request"
          style={{ marginTop: 14 }}
          lines={[
            `GET /v1/quizzes/qz_8sd1/stats`,
            `Authorization: Bearer hk_live_3fY9…ax2P`,
            `Accept: application/json`,
            `X-Cohort: spring-2026`,
          ]}
        />
        <Box sx={{ mt: 2.25, pt: 2.25, borderTop: 1, borderColor: "divider" }}>
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              fontFamily: "monospace",
              letterSpacing: 1.3,
              textTransform: "uppercase",
              display: "block",
              mb: 1,
            }}
          >
            Scope reference
          </Typography>
          <Box
            sx={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 0.75 }}
          >
            {[
              "quiz.read",
              "quiz.write",
              "attempt.read",
              "stats.read",
              "feedback.write",
              "plan.write",
            ].map((s) => (
              <Box
                key={s}
                sx={{
                  display: "flex",
                  gap: 1,
                  fontSize: 12,
                  color: "text.secondary",
                }}
              >
                <Typography variant="caption" color="primary.main">
                  ✓
                </Typography>
                <Box component="code" sx={codeStyle}>
                  {s}
                </Box>
              </Box>
            ))}
          </Box>
        </Box>
      </Card>

      {/* Secret shown once modal */}
      {createdSecret && (
        <Box
          sx={{
            position: "fixed",
            inset: 0,
            bgcolor: "rgba(0,0,0,0.6)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1300,
          }}
        >
          <Card sx={{ width: 480, p: 3.5 }}>
            <Typography variant="h6" sx={{ fontWeight: 600, mb: 1.25 }}>
              API key created
            </Typography>
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ lineHeight: 1.6, mb: 1.75 }}
            >
              Copy this key now — it will not be shown again.
            </Typography>
            <Box
              sx={{
                p: "12px 14px",
                bgcolor: "action.hover",
                border: 1,
                borderColor: "primary.main",
                borderRadius: 0.5,
                fontFamily: "monospace",
                fontSize: 13,
                color: "primary.main",
                wordBreak: "break-all",
                mb: 1.75,
              }}
            >
              {createdSecret}
            </Box>
            <Stack
              direction="row"
              spacing={1}
              sx={{ justifyContent: "flex-end" }}
            >
              <Button
                variant="outlined"
                size="small"
                onClick={() => {
                  navigator.clipboard.writeText(createdSecret).catch(() => {});
                }}
              >
                Copy
              </Button>
              <Button
                variant="contained"
                size="small"
                onClick={() => setCreatedSecret(null)}
              >
                Done
              </Button>
            </Stack>
          </Card>
        </Box>
      )}
    </Box>
  );
}

// ── MCP Tools tab ─────────────────────────────────────────────────────────────

function ToolsTab() {
  const [tools, setTools] = useState<McpTool[]>([]);
  const [loading, setLoading] = useState(true);
  const [active, setActive] = useState<string | null>(null);

  useEffect(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (api as any)
      .GET("/v1/agents/mcp.json")
      .then(({ data }: { data?: { tools: McpTool[] } }) => {
        if (data?.tools) {
          setTools(data.tools);
          if (data.tools.length) setActive(data.tools[0].name);
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const tool = tools.find((t) => t.name === active) ?? null;

  if (loading) {
    return (
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 1,
          color: "text.secondary",
        }}
      >
        <CircularProgress size={16} />
        <Typography variant="body2">Loading MCP manifest…</Typography>
      </Box>
    );
  }

  return (
    <Box sx={{ display: "grid", gridTemplateColumns: "280px 1fr", gap: 2.25 }}>
      {/* Tool list */}
      <Card variant="outlined">
        <Box
          sx={{
            px: 2.25,
            py: 1.75,
            borderBottom: 1,
            borderColor: "divider",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
            MCP descriptors
          </Typography>
          <Chip label={tools.length} color="primary" size="small" />
        </Box>
        {tools.map((t) => {
          const sel = active === t.name;
          return (
            <Box
              key={t.name}
              component="button"
              onClick={() => setActive(t.name)}
              sx={{
                width: "100%",
                px: 2.25,
                py: 1.5,
                bgcolor: sel ? "primary.50" : "transparent",
                border: "none",
                borderLeft: `2px solid`,
                borderLeftColor: sel ? "primary.main" : "transparent",
                borderBottom: 1,
                borderColor: "divider",
                textAlign: "left",
                cursor: "pointer",
              }}
            >
              <Typography
                variant="caption"
                sx={{
                  fontFamily: "monospace",
                  fontWeight: 600,
                  color: sel ? "primary.main" : "text.primary",
                  display: "block",
                }}
              >
                {t.name}
              </Typography>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ lineHeight: 1.4 }}
              >
                {t.description}
              </Typography>
            </Box>
          );
        })}
      </Card>

      {/* Tool detail */}
      {tool ? (
        <Card variant="outlined">
          <Box
            sx={{ px: 2.75, py: 2.25, borderBottom: 1, borderColor: "divider" }}
          >
            <Typography
              variant="caption"
              color="primary.main"
              sx={{
                fontFamily: "monospace",
                letterSpacing: 0.5,
                display: "block",
              }}
            >
              {tool.name}
            </Typography>
            <Typography variant="h6" sx={{ fontWeight: 600, mt: 0.5 }}>
              {tool.description}
            </Typography>
          </Box>
          <Box sx={{ p: 2.75 }}>
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{
                fontFamily: "monospace",
                letterSpacing: 1.3,
                textTransform: "uppercase",
                display: "block",
                mb: 1.25,
              }}
            >
              MCP descriptor
            </Typography>
            <Box
              component="pre"
              sx={{
                m: 0,
                p: "12px 14px",
                bgcolor: "action.hover",
                border: 1,
                borderColor: "divider",
                borderRadius: 1,
                fontFamily: "monospace",
                fontSize: 12,
                color: "text.secondary",
                lineHeight: 1.65,
                overflowX: "auto",
              }}
            >
              {JSON.stringify(tool, null, 2)}
            </Box>
          </Box>
        </Card>
      ) : (
        <Typography variant="body2" color="text.secondary">
          Select a tool to view its descriptor.
        </Typography>
      )}
    </Box>
  );
}

// ── Import demo tab ───────────────────────────────────────────────────────────

function ImportTab() {
  const [text, setText] = useState(SAMPLE_IMPORT);
  const [response, setResponse] = useState<{
    ok: boolean;
    status: number;
    body: unknown;
    latencyMs: number;
  } | null>(null);
  const [running, setRunning] = useState(false);

  async function send() {
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
      const client = api as any;
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
    <Box sx={{ display: "grid", gridTemplateColumns: "1.1fr 1fr", gap: 2.25 }}>
      <Card variant="outlined">
        <Box
          sx={{
            px: 2.5,
            py: 1.75,
            borderBottom: 1,
            borderColor: "divider",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Box>
            <Typography
              variant="caption"
              color="primary.main"
              sx={{ fontFamily: "monospace", display: "block" }}
            >
              quiz.import
            </Typography>
            <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
              Try the import endpoint
            </Typography>
          </Box>
        </Box>
        <Box
          component="textarea"
          value={text}
          onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) =>
            setText(e.target.value)
          }
          spellCheck={false}
          sx={{
            width: "100%",
            minHeight: 340,
            bgcolor: "action.hover",
            border: "none",
            borderTop: 1,
            borderColor: "divider",
            color: "text.primary",
            fontFamily: "monospace",
            fontSize: 12.5,
            lineHeight: 1.6,
            p: "14px 18px",
            outline: "none",
            resize: "vertical",
            boxSizing: "border-box",
            display: "block",
          }}
        />
        <Box
          sx={{
            px: 2.5,
            py: 1.5,
            borderTop: 1,
            borderColor: "divider",
            bgcolor: "background.paper",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: "monospace" }}
          >
            POST /v1/quizzes · Bearer hk_live_…
          </Typography>
          <Button
            variant="contained"
            size="small"
            onClick={send}
            disabled={running}
          >
            {running ? "Sending…" : "Send request →"}
          </Button>
        </Box>
      </Card>

      <Card variant="outlined">
        <Box
          sx={{
            px: 2.5,
            py: 1.75,
            borderBottom: 1,
            borderColor: "divider",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
            Response
          </Typography>
          {response ? (
            <Chip
              label={`${response.status} · ${response.latencyMs}ms`}
              color={response.ok ? "success" : "warning"}
              size="small"
            />
          ) : (
            <Chip label="awaiting request" variant="outlined" size="small" />
          )}
        </Box>
        {response ? (
          <Box sx={{ p: 2.5 }}>
            <Box
              component="pre"
              sx={{
                m: 0,
                p: "12px 14px",
                bgcolor: "action.hover",
                border: 1,
                borderColor: "divider",
                borderRadius: 1,
                fontFamily: "monospace",
                fontSize: 12,
                color: "text.secondary",
                lineHeight: 1.65,
                overflowX: "auto",
              }}
            >
              {JSON.stringify(response.body, null, 2)}
            </Box>
          </Box>
        ) : (
          <Box sx={{ py: 6, px: 2.5, textAlign: "center" }}>
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ fontFamily: "monospace" }}
            >
              Click Send request to see the response
            </Typography>
          </Box>
        )}
      </Card>
    </Box>
  );
}

// ── Activity tab ──────────────────────────────────────────────────────────────

function ActivityTab() {
  const [entries, setEntries] = useState<ActivityEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (api as any)
      .GET("/v1/agents/activity", { params: { query: { limit: 50 } } })
      .then(({ data }: { data?: { entries: ActivityEntry[] } }) => {
        if (data?.entries) setEntries(data.entries);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  return (
    <Card variant="outlined">
      <Box sx={{ px: 2.5, py: 1.75, borderBottom: 1, borderColor: "divider" }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
          Recent activity
        </Typography>
      </Box>
      {loading ? (
        <Box sx={{ p: "24px 20px" }}>
          <Typography variant="body2" color="text.secondary">
            Loading…
          </Typography>
        </Box>
      ) : entries.length === 0 ? (
        <Box sx={{ p: "24px 20px" }}>
          <Typography variant="body2" color="text.secondary">
            No agent activity yet.
          </Typography>
        </Box>
      ) : (
        <Box sx={{ fontFamily: "monospace" }}>
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "160px 1fr 80px",
              px: 2.5,
              py: 1,
              borderBottom: 1,
              borderColor: "divider",
              bgcolor: "action.hover",
            }}
          >
            {["Time", "Tool", "Status"].map((h) => (
              <Typography
                key={h}
                variant="caption"
                color="text.secondary"
                sx={{
                  fontFamily: "monospace",
                  letterSpacing: 1.1,
                  textTransform: "uppercase",
                  fontSize: 10,
                }}
              >
                {h}
              </Typography>
            ))}
          </Box>
          {entries.map((e, i) => (
            <Box
              key={e.id}
              sx={{
                display: "grid",
                gridTemplateColumns: "160px 1fr 80px",
                px: 2.5,
                py: 1.5,
                borderBottom: i < entries.length - 1 ? 1 : 0,
                borderColor: "divider",
                alignItems: "center",
              }}
            >
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ fontFamily: "monospace", letterSpacing: 0.5 }}
              >
                {new Date(e.createdAt).toLocaleTimeString()}
              </Typography>
              <Typography
                variant="caption"
                color="primary.main"
                sx={{ fontFamily: "monospace" }}
              >
                {e.toolName}
              </Typography>
              <Typography
                variant="caption"
                sx={{
                  fontFamily: "monospace",
                  fontWeight: 600,
                  color: e.status < 400 ? "success.main" : "error.main",
                }}
              >
                {e.status}
              </Typography>
            </Box>
          ))}
        </Box>
      )}
    </Card>
  );
}

const codeStyle = {
  fontFamily: "monospace",
  fontSize: 12,
  bgcolor: "action.hover",
  border: 1,
  borderColor: "divider",
  borderRadius: 0.5,
  px: 0.625,
  py: 0.125,
  color: "text.primary",
} as const;
