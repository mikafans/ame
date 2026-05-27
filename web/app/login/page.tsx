"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { setAuthToken } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";
import Divider from "@mui/material/Divider";
import Grid from "@mui/material/Grid";
import Paper from "@mui/material/Paper";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import Alert from "@mui/material/Alert";
import AutoFixHighOutlinedIcon from "@mui/icons-material/AutoFixHighOutlined";
import { Logo } from "@/components/Logo";

type TabId = "signup" | "login";
type Role = "learner" | "instructor" | "agent";

export default function LoginPage() {
  const router = useRouter();
  const [tab, setTab] = useState<TabId>("signup");
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
      const data = await response.json();
      setAuthToken(data.token);
      router.push("/library");
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Could not reach the server",
      );
    } finally {
      setLoading(false);
    }
  }

  const roleDescriptions: Record<Role, string> = {
    learner: "Take assigned quizzes and track your progress.",
    instructor: "Author quizzes, manage cohorts, and review attempts.",
    agent: "Get an API key, an OpenAPI schema, and MCP tool descriptors.",
  };

  return (
    <Grid container sx={{ minHeight: "100vh" }}>
      {/* Left panel */}
      <Grid
        size={6}
        sx={{
          p: "56px 64px",
          borderRight: 1,
          borderColor: "divider",
          background: "linear-gradient(180deg, #f5f5f5 0%, #ffffff 70%)",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
        }}
      >
        <Logo size={28} />

        <Box sx={{ maxWidth: 520 }}>
          <Typography
            variant="caption"
            sx={{
              letterSpacing: 1.6,
              textTransform: "uppercase",
              color: "primary.main",
              mb: 2.25,
              display: "block",
            }}
          >
            Assessment platform · est. 2025
          </Typography>
          <Typography
            variant="h3"
            sx={{
              fontWeight: 500,
              letterSpacing: -1.2,
              lineHeight: 1.04,
              mb: 2.75,
            }}
          >
            Quizzes that learners and agents can both read.
          </Typography>
          <Typography
            variant="body1"
            color="text.secondary"
            sx={{ lineHeight: 1.55, maxWidth: 460 }}
          >
            Harus is an assessment platform built for two audiences at once.
            Students get a focused test-taking experience and a real progress
            dashboard. Authors and AI agents share the same structured surface —
            every quiz, attempt, and rubric is addressable, importable, and
            queryable through a single API.
          </Typography>

          <Grid container spacing={1.5} sx={{ mt: 4.5, maxWidth: 460 }}>
            {[
              ["18,402", "active learners"],
              ["1,243", "instructors"],
              ["94", "institutions"],
              ["6.1M", "graded attempts"],
            ].map(([value, label]) => (
              <Grid size={6} key={label}>
                <Paper variant="outlined" sx={{ p: "12px 14px" }}>
                  <Typography variant="h6" sx={{ fontWeight: 500 }}>
                    {value}
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ letterSpacing: 1, textTransform: "uppercase" }}
                  >
                    {label}
                  </Typography>
                </Paper>
              </Grid>
            ))}
          </Grid>
        </Box>

        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ letterSpacing: 0.6 }}
        >
          SSO · SAML &nbsp;·&nbsp; FERPA · GDPR &nbsp;·&nbsp; OpenAPI 3.1 · MCP
        </Typography>
      </Grid>

      {/* Right panel */}
      <Grid
        size={6}
        sx={{ p: "56px 64px", display: "flex", alignItems: "center" }}
      >
        <Box sx={{ width: "100%", maxWidth: 420 }}>
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
              />
            )}
            <TextField
              label="Institutional email"
              type="email"
              id="email"
              name="email"
              autoComplete="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              fullWidth
              size="small"
              helperText="Recognized: stanford.edu · SSO available"
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
                <Grid container spacing={1} sx={{ mb: 1 }}>
                  {(["learner", "instructor", "agent"] as const).map((r) => (
                    <Grid size={4} key={r}>
                      <Button
                        fullWidth
                        variant={role === r ? "contained" : "outlined"}
                        size="small"
                        onClick={() => setRole(r)}
                        sx={{ textTransform: "capitalize" }}
                      >
                        {r}
                      </Button>
                    </Grid>
                  ))}
                </Grid>
                <Typography variant="caption" color="text.secondary">
                  {roleDescriptions[role]}
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
              sx={{ mt: 1.5 }}
            >
              {loading
                ? "Loading…"
                : tab === "signup"
                  ? "Create account"
                  : "Sign in"}
            </Button>

            <Divider sx={{ my: 1 }}>
              <Typography variant="caption" color="text.secondary">
                OR
              </Typography>
            </Divider>

            <Grid container spacing={1}>
              <Grid size={6}>
                <Button variant="text" fullWidth>
                  Continue with SSO
                </Button>
              </Grid>
              <Grid size={6}>
                <Button variant="text" fullWidth>
                  Use access code
                </Button>
              </Grid>
            </Grid>
          </Box>

          <Paper
            variant="outlined"
            sx={{ mt: 4.5, p: 1.75, borderStyle: "dashed" }}
          >
            <Box
              sx={{
                display: "flex",
                alignItems: "center",
                gap: 1,
                color: "primary.main",
                mb: 0.5,
              }}
            >
              <AutoFixHighOutlinedIcon sx={{ fontSize: 14 }} />
              <Typography variant="caption" sx={{ fontWeight: 600 }}>
                Agent shortcut
              </Typography>
            </Box>
            <Typography variant="caption" color="text.secondary">
              Programmatic access?{" "}
              <Box
                component="span"
                sx={{ fontFamily: "monospace", color: "text.primary" }}
              >
                POST /v1/agents/register
              </Box>{" "}
              returns a key, an OpenAPI schema, and an MCP manifest in one call.
            </Typography>
          </Paper>
        </Box>
      </Grid>
    </Grid>
  );
}
