"use client";

import React, { useState, useEffect } from "react";
import { formatDate, formatTime } from "@/utils/format";
import { copyToClipboard } from "@/utils/clipboard";
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
import TextField from "@mui/material/TextField";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogActions from "@mui/material/DialogActions";
import FormGroup from "@mui/material/FormGroup";
import FormControlLabel from "@mui/material/FormControlLabel";
import Checkbox from "@mui/material/Checkbox";
import IconButton from "@mui/material/IconButton";
import Tooltip from "@mui/material/Tooltip";
import Alert from "@mui/material/Alert";

// Icons
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import ContentCopyOutlinedIcon from "@mui/icons-material/ContentCopyOutlined";
import FileDownloadOutlinedIcon from "@mui/icons-material/FileDownloadOutlined";
import WarningAmberOutlinedIcon from "@mui/icons-material/WarningAmberOutlined";
import FlagOutlinedIcon from "@mui/icons-material/FlagOutlined";
import TagOutlinedIcon from "@mui/icons-material/TagOutlined";
import { PageShell } from "@/components/PageShell";
import { HighlightedCode } from "@/components/HighlightedCode";
import { CLIENT_PY } from "@/generated/clientSource";

interface AgentTokenSummary {
  id: string;
  name: string;
  scopes: string[];
  createdAt: string;
  expiresAt?: string | null;
  lastUsedAt?: string | null;
}

interface AgentSummary {
  id: string;
  label: string;
  tokens: AgentTokenSummary[];
  createdAt: string;
  focusTags: string[];
  currentGoal?: string | null;
  nextTarget?: string | null;
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
  agentName?: string;
  status: number;
  ts: string;
}

type TabId = "keys" | "tools" | "import" | "activity";

const SAMPLE_IMPORT = JSON.stringify(
  {
    title: "Mastery of Rust Ownership",
    description: "A deep dive into move semantics and borrowing.",
    mode: "graded",
    method: "agent",
    objectives: ["Understand Move semantics", "Differentiate &T and &mut T"],

    questions: [
      {
        kind: "mc",
        prompt: "Which keyword is used to transfer ownership in Rust?",
        payload: {
          options: ["copy", "clone", "move", "transfer"],
          correct_index: 2,
        },
        tags: ["rust", "ownership"],
      },
      {
        kind: "tf",
        prompt: "A value can have multiple mutable references at once.",
        payload: {
          correct: false,
        },
        tags: ["rust", "borrowing"],
      },
    ],
  },
  null,
  2,
);

export default function AgentPage() {
  const [tab, setTab] = useState<TabId>("keys");

  // API URL for copy-correct usage hints (set after mount — avoids SSR `window`,
  // which would crash prerender; falls back to localhost until hydrated).
  const [apiUrl, setApiUrl] = useState("");
  useEffect(() => {
    setApiUrl(
      process.env.NEXT_PUBLIC_API_URL ||
        `http://${window.location.hostname}:28080`,
    );
  }, []);

  return (
    <PageShell
      kicker="Programmatic surface · OpenAPI 3.1 · MCP-compatible"
      title="Agent integration"
      action={
        <Stack direction="row" spacing={1}>
          <Button
            variant="outlined"
            size="small"
            onClick={() => window.open("/llms.txt", "_blank")}
          >
            llms.txt
          </Button>
          <Button
            variant="outlined"
            size="small"
            onClick={() => window.open("/openapi.yaml", "_blank")}
          >
            OpenAPI
          </Button>
          <Button
            variant="outlined"
            size="small"
            onClick={() => window.open("/skill.json", "_blank")}
          >
            MCP manifest
          </Button>
        </Stack>
      }
    >
      {/* Intro card */}
      <Card
        variant="outlined"
        sx={{
          mb: 3.5,
          display: "grid",
          gridTemplateColumns: { xs: "1fr", md: "1.2fr 1fr" },
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
            Assessments, attempts, and rubrics are first-class API objects.
          </Typography>
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ lineHeight: 1.55, mb: 2.75, maxWidth: 540 }}
          >
            Every screen you see is backed by the same REST surface that agents
            use. A grading agent reads an attempt with one call; an authoring
            agent imports a new assessment with another. No scraping, no
            duplicate state.
          </Typography>
          <Stack direction="row" spacing={2.75} sx={{ flexWrap: "wrap" }}>
            <KV2 k="Endpoints" v="34" />
            <KV2 k="Auth" v="Bearer + scopes" />
            <KV2 k="Rate limit" v="60 - 600 / min" />
            <KV2 k="SDKs" v="ts · py" />
          </Stack>
        </Box>
        <Box
          sx={{
            bgcolor: "action.hover",
            borderLeft: 1,
            borderColor: "divider",
            p: "22px 26px",
            minWidth: 0,
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
            label="cURL request"
            lines={[
              `curl ${apiUrl || "http://localhost:28080"}/v1/assessments \\`,
              `  -H "Authorization: Bearer <your_agent_key>" \\`,
              `  -H "Content-Type: application/json"`,
              ``,
              `# → Response: 200 OK · 24 assessments`,
            ]}
            language="bash"
          />
        </Box>
      </Card>

      {/* Tabs */}
      <Tabs
        value={tab}
        onChange={(_, v) => setTab(v as TabId)}
        variant="scrollable"
        allowScrollButtonsMobile
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
    </PageShell>
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
  language,
  style,
}: {
  label?: string;
  lines: string[];
  language?: string;
  style?: React.CSSProperties;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    // Exclude response lines or comments starting with → or # →
    const copyText = lines
      .filter((l) => !l.trim().startsWith("→") && !l.trim().startsWith("# →"))
      .join("\n");
    copyToClipboard(copyText).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  const codeContent = lines.join("\n");
  const lang =
    language ||
    (label && label.toLowerCase().includes("py") ? "python" : "bash");

  return (
    <Paper
      variant="outlined"
      style={style}
      sx={{
        overflow: "hidden",
        borderRadius: 2,
        borderColor: "divider",
        boxShadow: "0 4px 12px rgba(0,0,0,0.03)",
      }}
    >
      {label && (
        <Box
          sx={{
            px: 2,
            py: 1,
            borderBottom: 1,
            borderColor: "divider",
            bgcolor: (theme) =>
              theme.palette.mode === "dark"
                ? "rgba(255,255,255,0.02)"
                : "rgba(0,0,0,0.01)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{
              fontFamily: "monospace",
              fontWeight: 600,
              letterSpacing: 0.5,
              textTransform: "uppercase",
            }}
          >
            {label}
          </Typography>
          <Button
            size="small"
            variant="text"
            onClick={handleCopy}
            startIcon={
              copied ? null : <ContentCopyOutlinedIcon sx={{ fontSize: 14 }} />
            }
            sx={{
              minWidth: 0,
              py: 0.5,
              px: 1.5,
              fontSize: 11,
              textTransform: "none",
              color: copied ? "success.main" : "text.secondary",
              "&:hover": {
                bgcolor: "action.hover",
              },
            }}
          >
            {copied ? "Copied! ✓" : "Copy"}
          </Button>
        </Box>
      )}
      <HighlightedCode code={codeContent} language={lang} />
    </Paper>
  );
}

// ── API Keys / Agents tab ──────────────────────────────────────────────────────

function KeysTab() {
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [loading, setLoading] = useState(true);

  // Create agent form state
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [newAgentLabel, setNewAgentLabel] = useState("");
  const [newAgentScopes, setNewAgentScopes] = useState<string[]>([
    "assessment.read",
  ]);
  const [newAgentFocus, setNewAgentFocus] = useState("");
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);

  // Edit agent modal state
  const [editAgent, setEditAgent] = useState<AgentSummary | null>(null);
  const [editLabel, setEditLabel] = useState("");
  const [editFocus, setEditFocus] = useState("");
  const [editGoal, setEditGoal] = useState("");
  const [editTarget, setEditTarget] = useState("");
  const [savingEdit, setSavingEdit] = useState(false);

  // Revoke agent modal state
  const [revokeAgent, setRevokeAgent] = useState<AgentSummary | null>(null);
  const [revoking, setRevoking] = useState(false);

  // Token management state
  const [createTokenAgent, setCreateTokenAgent] = useState<AgentSummary | null>(
    null,
  );
  const [newTokenName, setNewTokenName] = useState("");
  const [newTokenScopes, setNewTokenScopes] = useState<string[]>([
    "assessment.read",
  ]);
  const [generatingToken, setGeneratingToken] = useState(false);
  const [generateTokenError, setGenerateTokenError] = useState<string | null>(
    null,
  );
  const [revokeTokenAgentId, setRevokeTokenAgentId] = useState<string | null>(
    null,
  );
  const [revokeToken, setRevokeToken] = useState<AgentTokenSummary | null>(
    null,
  );
  const [revokingToken, setRevokingToken] = useState(false);

  // Origin for copy-correct usage hints (set after mount — avoids SSR `window`).
  const [origin, setOrigin] = useState("");
  useEffect(() => {
    setOrigin(window.location.origin);
  }, []);

  const downloadClient = () => {
    const blob = new Blob([CLIENT_PY], { type: "text/x-python" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "client.py";
    a.click();
    URL.revokeObjectURL(url);
  };

  const ALL_SCOPES = [
    {
      value: "assessment.read",
      label: "assessment.read",
      desc: "Read assessments and questions",
      tools: [
        "assessment.list",
        "assessment.get",
        "question.list",
        "activity.list",
      ],
    },
    {
      value: "assessment.write",
      label: "assessment.write",
      desc: "Create, edit, and archive assessments",
      tools: [
        "assessment.create",
        "assessment.batchCreate",
        "assessment.update",
        "assessment.addQuestion",
        "question.create",
        "question.promote",
      ],
    },
    {
      value: "attempt.read",
      label: "attempt.read",
      desc: "Read session attempt details",
      tools: ["attempt.list"],
    },
    {
      value: "attempt.write",
      label: "attempt.write",
      desc: "Answer questions and submit attempts",
      tools: ["attempt.grade"],
    },
    {
      value: "stats.read",
      label: "stats.read",
      desc: "Read metrics and user ELO",
      tools: ["assessment.stats", "stats.user"],
    },
  ];

  function loadAgents() {
    setLoading(true);
    api
      .GET("/v1/me/agents")
      .then(({ data }) => {
        if (data?.agents) {
          setAgents(data.agents as unknown as AgentSummary[]);
        }
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    loadAgents();
  }, []);

  async function createAgent() {
    if (!newAgentLabel.trim()) return;
    setCreating(true);
    setCreateError(null);
    try {
      const { data, error } = await api.POST("/v1/me/agents", {
        body: {
          label: newAgentLabel,
          scopes: newAgentScopes,
          focusTags: newAgentFocus
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
        },
      });
      if (error) {
        setCreateError((error as any)?.message || "Failed to create agent.");
      } else if (data?.apiKey) {
        setCreatedSecret(data.apiKey);
        setNewAgentLabel("");
        setNewAgentFocus("");
        setNewAgentScopes(["assessment.read"]);
        loadAgents();
      }
    } catch (err) {
      console.error(err);
      setCreateError("An unexpected error occurred.");
    } finally {
      setCreating(false);
    }
  }

  async function handleSaveEdit() {
    if (!editAgent) return;
    setSavingEdit(true);
    try {
      const res = await api.PATCH("/v1/me/agents/{id}", {
        params: {
          path: { id: editAgent.id },
        },
        body: {
          label: editLabel || undefined,
          focusTags: editFocus
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
          currentGoal: editGoal || "",
          nextTarget: editTarget || "",
        },
      });
      if (res.response.ok) {
        setEditAgent(null);
        loadAgents();
      }
    } catch (err) {
      console.error(err);
    } finally {
      setSavingEdit(false);
    }
  }

  async function handleConfirmRevoke() {
    if (!revokeAgent) return;
    setRevoking(true);
    try {
      const res = await api.DELETE("/v1/me/agents/{id}", {
        params: {
          path: { id: revokeAgent.id },
        },
      });
      if (res.response.ok) {
        setRevokeAgent(null);
        loadAgents();
      }
    } catch (err) {
      console.error(err);
    } finally {
      setRevoking(false);
    }
  }

  async function handleGenerateToken() {
    if (!createTokenAgent || !newTokenName.trim()) return;
    setGeneratingToken(true);
    setGenerateTokenError(null);
    try {
      const { data, error } = await api.POST("/v1/me/agents/{id}/tokens", {
        params: { path: { id: createTokenAgent.id } },
        body: {
          name: newTokenName,
          scopes: newTokenScopes,
        },
      });
      if (error) {
        setGenerateTokenError(
          (error as any)?.error?.message ||
            (error as any)?.message ||
            "Failed to generate token.",
        );
      } else if (data?.secret) {
        setCreatedSecret(data.secret);
        setCreateTokenAgent(null);
        setNewTokenName("");
        setNewTokenScopes(["assessment.read"]);
        loadAgents();
      }
    } catch (err) {
      console.error(err);
      setGenerateTokenError("An unexpected error occurred.");
    } finally {
      setGeneratingToken(false);
    }
  }

  async function handleConfirmRevokeToken() {
    if (!revokeToken || !revokeTokenAgentId) return;
    setRevokingToken(true);
    try {
      const res = await api.DELETE("/v1/me/keys/{id}", {
        params: {
          path: { id: revokeToken.id },
        },
      });
      if (res.response.ok) {
        setRevokeToken(null);
        setRevokeTokenAgentId(null);
        loadAgents();
      }
    } catch (err) {
      console.error(err);
    } finally {
      setRevokingToken(false);
    }
  }

  const handleToggleScope = (scope: string) => {
    setNewAgentScopes((prev) =>
      prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope],
    );
  };

  const handleToggleTokenScope = (scope: string) => {
    setNewTokenScopes((prev) =>
      prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope],
    );
  };

  const handleOpenEdit = (agent: AgentSummary) => {
    setEditAgent(agent);
    setEditLabel(agent.label);
    setEditFocus((agent.focusTags || []).join(", "));
    setEditGoal(agent.currentGoal || "");
    setEditTarget(agent.nextTarget || "");
  };

  return (
    <Box
      sx={{
        display: "grid",
        gridTemplateColumns: { xs: "1fr", md: "1.4fr 1fr" },
        gap: 3.5,
        alignItems: "start",
      }}
    >
      {/* Agents list */}
      <Stack spacing={2.5} sx={{ minWidth: 0 }}>
        <Card variant="outlined">
          <Box
            sx={{
              px: 2.5,
              py: 2,
              borderBottom: 1,
              borderColor: "divider",
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
              Agent Sub-Accounts
            </Typography>
            <Chip label={agents.length} size="small" color="primary" />
          </Box>

          {loading ? (
            <Box sx={{ p: 4, display: "flex", justifyContent: "center" }}>
              <CircularProgress size={24} />
            </Box>
          ) : agents.length === 0 ? (
            <Box sx={{ p: 4, textAlign: "center" }}>
              <Typography variant="body2" color="text.secondary">
                No agent keys created yet. Configure your first agent
                sub-account on the right.
              </Typography>
            </Box>
          ) : (
            <Stack
              divider={<Box sx={{ borderBottom: 1, borderColor: "divider" }} />}
            >
              {agents.map((agent) => (
                <Box key={agent.id} sx={{ p: 2.5 }}>
                  <Box
                    sx={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "flex-start",
                      mb: 1.5,
                    }}
                  >
                    <Box
                      sx={{ display: "flex", gap: 1.5, alignItems: "center" }}
                    >
                      <Paper
                        sx={{
                          width: 40,
                          height: 40,
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          bgcolor: (theme) =>
                            theme.palette.mode === "dark"
                              ? "rgba(25, 118, 210, 0.16)"
                              : "rgba(25, 118, 210, 0.08)",
                          color: "primary.main",
                          borderRadius: "10px",
                        }}
                      >
                        <SmartToyOutlinedIcon />
                      </Paper>
                      <Box>
                        <Typography
                          variant="body2"
                          sx={{ fontWeight: 600, fontSize: 15 }}
                        >
                          {agent.label}
                        </Typography>
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ display: "block", mt: 0.25 }}
                        >
                          Created on {formatDate(agent.createdAt)}
                        </Typography>
                      </Box>
                    </Box>

                    <Stack direction="row" spacing={0.5}>
                      <Tooltip title="Edit Profile & Focus">
                        <IconButton
                          size="small"
                          onClick={() => handleOpenEdit(agent)}
                        >
                          <EditOutlinedIcon fontSize="small" />
                        </IconButton>
                      </Tooltip>
                      <Tooltip title="Revoke Programmatic Access">
                        <IconButton
                          size="small"
                          color="error"
                          onClick={() => setRevokeAgent(agent)}
                        >
                          <DeleteOutlineIcon fontSize="small" />
                        </IconButton>
                      </Tooltip>
                    </Stack>
                  </Box>

                  {/* Tokens */}
                  <Box sx={{ mb: 2 }}>
                    <Box
                      sx={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                        mb: 1,
                      }}
                    >
                      <Typography
                        variant="caption"
                        sx={{ fontWeight: 600, color: "text.secondary" }}
                      >
                        ACTIVE TOKENS
                      </Typography>
                      <Button
                        size="small"
                        variant="outlined"
                        onClick={() => setCreateTokenAgent(agent)}
                        sx={{ fontSize: 11, textTransform: "none", py: 0 }}
                      >
                        Generate Token
                      </Button>
                    </Box>
                    {agent.tokens.length === 0 ? (
                      <Typography variant="caption" color="text.secondary">
                        No active tokens
                      </Typography>
                    ) : (
                      agent.tokens.map((token) => (
                        <Paper
                          key={token.id}
                          variant="outlined"
                          sx={{ p: 1.5, mb: 1, borderRadius: 1 }}
                        >
                          <Box
                            sx={{
                              display: "flex",
                              justifyContent: "space-between",
                              alignItems: "flex-start",
                              mb: 1,
                            }}
                          >
                            <Box>
                              <Typography
                                variant="body2"
                                sx={{ fontWeight: 600 }}
                              >
                                {token.name}
                              </Typography>
                              <Typography
                                variant="caption"
                                color="text.secondary"
                                sx={{ display: "block" }}
                              >
                                Created: {formatDate(token.createdAt)}
                              </Typography>
                              <Typography
                                variant="caption"
                                color="text.secondary"
                                sx={{ display: "block" }}
                              >
                                {token.expiresAt
                                  ? `Expires: ${formatDate(token.expiresAt)}`
                                  : "No expiry"}
                              </Typography>
                            </Box>
                            <Tooltip title="Revoke Token">
                              <IconButton
                                size="small"
                                color="error"
                                onClick={() => {
                                  setRevokeTokenAgentId(agent.id);
                                  setRevokeToken(token);
                                }}
                              >
                                <DeleteOutlineIcon fontSize="small" />
                              </IconButton>
                            </Tooltip>
                          </Box>
                          <Box
                            sx={{
                              display: "flex",
                              flexWrap: "wrap",
                              gap: 0.75,
                            }}
                          >
                            {token.scopes.map((scope) => (
                              <Chip
                                key={scope}
                                label={scope}
                                size="small"
                                color={
                                  scope === "admin" ? "warning" : "default"
                                }
                                variant="outlined"
                                sx={{ fontSize: 11, height: 22 }}
                              />
                            ))}
                          </Box>
                        </Paper>
                      ))
                    )}
                  </Box>

                  {/* Focus Tags */}
                  {agent.focusTags && agent.focusTags.length > 0 && (
                    <Box
                      sx={{
                        mb: 1.5,
                        display: "flex",
                        alignItems: "center",
                        gap: 1,
                      }}
                    >
                      <TagOutlinedIcon
                        sx={{ fontSize: 16, color: "text.secondary" }}
                      />
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ fontWeight: 500, mr: 0.5 }}
                      >
                        Focus:
                      </Typography>
                      <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.5 }}>
                        {agent.focusTags.map((tag) => (
                          <Chip
                            key={tag}
                            label={tag}
                            size="small"
                            variant="outlined"
                            sx={{
                              height: 18,
                              fontSize: 10,
                              borderColor: "primary.light",
                              color: "primary.main",
                              bgcolor: (theme) =>
                                theme.palette.mode === "dark"
                                  ? "rgba(25, 118, 210, 0.16)"
                                  : "rgba(25, 118, 210, 0.08)",
                            }}
                          />
                        ))}
                      </Box>
                    </Box>
                  )}

                  {/* Goal & Trajectory (if exists) */}
                  {(agent.currentGoal || agent.nextTarget) && (
                    <Paper
                      variant="outlined"
                      sx={{
                        p: 1.5,
                        bgcolor: "action.hover",
                        borderColor: "divider",
                        borderRadius: 1,
                        fontSize: 12,
                      }}
                    >
                      {agent.currentGoal && (
                        <Box
                          sx={{
                            display: "flex",
                            gap: 1,
                            alignItems: "flex-start",
                            mb: agent.nextTarget ? 1 : 0,
                          }}
                        >
                          <FlagOutlinedIcon
                            sx={{
                              fontSize: 16,
                              color: "warning.main",
                              mt: 0.25,
                            }}
                          />
                          <Box>
                            <Typography
                              variant="caption"
                              color="text.secondary"
                              sx={{ display: "block", fontWeight: 600 }}
                            >
                              Current Goal
                            </Typography>
                            <Typography
                              variant="body2"
                              sx={{
                                fontSize: 12,
                                mt: 0.25,
                                whiteSpace: "pre-wrap",
                              }}
                            >
                              {agent.currentGoal}
                            </Typography>
                          </Box>
                        </Box>
                      )}
                      {agent.nextTarget && (
                        <Box
                          sx={{
                            display: "flex",
                            gap: 1,
                            alignItems: "flex-start",
                          }}
                        >
                          <TagOutlinedIcon
                            sx={{
                              fontSize: 16,
                              color: "success.main",
                              mt: 0.25,
                            }}
                          />
                          <Box>
                            <Typography
                              variant="caption"
                              color="text.secondary"
                              sx={{ display: "block", fontWeight: 600 }}
                            >
                              Next Target
                            </Typography>
                            <Typography
                              variant="body2"
                              sx={{ fontSize: 12, mt: 0.25 }}
                            >
                              {agent.nextTarget}
                            </Typography>
                          </Box>
                        </Box>
                      )}
                    </Paper>
                  )}
                </Box>
              ))}
            </Stack>
          )}
        </Card>
      </Stack>

      {/* Create agent column */}
      <Stack spacing={2.5} sx={{ minWidth: 0 }}>
        <Card variant="outlined" sx={{ p: 3 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 0.5 }}>
            Create Agent Sub-Account
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
            Assign a label, target focus tags, and specific capability scopes to
            authorize an autonomous sub-account.
          </Typography>

          {createError && (
            <Alert severity="error" sx={{ mb: 2.5 }}>
              {createError}
            </Alert>
          )}

          <Stack spacing={2.5}>
            <TextField
              label="Agent Label"
              placeholder="e.g. Grader Agent"
              size="small"
              value={newAgentLabel}
              onChange={(e) => setNewAgentLabel(e.target.value)}
              fullWidth
              required
            />

            <TextField
              label="Focus Tags (comma-separated)"
              placeholder="e.g. rust, math, logic"
              size="small"
              value={newAgentFocus}
              onChange={(e) => setNewAgentFocus(e.target.value)}
              fullWidth
            />

            <Box>
              <Box
                sx={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  mb: 1.25,
                }}
              >
                <Typography variant="body2" sx={{ fontWeight: 600 }}>
                  Select Capability Scopes
                </Typography>
                <Box sx={{ display: "flex", gap: 0.5 }}>
                  <Button
                    size="small"
                    onClick={() =>
                      setNewAgentScopes(ALL_SCOPES.map((s) => s.value))
                    }
                    disabled={newAgentScopes.length === ALL_SCOPES.length}
                  >
                    Select all
                  </Button>
                  <Button
                    size="small"
                    color="inherit"
                    onClick={() => setNewAgentScopes([])}
                    disabled={newAgentScopes.length === 0}
                  >
                    Clear
                  </Button>
                </Box>
              </Box>
              <FormGroup>
                <Box
                  sx={{
                    display: "grid",
                    gridTemplateColumns: "1fr",
                    gap: 1.25,
                  }}
                >
                  {ALL_SCOPES.map((s) => (
                    <Box
                      key={s.value}
                      sx={{
                        display: "flex",
                        alignItems: "flex-start",
                        p: 1,
                        borderRadius: 1,
                        border: 1,
                        borderColor: newAgentScopes.includes(s.value)
                          ? "primary.light"
                          : "divider",
                        bgcolor: newAgentScopes.includes(s.value)
                          ? (theme) =>
                              theme.palette.mode === "dark"
                                ? "rgba(25, 118, 210, 0.16)"
                                : "rgba(25, 118, 210, 0.08)"
                          : "transparent",
                        transition: "all 0.2s",
                        "&:hover": { borderColor: "primary.main" },
                      }}
                    >
                      <Checkbox
                        size="small"
                        checked={newAgentScopes.includes(s.value)}
                        onChange={() => handleToggleScope(s.value)}
                        sx={{ mt: -0.5 }}
                      />
                      <Box
                        sx={{ cursor: "pointer" }}
                        onClick={() => handleToggleScope(s.value)}
                      >
                        <Typography
                          variant="caption"
                          sx={{
                            fontFamily: "monospace",
                            fontWeight: 600,
                            display: "block",
                          }}
                        >
                          {s.label}
                        </Typography>
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ fontSize: 11 }}
                        >
                          {s.desc}
                        </Typography>
                        <Box
                          sx={{
                            display: "flex",
                            flexWrap: "wrap",
                            gap: 0.5,
                            mt: 0.75,
                          }}
                        >
                          {s.tools.map((tool) => (
                            <Chip
                              key={tool}
                              size="small"
                              variant="outlined"
                              label={tool}
                              sx={{
                                fontFamily: "monospace",
                                fontSize: 10,
                                height: 18,
                              }}
                            />
                          ))}
                        </Box>
                      </Box>
                    </Box>
                  ))}
                </Box>
              </FormGroup>
              <Box
                sx={{ mt: 1.5, pt: 1.25, borderTop: 1, borderColor: "divider" }}
              >
                <Typography
                  variant="caption"
                  sx={{ fontWeight: 600, display: "block", mb: 0.5 }}
                >
                  Always available (no scope required)
                </Typography>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ fontSize: 11, display: "block", mb: 0.75 }}
                >
                  Every agent token can manage its own profile and memory.
                </Typography>
                <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.5 }}>
                  {[
                    "profile.get",
                    "memory.set",
                    "memory.append",
                    "target.set",
                  ].map((tool) => (
                    <Chip
                      key={tool}
                      size="small"
                      variant="outlined"
                      label={tool}
                      sx={{ fontFamily: "monospace", fontSize: 10, height: 18 }}
                    />
                  ))}
                </Box>
              </Box>
            </Box>

            <Button
              variant="contained"
              disabled={creating || !newAgentLabel.trim()}
              onClick={createAgent}
              fullWidth
            >
              {creating ? <CircularProgress size={20} /> : "+ Create Agent"}
            </Button>
          </Stack>
        </Card>

        {/* Right side info panel */}
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
            Agent Authentication
          </Typography>
          <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 1 }}>
            Secure bearer authorization
          </Typography>
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ lineHeight: 1.55 }}
          >
            Send the key in an{" "}
            <Box component="code" sx={codeStyle}>
              Authorization
            </Box>{" "}
            header. Sub-accounts act within their restricted scopes and plan
            ceilings.
          </Typography>
        </Card>

        {/* Onboarding Guide Card */}
        <Card variant="outlined" sx={{ p: 2.5 }}>
          <Typography
            variant="caption"
            color="secondary.main"
            sx={{
              fontFamily: "monospace",
              letterSpacing: 1.3,
              textTransform: "uppercase",
              display: "block",
              mb: 1.25,
              fontWeight: 600,
            }}
          >
            Zero-Knowledge Discovery
          </Typography>
          <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 1 }}>
            How do agents learn AME?
          </Typography>
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ lineHeight: 1.55, mb: 2 }}
          >
            You don't need to manually teach external agents. AME exposes
            public-facing discovery endpoints that allow any LLM assistant to
            learn all API scopes, structures, and workflows dynamically:
          </Typography>
          <Stack spacing={1.5} sx={{ mb: 3 }}>
            {[
              {
                num: "1",
                name: "/llms.txt",
                title: "LLM Developer Specs",
                desc: "A structured, agent-optimized overview of architecture, scopes, data models, and step-by-step diagnostic recipes.",
                url: "/llms.txt",
              },
              {
                num: "2",
                name: "/skill.json",
                title: "MCP Tool Manifest",
                desc: "A machine-readable, Model Context Protocol compatible tool schema mapping capabilities to REST endpoints.",
                url: "/skill.json",
              },
              {
                num: "3",
                name: "/openapi.yaml",
                title: "OpenAPI Spec",
                desc: "Standard OpenAPI 3.1 schema snapshot for direct client code generation and API broker orchestration.",
                url: "/openapi.yaml",
              },
            ].map((item) => (
              <Paper
                key={item.num}
                variant="outlined"
                sx={{
                  p: 2,
                  borderRadius: 2,
                  borderColor: "divider",
                  transition: "all 0.2s ease",
                  display: "flex",
                  gap: 2,
                  alignItems: "flex-start",
                  "&:hover": {
                    borderColor: "primary.main",
                    bgcolor: (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(25, 118, 210, 0.04)"
                        : "rgba(25, 118, 210, 0.01)",
                  },
                }}
              >
                <Paper
                  sx={{
                    width: 28,
                    height: 28,
                    borderRadius: "50%",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    bgcolor: (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(25, 118, 210, 0.2)"
                        : "rgba(25, 118, 210, 0.08)",
                    color: "primary.main",
                    fontWeight: 600,
                    fontSize: 13,
                    flexShrink: 0,
                  }}
                >
                  {item.num}
                </Paper>
                <Box sx={{ flexGrow: 1 }}>
                  <Box
                    sx={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      mb: 0.5,
                    }}
                  >
                    <Typography
                      variant="body2"
                      sx={{
                        fontWeight: 600,
                        fontFamily: "monospace",
                        fontSize: 13,
                      }}
                    >
                      {item.name}
                    </Typography>
                    <Button
                      size="small"
                      variant="text"
                      onClick={() => window.open(item.url, "_blank")}
                      sx={{
                        fontSize: 11,
                        textTransform: "none",
                        py: 0.25,
                        px: 1,
                        minWidth: 0,
                      }}
                    >
                      Open ↗
                    </Button>
                  </Box>
                  <Typography
                    variant="body2"
                    sx={{ fontWeight: 600, mb: 0.5, fontSize: 12.5 }}
                  >
                    {item.title}
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: "block", lineHeight: 1.4 }}
                  >
                    {item.desc}
                  </Typography>
                </Box>
              </Paper>
            ))}
          </Stack>

          <Box sx={{ mt: 2.5 }}>
            <Box
              sx={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                mb: 0.75,
              }}
            >
              <Typography
                variant="caption"
                sx={{ fontFamily: "monospace", fontWeight: 600 }}
              >
                Reference client — agents/client.py
              </Typography>
              <Button
                size="small"
                variant="outlined"
                startIcon={<FileDownloadOutlinedIcon />}
                onClick={downloadClient}
              >
                Download
              </Button>
            </Box>
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ display: "block", mb: 1.25, lineHeight: 1.3 }}
            >
              A dependency-free Python wrapper and the canonical worked example.
              Download it, then run the end-to-end demo against this host (or
              import{" "}
              <Box component="code" sx={codeStyle}>
                AmeAgent
              </Box>{" "}
              as a library):
            </Typography>
            <Box sx={{ mb: 1.25 }}>
              <CodeBlock
                label="Run the Demo"
                lines={[
                  `uv run client.py --api ${origin || "http://localhost:28080"} --token <your_auth_token>`,
                ]}
                language="bash"
              />
            </Box>
            <CodeBlock
              label="agents/client.py"
              lines={CLIENT_PY.replace(/\n$/, "").split("\n")}
            />
          </Box>
        </Card>
      </Stack>

      {/* Secret Shown Once Modal */}
      {createdSecret && (
        <Dialog
          open={!!createdSecret}
          onClose={() => setCreatedSecret(null)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle sx={{ fontWeight: 600 }}>
            API Token Generated Successfully
          </DialogTitle>
          <DialogContent>
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ mb: 1.5, lineHeight: 1.6 }}
            >
              Copy this token secret now. **For security, it will never be
              displayed again.**
            </Typography>
            <Box
              sx={{
                p: 2,
                bgcolor: "action.hover",
                border: 1,
                borderColor: "primary.main",
                borderRadius: 1,
                fontFamily: "monospace",
                fontSize: 13,
                color: "primary.main",
                wordBreak: "break-all",
                mb: 3,
                userSelect: "all",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <Box
                component="span"
                sx={{ overflow: "hidden", textOverflow: "ellipsis" }}
              >
                {createdSecret}
              </Box>
              <Button
                size="small"
                onClick={() => copyToClipboard(createdSecret)}
                sx={{ minWidth: 0, ml: 1, p: 0.5 }}
              >
                Copy
              </Button>
            </Box>

            <Box sx={{ borderTop: 1, borderColor: "divider", pt: 2.5 }}>
              <Chip
                label="Agent UX"
                color="secondary"
                size="small"
                sx={{ mb: 1.25, fontWeight: 600, fontSize: 10, height: 20 }}
              />
              <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 0.5 }}>
                🚀 AI Assistant Boot Prompt
              </Typography>
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ display: "block", mb: 2, lineHeight: 1.4 }}
              >
                Copy and paste this starter prompt directly into your AI chat
                window (Claude, ChatGPT, Gemini, etc.). It points the agent
                directly to AME's public discovery schemas, enabling it to learn
                the platform from scratch and start helping you grow.
              </Typography>
              <Paper
                variant="outlined"
                sx={{
                  p: 2,
                  bgcolor: "action.hover",
                  fontFamily: "monospace",
                  fontSize: 11,
                  lineHeight: 1.5,
                  maxHeight: 180,
                  overflowY: "auto",
                  color: "text.secondary",
                  whiteSpace: "pre-wrap",
                  borderRadius: 1,
                  mb: 1,
                }}
              >
                {`You are an AI assistant helping me with my study on AME. AME has a first-class agent surface. To learn how to use it, please fetch and read the platform capabilities at:
${origin || "http://localhost:23000"}/llms.txt

The machine-readable tool schemas are available at:
${origin || "http://localhost:23000"}/skill.json

Authenticate all your requests using this API Key:
Bearer ${createdSecret}

Your first task is to read my learning stats at /v1/me/stats, identify my weakest topics, and create a targeted practice assessment to help me master them!`}
              </Paper>
            </Box>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2.5 }}>
            <Button
              variant="outlined"
              size="small"
              onClick={() => {
                const promptText = `You are an AI assistant helping me with my study on AME. AME has a first-class agent surface. To learn how to use it, please fetch and read the platform capabilities at:
${origin || "http://localhost:23000"}/llms.txt

The machine-readable tool schemas are available at:
${origin || "http://localhost:23000"}/skill.json

Authenticate all your requests using this API Key:
Bearer ${createdSecret}

Your first task is to read my learning stats at /v1/me/stats, identify my weakest topics, and create a targeted practice assessment to help me master them!`;
                copyToClipboard(promptText);
              }}
              startIcon={<ContentCopyOutlinedIcon />}
              sx={{ mr: "auto" }}
            >
              Copy Boot Prompt
            </Button>
            <Button
              variant="contained"
              size="small"
              onClick={() => setCreatedSecret(null)}
            >
              Done
            </Button>
          </DialogActions>
        </Dialog>
      )}

      {/* Edit agent profile dialog */}
      {editAgent && (
        <Dialog
          open={!!editAgent}
          onClose={() => setEditAgent(null)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle sx={{ fontWeight: 600 }}>
            Edit Agent Profile & Focus
          </DialogTitle>
          <DialogContent>
            <Stack spacing={2.5} sx={{ mt: 1.5 }}>
              <TextField
                label="Agent Label"
                size="small"
                value={editLabel}
                onChange={(e) => setEditLabel(e.target.value)}
                fullWidth
                required
              />
              <TextField
                label="Focus Tags (comma-separated)"
                placeholder="e.g. rust, mathematics"
                size="small"
                value={editFocus}
                onChange={(e) => setEditFocus(e.target.value)}
                fullWidth
              />
              <TextField
                label="Current Goal"
                placeholder="Describe current task goals..."
                size="small"
                multiline
                rows={2}
                value={editGoal}
                onChange={(e) => setEditGoal(e.target.value)}
                fullWidth
              />
              <TextField
                label="Next Target"
                placeholder="e.g. Solve weakest ELO tag"
                size="small"
                value={editTarget}
                onChange={(e) => setEditTarget(e.target.value)}
                fullWidth
              />
            </Stack>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2.5 }}>
            <Button
              variant="outlined"
              size="small"
              onClick={() => setEditAgent(null)}
            >
              Cancel
            </Button>
            <Button
              variant="contained"
              size="small"
              onClick={handleSaveEdit}
              disabled={savingEdit || !editLabel.trim()}
            >
              {savingEdit ? <CircularProgress size={16} /> : "Save Changes"}
            </Button>
          </DialogActions>
        </Dialog>
      )}

      {/* Revoke confirmation dialog */}
      {revokeAgent && (
        <Dialog
          open={!!revokeAgent}
          onClose={() => setRevokeAgent(null)}
          maxWidth="xs"
          fullWidth
        >
          <DialogTitle
            sx={{
              fontWeight: 600,
              display: "flex",
              gap: 1,
              alignItems: "center",
            }}
          >
            <WarningAmberOutlinedIcon color="error" />
            Delete Agent?
          </DialogTitle>
          <DialogContent>
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ lineHeight: 1.6 }}
            >
              Are you sure you want to delete the agent **
              {revokeAgent.label}**? This action is permanent. All active tokens
              will be revoked and the agent's memory will be lost.
            </Typography>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2.5 }}>
            <Button
              variant="outlined"
              size="small"
              onClick={() => setRevokeAgent(null)}
            >
              Cancel
            </Button>
            <Button
              variant="contained"
              size="small"
              color="error"
              onClick={handleConfirmRevoke}
              disabled={revoking}
            >
              {revoking ? <CircularProgress size={16} /> : "Delete Agent"}
            </Button>
          </DialogActions>
        </Dialog>
      )}

      {/* Generate token dialog */}
      {createTokenAgent && (
        <Dialog
          open={!!createTokenAgent}
          onClose={() => setCreateTokenAgent(null)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle sx={{ fontWeight: 600 }}>Generate API Token</DialogTitle>
          <DialogContent>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              Generate a new API token for **{createTokenAgent.label}**. Tokens
              automatically expire after 7 days.
            </Typography>

            {generateTokenError && (
              <Alert severity="error" sx={{ mb: 2 }}>
                {generateTokenError}
              </Alert>
            )}

            <Stack spacing={2.5} sx={{ mt: 1 }}>
              <TextField
                label="Token Name"
                placeholder="e.g. CI workflow"
                size="small"
                value={newTokenName}
                onChange={(e) => setNewTokenName(e.target.value)}
                fullWidth
                required
              />
              <Box>
                <Typography
                  variant="caption"
                  sx={{ fontWeight: 600, display: "block", mb: 1 }}
                >
                  Token Scopes
                </Typography>
                <FormGroup
                  sx={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: 1,
                  }}
                >
                  {ALL_SCOPES.map((scope) => (
                    <FormControlLabel
                      key={scope.value}
                      control={
                        <Checkbox
                          size="small"
                          checked={newTokenScopes.includes(scope.value)}
                          onChange={() => handleToggleTokenScope(scope.value)}
                          color={
                            scope.value === "admin" ? "warning" : "primary"
                          }
                        />
                      }
                      label={
                        <Box>
                          <Typography variant="body2" sx={{ fontWeight: 500 }}>
                            {scope.label}
                          </Typography>
                          <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{ display: "block", lineHeight: 1.2 }}
                          >
                            {scope.desc}
                          </Typography>
                        </Box>
                      }
                      sx={{ alignItems: "flex-start", m: 0 }}
                    />
                  ))}
                </FormGroup>
              </Box>
            </Stack>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2.5 }}>
            <Button
              variant="outlined"
              size="small"
              onClick={() => setCreateTokenAgent(null)}
            >
              Cancel
            </Button>
            <Button
              variant="contained"
              size="small"
              onClick={handleGenerateToken}
              disabled={generatingToken || !newTokenName.trim()}
            >
              {generatingToken ? (
                <CircularProgress size={16} />
              ) : (
                "Generate Token"
              )}
            </Button>
          </DialogActions>
        </Dialog>
      )}

      {/* Revoke token dialog */}
      {revokeToken && (
        <Dialog
          open={!!revokeToken}
          onClose={() => {
            setRevokeToken(null);
            setRevokeTokenAgentId(null);
          }}
          maxWidth="xs"
          fullWidth
        >
          <DialogTitle
            sx={{
              fontWeight: 600,
              display: "flex",
              gap: 1,
              alignItems: "center",
            }}
          >
            <WarningAmberOutlinedIcon color="error" />
            Revoke Token?
          </DialogTitle>
          <DialogContent>
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ lineHeight: 1.6 }}
            >
              Are you sure you want to revoke the token **{revokeToken.name}**?
              Any API requests using this token will immediately begin to fail.
            </Typography>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2.5 }}>
            <Button
              variant="outlined"
              size="small"
              onClick={() => {
                setRevokeToken(null);
                setRevokeTokenAgentId(null);
              }}
            >
              Cancel
            </Button>
            <Button
              variant="contained"
              size="small"
              color="error"
              onClick={handleConfirmRevokeToken}
              disabled={revokingToken}
            >
              {revokingToken ? <CircularProgress size={16} /> : "Revoke Token"}
            </Button>
          </DialogActions>
        </Dialog>
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
      .GET("/skill.json")
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
    <Box
      sx={{
        display: "grid",
        gridTemplateColumns: { xs: "1fr", md: "280px 1fr" },
        gap: 2.25,
      }}
    >
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
                bgcolor: sel
                  ? (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(25, 118, 210, 0.16)"
                        : "rgba(25, 118, 210, 0.08)"
                  : "transparent",
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
            <HighlightedCode
              code={JSON.stringify(tool, null, 2)}
              language="json"
            />
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
      } = await client.POST("/v1/assessments", { body });
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
    <Box
      sx={{
        display: "grid",
        gridTemplateColumns: { xs: "1fr", md: "1.1fr 1fr" },
        gap: 2.25,
      }}
    >
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
              assessment.import
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
            POST /v1/assessments · Bearer hk_live_…
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
            <HighlightedCode
              code={JSON.stringify(response.body, null, 2)}
              language="json"
            />
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
      .then(({ data }: { data?: { items: ActivityEntry[] } }) => {
        if (data?.items) setEntries(data.items);
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
        <Box sx={{ fontFamily: "monospace", overflowX: "auto" }}>
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "160px 240px 1fr 80px",
              px: 2.5,
              py: 1,
              borderBottom: 1,
              borderColor: "divider",
              bgcolor: "action.hover",
              minWidth: 640,
            }}
          >
            {["Time", "Agent", "Tool", "Status"].map((h) => (
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
                gridTemplateColumns: "160px 240px 1fr 80px",
                px: 2.5,
                py: 1.5,
                borderBottom: i < entries.length - 1 ? 1 : 0,
                borderColor: "divider",
                alignItems: "center",
                minWidth: 640,
              }}
            >
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ fontFamily: "monospace", letterSpacing: 0.5 }}
              >
                {formatTime(e.ts)}
              </Typography>
              <Tooltip
                title={e.agentName || (e.agentId ? e.agentId : "System")}
              >
                <Typography
                  variant="caption"
                  color="text.primary"
                  sx={{
                    fontFamily: "monospace",
                    fontWeight: 500,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    pr: 1,
                  }}
                >
                  {e.agentName ||
                    (e.agentId ? `${e.agentId.substring(0, 8)}…` : "System")}
                </Typography>
              </Tooltip>
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
