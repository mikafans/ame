"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useColorMode } from "@/components/ThemeRegistry";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";
import Grid from "@mui/material/Grid";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import Alert from "@mui/material/Alert";
import Chip from "@mui/material/Chip";
import IconButton from "@mui/material/IconButton";
import Tooltip from "@mui/material/Tooltip";
import DarkModeOutlinedIcon from "@mui/icons-material/DarkModeOutlined";
import LightModeOutlinedIcon from "@mui/icons-material/LightModeOutlined";
import { useTheme } from "@mui/material/styles";
import { useAuth } from "@/hooks/useAuth";

type TabId = "signup" | "login";
type Role = "learner" | "instructor";

const ROLES: { id: Role; label: string; color: string; desc: string }[] = [
  {
    id: "learner",
    label: "Learner",
    color: "#22c55e",
    desc: "Take quizzes, track scores, build a progress history. Adaptive difficulty adjusts to your level over time.",
  },
  {
    id: "instructor",
    label: "Instructor",
    color: "#f59e0b",
    desc: "Author quizzes with a point-per-question rubric, manage cohorts, and review graded attempts.",
  },
];

export default function LoginPage() {
  const router = useRouter();
  const { refresh } = useAuth();
  const { mode, toggle } = useColorMode();
  const theme = useTheme();
  const isDark = mode === "dark";

  const [tab, setTab] = useState<TabId>("login");
  const [fullName, setFullName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("learner");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const apiUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const endpoint =
        tab === "signup"
          ? `${apiUrl}/v1/auth/register`
          : `${apiUrl}/v1/auth/login`;
      const body =
        tab === "signup"
          ? { email, name: fullName, password, role }
          : { email, password };
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        let message = "Authentication failed";
        try {
          const data = await response.json();
          message =
            data?.error?.message ||
            data?.message ||
            (typeof data?.error === "string" ? data.error : null) ||
            `Error: ${response.statusText}`;
        } catch {
          message = `Authentication failed: ${response.status} ${response.statusText}`;
        }
        setError(message);
        return;
      }
      await refresh();
      router.push("/library");
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Could not reach the server",
      );
    } finally {
      setLoading(false);
    }
  }

  const leftBg = isDark ? "#0d1117" : "#f5f7ff";
  const leftBorder = isDark ? "none" : `1px solid ${theme.palette.divider}`;
  const cardBg = isDark ? "rgba(255,255,255,0.04)" : "#ffffff";
  const cardBorder = isDark
    ? "1px solid rgba(255,255,255,0.1)"
    : `1px solid ${theme.palette.divider}`;
  const subColor = isDark
    ? "rgba(255,255,255,0.45)"
    : theme.palette.text.secondary;
  const agentBg = isDark ? "rgba(37,99,235,0.1)" : "#eff6ff";
  const agentBorder = isDark
    ? "1px solid rgba(96,165,250,0.25)"
    : "1px dashed #93c5fd";
  const agentCodeColor = isDark ? "#93c5fd" : "#1d4ed8";
  const agentTextColor = isDark ? "rgba(255,255,255,0.4)" : "#64748b";

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      {/* Left panel */}
      <Box
        sx={{
          width: { xs: "100%", md: "50%" },
          display: { xs: "none", md: "flex" },
          flexDirection: "column",
          p: "44px 48px",
          background: leftBg,
          borderRight: leftBorder,
        }}
      >
        {/* Logo + toggle */}
        <Box
          sx={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            mb: 5,
          }}
        >
          <Box sx={{ display: "flex", alignItems: "center", gap: 1.25 }}>
            <Box
              sx={{
                width: 30,
                height: 30,
                background: "#1976d2",
                borderRadius: "7px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 11,
                fontWeight: 700,
                color: "white",
                fontFamily: "monospace",
                letterSpacing: -0.5,
              }}
            >
              ame
            </Box>
            <Typography
              sx={{ fontSize: 16, fontWeight: 600, letterSpacing: -0.2 }}
            >
              ame-platform
            </Typography>
          </Box>
          <Tooltip title={isDark ? "Light mode" : "Dark mode"}>
            <IconButton size="small" onClick={toggle}>
              {isDark ? (
                <LightModeOutlinedIcon fontSize="small" />
              ) : (
                <DarkModeOutlinedIcon fontSize="small" />
              )}
            </IconButton>
          </Tooltip>
        </Box>

        {/* Tagline */}
        <Typography
          variant="h5"
          sx={{ fontWeight: 600, lineHeight: 1.25, letterSpacing: -0.4, mb: 1 }}
        >
          Assessment infrastructure
          <br />
          for learners and agents.
        </Typography>
        <Typography
          sx={{ fontSize: 13, color: subColor, lineHeight: 1.65, mb: 4 }}
        >
          A structured quiz engine with a real API. Every quiz, attempt, and
          rubric is typed, documented, and queryable.
        </Typography>

        {/* Role cards */}
        <Box
          sx={{ display: "flex", flexDirection: "column", gap: 1.25, mb: 3 }}
        >
          {ROLES.map((r) => (
            <Box
              key={r.id}
              sx={{
                border: cardBorder,
                borderRadius: 2,
                p: "12px 16px",
                background: cardBg,
                display: "flex",
                gap: 1.5,
                alignItems: "flex-start",
              }}
            >
              <Box
                sx={{
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: r.color,
                  flexShrink: 0,
                  mt: "5px",
                }}
              />
              <Box>
                <Typography sx={{ fontSize: 12, fontWeight: 600, mb: 0.25 }}>
                  {r.label}
                </Typography>
                <Typography
                  sx={{ fontSize: 12, color: subColor, lineHeight: 1.5 }}
                >
                  {r.desc}
                </Typography>
              </Box>
            </Box>
          ))}
        </Box>

        {/* Agent block */}
        <Box
          sx={{
            border: agentBorder,
            borderRadius: 2,
            p: "12px 16px",
            background: agentBg,
            mt: "auto",
          }}
        >
          <Typography
            sx={{
              fontSize: 10,
              fontWeight: 700,
              letterSpacing: 1.5,
              textTransform: "uppercase",
              color: "#1976d2",
              mb: 0.75,
            }}
          >
            For agents &amp; integrations
          </Typography>
          <Box
            component="pre"
            sx={{
              m: 0,
              fontFamily: "monospace",
              fontSize: 11.5,
              color: agentCodeColor,
              lineHeight: 1.6,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {`curl -X POST $API/v1/agents/register \\
  -d '{"access_code":"…","scopes":["quiz.read"]}'`}
          </Box>
          <Typography
            sx={{
              fontSize: 11,
              color: agentTextColor,
              mt: 0.75,
              lineHeight: 1.5,
            }}
          >
            Returns an API key plus the OpenAPI 3.1 schema and MCP manifest
            URLs. Requires an access code from the operator and at least one
            scope.
          </Typography>
        </Box>

        <Typography
          sx={{
            fontSize: 10,
            color: isDark ? "rgba(255,255,255,0.2)" : "text.disabled",
            letterSpacing: 0.5,
            mt: 2.5,
          }}
        >
          OpenAPI 3.1 · MCP · FERPA · GDPR
        </Typography>
      </Box>

      {/* Right panel — form */}
      <Box
        sx={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          p: "52px 48px",
        }}
      >
        <Box sx={{ width: "100%", maxWidth: 380 }}>
          <Tabs
            value={tab}
            onChange={(_, v) => setTab(v)}
            sx={{ mb: 3.5, borderBottom: 1, borderColor: "divider" }}
          >
            <Tab
              value="signup"
              label="Create account"
              sx={{ textTransform: "none", fontWeight: 500 }}
            />
            <Tab
              value="login"
              label="Sign in"
              sx={{ textTransform: "none", fontWeight: 500 }}
            />
          </Tabs>

          <Box
            component="form"
            onSubmit={handleSubmit}
            sx={{ display: "flex", flexDirection: "column", gap: 2 }}
          >
            {tab === "signup" && (
              <TextField
                label="Full name"
                value={fullName}
                onChange={(e) => setFullName(e.target.value)}
                fullWidth
                size="small"
                autoComplete="name"
              />
            )}
            <TextField
              label="Email"
              type="email"
              id="email"
              name="email"
              autoComplete="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              fullWidth
              size="small"
            />
            <TextField
              label="Password"
              type="password"
              id="password"
              name="password"
              autoComplete={
                tab === "signup" ? "new-password" : "current-password"
              }
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              fullWidth
              size="small"
            />

            {tab === "signup" && (
              <Box>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ display: "block", mb: 1 }}
                >
                  Role
                </Typography>
                <Grid container spacing={1} sx={{ mb: 0.5 }}>
                  {ROLES.map((r) => (
                    <Grid size={6} key={r.id}>
                      <Button
                        fullWidth
                        variant={role === r.id ? "contained" : "outlined"}
                        size="small"
                        onClick={() => setRole(r.id)}
                        sx={{ textTransform: "capitalize" }}
                      >
                        {r.label}
                      </Button>
                    </Grid>
                  ))}
                </Grid>
                <Typography variant="caption" color="text.secondary">
                  {ROLES.find((r) => r.id === role)?.desc}
                </Typography>
              </Box>
            )}

            {error && <Alert severity="error">{error}</Alert>}

            <Button
              type="submit"
              variant="contained"
              size="large"
              disabled={loading}
              fullWidth
              sx={{ mt: 0.5 }}
            >
              {loading
                ? "Loading…"
                : tab === "signup"
                  ? "Create account"
                  : "Sign in"}
            </Button>
          </Box>

          <Box
            sx={{
              mt: 4,
              p: 1.75,
              border: "1px dashed",
              borderColor: "divider",
              borderRadius: 2,
            }}
          >
            <Typography
              variant="caption"
              sx={{ fontWeight: 600, display: "block", mb: 0.5 }}
            >
              Programmatic access?
            </Typography>
            <Typography variant="caption" color="text.secondary">
              <Chip
                label="POST /v1/agents/register"
                size="small"
                variant="outlined"
                sx={{ fontFamily: "monospace", fontSize: 11, mr: 0.5 }}
              />
              returns a key, schema &amp; MCP manifest. Needs an operator access
              code and a scope list.
            </Typography>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
