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

// Icons
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import ContentCopyOutlinedIcon from "@mui/icons-material/ContentCopyOutlined";
import WarningAmberOutlinedIcon from "@mui/icons-material/WarningAmberOutlined";
import FlagOutlinedIcon from "@mui/icons-material/FlagOutlined";
import TagOutlinedIcon from "@mui/icons-material/TagOutlined";

interface AgentSummary {
  id: string;
  label: string;
  scopes: string[];
  createdAt: string;
  lastUsedAt?: string | null;
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
  status: number;
  createdAt: string;
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
              `curl https://api.ame-platform.app/v1/assessments \\`,
              `  -H "Authorization: Bearer hk_agent_3fY9…ax2P" \\`,
              `  -H "Content-Type: application/json"`,
              ``,
              `→ 200 OK · 24 assessments`,
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

// ── API Keys / Agents tab ──────────────────────────────────────────────────────

function KeysTab() {
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [loading, setLoading] = useState(true);

  // Create agent form state
  const [creating, setCreating] = useState(false);
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

  const ALL_SCOPES = [
    {
      value: "assessment.read",
      label: "assessment.read",
      desc: "Read assessments and questions",
    },
    {
      value: "assessment.write",
      label: "assessment.write",
      desc: "Create, edit, and archive assessments",
    },
    {
      value: "attempt.read",
      label: "attempt.read",
      desc: "Read session attempt details",
    },
    {
      value: "attempt.write",
      label: "attempt.write",
      desc: "Answer questions and submit attempts",
    },
    {
      value: "stats.read",
      label: "stats.read",
      desc: "Read metrics and user ELO",
    },
    {
      value: "feedback.write",
      label: "feedback.write",
      desc: "Submit essay grades and notes",
    },
    {
      value: "plan.read",
      label: "plan.read",
      desc: "Read recommended study plans",
    },
    {
      value: "plan.write",
      label: "plan.write",
      desc: "Generate and configure study plans",
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
    try {
      const { data } = await api.POST("/v1/me/agents", {
        body: {
          label: newAgentLabel,
          scopes: newAgentScopes,
          focusTags: newAgentFocus
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean),
        },
      });
      if (data?.apiKey) {
        setCreatedSecret(data.apiKey);
        setNewAgentLabel("");
        setNewAgentFocus("");
        setNewAgentScopes(["assessment.read"]);
        loadAgents();
      }
    } catch (err) {
      console.error(err);
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

  const handleToggleScope = (scope: string) => {
    setNewAgentScopes((prev) =>
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
        gridTemplateColumns: "1.4fr 1fr",
        gap: 3.5,
        alignItems: "start",
      }}
    >
      {/* Agents list */}
      <Stack spacing={2.5}>
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
                          bgcolor: "primary.50",
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
                          Created on{" "}
                          {new Date(agent.createdAt).toLocaleDateString()} ·
                          Last used{" "}
                          {agent.lastUsedAt
                            ? new Date(agent.lastUsedAt).toLocaleDateString()
                            : "never"}
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

                  {/* Scopes */}
                  <Box
                    sx={{
                      display: "flex",
                      flexWrap: "wrap",
                      gap: 0.75,
                      mb: 1.5,
                    }}
                  >
                    {agent.scopes.map((scope) => (
                      <Chip
                        key={scope}
                        label={scope}
                        size="small"
                        color={scope === "admin" ? "warning" : "default"}
                        variant="outlined"
                        sx={{ fontSize: 11, height: 22 }}
                      />
                    ))}
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
                              bgcolor: "primary.50",
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
      <Stack spacing={2.5}>
        <Card variant="outlined" sx={{ p: 3 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 0.5 }}>
            Create Agent Sub-Account
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
            Assign a label, target focus tags, and specific capability scopes to
            authorize an autonomous sub-account.
          </Typography>

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
              <Typography variant="body2" sx={{ fontWeight: 600, mb: 1.25 }}>
                Select Capability Scopes
              </Typography>
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
                          ? "primary.50"
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
                      </Box>
                    </Box>
                  ))}
                </Box>
              </FormGroup>
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
      </Stack>

      {/* Secret Shown Once Modal */}
      {createdSecret && (
        <Dialog
          open={!!createdSecret}
          onClose={() => setCreatedSecret(null)}
          maxWidth="xs"
          fullWidth
        >
          <DialogTitle sx={{ fontWeight: 600 }}>
            API Token Generated
          </DialogTitle>
          <DialogContent>
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ mb: 2, lineHeight: 1.6 }}
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
                mb: 1,
                userSelect: "all",
              }}
            >
              {createdSecret}
            </Box>
          </DialogContent>
          <DialogActions sx={{ px: 3, pb: 2.5 }}>
            <Button
              variant="outlined"
              size="small"
              onClick={() =>
                navigator.clipboard.writeText(createdSecret).catch(() => {})
              }
              startIcon={<ContentCopyOutlinedIcon />}
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
            Revoke Agent Key?
          </DialogTitle>
          <DialogContent>
            <Typography
              variant="body2"
              color="text.secondary"
              sx={{ lineHeight: 1.6 }}
            >
              Are you sure you want to revoke programmatic access for **
              {revokeAgent.label}**? This action is permanent. The agent will
              lose all access immediately.
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
              {revoking ? <CircularProgress size={16} /> : "Revoke Access"}
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
      .GET("/v1/agents/skill.json")
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
