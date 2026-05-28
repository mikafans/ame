# MUI Full-Adoption Migration + Makefile Cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all custom inline-style UI components and page layouts with MUI v6 primitives, remove Tailwind, and clean up Makefile targets.

**Architecture:** Big-bang replacement — delete `web/src/components/ui/` entirely, move `Logo` to `web/src/components/Logo.tsx`, rewrite `Sidebar.tsx` and all 15 page/layout files to use MUI primitives. ThemeRegistry expands to a full MUI theme. Tailwind packages are removed. Makefile gets cleaner target names and a single `ci` gate.

**Tech Stack:** MUI v9 (`@mui/material`, `@mui/icons-material`), Emotion, `@fontsource/roboto`, Next.js 16 App Router, Bun, TypeScript 5.7. Branch: `feat/mui-migration`. Before screenshots at `/tmp/before-*.png`.

**Verify commands:**
```bash
# type-check only (fast)
cd web && mise exec -- bun run type-check

# full pre-commit gate
make check

# visual regression
make uiux   # requires make dev + make db-seed
```

---

## File Map

| Action | Path |
|---|---|
| Modify | `Makefile` |
| Modify | `web/package.json` |
| Modify | `web/src/components/ThemeRegistry.tsx` |
| Modify | `web/app/globals.css` |
| Modify | `web/app/layout.tsx` |
| Delete | `web/tailwind.config.ts` (if exists) |
| Delete | `web/postcss.config.mjs` (if exists) |
| **Move** | `web/src/components/ui/Logo.tsx` → `web/src/components/Logo.tsx` |
| Delete | `web/src/components/ui/` (entire directory) |
| Modify | `web/src/components/Sidebar.tsx` |
| Modify | `web/app/login/page.tsx` |
| Modify | `web/app/(learner)/layout.tsx` |
| Modify | `web/app/(learner)/library/page.tsx` |
| Modify | `web/app/(learner)/exams/page.tsx` |
| Modify | `web/app/(learner)/progress/page.tsx` |
| Modify | `web/app/(learner)/sessions/[id]/page.tsx` |
| Modify | `web/app/(learner)/sessions/[id]/results/page.tsx` |
| Modify | `web/app/(learner)/agent/page.tsx` |
| Modify | `web/app/(learner)/grading/page.tsx` |
| Modify | `web/app/(learner)/plans/[id]/page.tsx` |
| Modify | `web/app/(learner)/quizzes/[id]/preview/page.tsx` |
| Modify | `web/app/(learner)/practice/page.tsx` |
| Modify | `web/app/(learner)/author/[quizId]/page.tsx` |

---

## Task 1: Makefile Cleanup

**Files:**
- Modify: `Makefile`

- [ ] **Step 1: Apply all renames and additions**

Replace the target block. Key changes: rename `dev-env`→`dev`, `dev-stop`→`stop`, drop `validate`+`pre-remote`, add `ci`, add missing `##` doc strings, update `.PHONY`.

In `Makefile`, make these edits:

1. Replace `.PHONY` line:
```makefile
.PHONY: help fmt fmt-check lint test test-engine test-db test-bank test-stats test-assess test-api e2e uiux check ci db-up db-down db-reset db-migrate db-shell db-seed simulate init-env dev stop hooks-install openapi
```

2. Rename `dev-env` target to `dev`:
```makefile
dev: db-up ## Start full stack: migrate, API on :8080, frontend on :3000
```

3. Rename `dev-stop` target to `stop`:
```makefile
stop: ## Stop API, frontend, and Postgres
	@lsof -ti :8080 -ti :3000 | xargs kill -9 2>/dev/null || true
	docker compose -f db/docker-compose.yml down
```

4. Replace `validate` and `pre-remote` targets with `ci`:
```makefile
ci: check test-db e2e ## Full CI gate: fmt + lint + unit + db tests + e2e
```

5. Add `## doc` to any target missing it. Check `simulate`, `init-env`, `openapi`, `hooks-install` all have `##` descriptions — they already do, but verify `db-migrate`, `db-shell`, `db-seed`, `test-bank`, `test-assess`, `test-stats`, `test-api` each have one too.

- [ ] **Step 2: Verify help output**

```bash
make help
```

Expected: all targets listed, `dev` and `stop` visible, `ci` visible, no `validate` or `pre-remote`, no `dev-env` or `dev-stop`.

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore: rename make targets, add ci gate, drop validate/pre-remote"
```

---

## Task 2: Foundation — Deps, Theme, Globals, Layout

**Files:**
- Modify: `web/package.json`
- Modify: `web/src/components/ThemeRegistry.tsx`
- Modify: `web/app/globals.css`
- Modify: `web/app/layout.tsx`
- Delete: `web/tailwind.config.ts`, `web/postcss.config.mjs` (if present)

- [ ] **Step 1: Add `@fontsource/roboto`, remove Tailwind packages**

```bash
cd web && mise exec -- bun add @fontsource/roboto
mise exec -- bun remove tailwindcss postcss autoprefixer
```

Expected: `bun.lock` updated, `package.json` `devDependencies` no longer contains `tailwindcss`, `postcss`, `autoprefixer`.

- [ ] **Step 2: Delete Tailwind config files**

```bash
rm -f web/tailwind.config.ts web/tailwind.config.js web/postcss.config.mjs web/postcss.config.js
```

- [ ] **Step 3: Rewrite `web/src/components/ThemeRegistry.tsx`**

```tsx
"use client";

import "@fontsource/roboto/300.css";
import "@fontsource/roboto/400.css";
import "@fontsource/roboto/500.css";
import "@fontsource/roboto/700.css";
import { createTheme, ThemeProvider } from "@mui/material/styles";
import CssBaseline from "@mui/material/CssBaseline";

const theme = createTheme({
  palette: {
    primary: { main: "#1976d2" },
    background: { default: "#ffffff", paper: "#f5f5f5" },
  },
  typography: {
    fontFamily: "Roboto, sans-serif",
    fontSize: 14,
  },
  shape: { borderRadius: 8 },
});

export default function ThemeRegistry({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
}
```

- [ ] **Step 4: Rewrite `web/app/globals.css`**

```css
/* MUI CssBaseline handles resets — no Tailwind */
```

- [ ] **Step 5: Rewrite `web/app/layout.tsx`**

Remove `next/font` imports and font CSS variables. `ThemeRegistry` is already wired.

```tsx
import "./globals.css";
import type { Metadata } from "next";
import ThemeRegistry from "@/components/ThemeRegistry";

export const metadata: Metadata = {
  title: "ame",
  description: "Question collector + exam platform",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>
        <ThemeRegistry>{children}</ThemeRegistry>
      </body>
    </html>
  );
}
```

- [ ] **Step 6: Type-check**

```bash
cd web && mise exec -- bun run type-check
```

Expected: no errors (Tailwind types are gone, MUI types are already installed).

- [ ] **Step 7: Commit**

```bash
git add web/package.json web/bun.lock web/src/components/ThemeRegistry.tsx web/app/globals.css web/app/layout.tsx
git commit -m "feat: expand MUI theme, add Roboto, strip Tailwind"
```

---

## Task 3: Mid-Checkpoint Screenshot

**Files:** none modified

- [ ] **Step 1: Ensure stack is running**

```bash
make dev
sleep 5
make db-seed
```

- [ ] **Step 2: Capture mid screenshot**

```bash
TOKEN=$(curl -s http://127.0.0.1:8080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"learner@example.com","password":"password123"}' | jq -r .token)

bunx @playwright/cli open http://localhost:3000/login &
sleep 3
bunx @playwright/cli cookie-set ame_token "$TOKEN" --domain=localhost
bunx @playwright/cli screenshot --filename=/tmp/mid-login.png
bunx @playwright/cli goto http://localhost:3000/library
bunx @playwright/cli screenshot --filename=/tmp/mid-library.png
```

Expected: screenshots render — app loads with MUI CssBaseline active (Roboto font, white background). No crash. The layout still looks like the old inline-style pages because pages haven't been migrated yet — that's expected at this checkpoint.

---

## Task 4: Move Logo + Delete `ui/` + Rewrite Sidebar

**Files:**
- Create: `web/src/components/Logo.tsx`
- Delete: `web/src/components/ui/` (entire directory)
- Modify: `web/src/components/Sidebar.tsx`

- [ ] **Step 1: Create `web/src/components/Logo.tsx`**

Updated to remove CSS variable references (`var(--serif)`, `var(--accent)`):

```tsx
"use client";

import React from "react";
import Link from "next/link";
import Typography from "@mui/material/Typography";

interface LogoProps {
  size?: number;
}

export function Logo({ size = 24 }: LogoProps) {
  return (
    <Link
      href="/"
      style={{ display: "inline-flex", alignItems: "center", gap: 9, textDecoration: "none" }}
    >
      <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
        <rect x="2" y="2" width="28" height="28" rx="6" fill="#1976d2" />
        <path
          d="M10 9v14M22 9v14M10 16h12"
          stroke="#ffffff"
          strokeWidth="2.5"
          strokeLinecap="round"
        />
      </svg>
      <Typography
        component="span"
        sx={{ fontSize: 19, fontWeight: 600, letterSpacing: -0.3, color: "text.primary" }}
      >
        Harus
      </Typography>
    </Link>
  );
}
```

- [ ] **Step 2: Delete `web/src/components/ui/`**

```bash
rm -rf web/src/components/ui
```

- [ ] **Step 3: Rewrite `web/src/components/Sidebar.tsx`**

Full replacement — uses MUI Drawer, List, Avatar. Business logic (items array, section grouping, route mapping) unchanged.

```tsx
"use client";

import React from "react";
import Box from "@mui/material/Box";
import Drawer from "@mui/material/Drawer";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import Typography from "@mui/material/Typography";
import Avatar from "@mui/material/Avatar";
import Divider from "@mui/material/Divider";
import LibraryBooksOutlinedIcon from "@mui/icons-material/LibraryBooksOutlined";
import LayersOutlinedIcon from "@mui/icons-material/LayersOutlined";
import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";
import AssessmentOutlinedIcon from "@mui/icons-material/AssessmentOutlined";
import DashboardOutlinedIcon from "@mui/icons-material/DashboardOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import GradingOutlinedIcon from "@mui/icons-material/GradingOutlined";
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";
import SettingsOutlinedIcon from "@mui/icons-material/SettingsOutlined";
import { Logo } from "@/components/Logo";
import { useAuth } from "@/hooks/useAuth";

const DRAWER_WIDTH = 232;

const ICON_MAP: Record<string, React.ReactElement> = {
  library: <LibraryBooksOutlinedIcon fontSize="small" />,
  stack: <LayersOutlinedIcon fontSize="small" />,
  take: <PlayArrowOutlinedIcon fontSize="small" />,
  results: <AssessmentOutlinedIcon fontSize="small" />,
  dashboard: <DashboardOutlinedIcon fontSize="small" />,
  author: <EditOutlinedIcon fontSize="small" />,
  grade: <GradingOutlinedIcon fontSize="small" />,
  agent: <SmartToyOutlinedIcon fontSize="small" />,
};

interface SidebarProps {
  route: string;
  setRoute: (route: string) => void;
  showAgent?: boolean;
}

export function Sidebar({ route, setRoute, showAgent = false }: SidebarProps) {
  const { user } = useAuth();
  const isInstructor = user?.role === "instructor" || user?.role === "admin";

  const items = [
    { id: "library", label: "Library", icon: "library", section: "Learn" },
    { id: "exams", label: "Exams", icon: "stack", section: "Learn" },
    { id: "quiz", label: "Take quiz", icon: "take", section: "Learn" },
    { id: "results", label: "Last results", icon: "results", section: "Learn" },
    { id: "dashboard", label: "Progress", icon: "dashboard", section: "Learn" },
    ...(isInstructor
      ? [
          { id: "author", label: "Author studio", icon: "author", section: "Teach" },
          { id: "grading", label: "Grading", icon: "grade", section: "Teach" },
        ]
      : []),
    ...(showAgent && isInstructor
      ? [{ id: "agent", label: "Agent API", icon: "agent", section: "Integrate" }]
      : []),
  ];

  const sections = ["Learn", "Teach", "Integrate"];
  const initials =
    user?.displayName
      ?.split(" ")
      .map((n) => n[0])
      .join("")
      .toUpperCase()
      .slice(0, 2) || "JT";
  const displayName = user?.displayName || "Jordan Tahir";
  const cohort = "CS '27";

  return (
    <Drawer
      variant="permanent"
      sx={{
        width: DRAWER_WIDTH,
        flexShrink: 0,
        "& .MuiDrawer-paper": {
          width: DRAWER_WIDTH,
          boxSizing: "border-box",
          display: "flex",
          flexDirection: "column",
        },
      }}
    >
      <Box sx={{ p: 2.5, pb: 2, borderBottom: 1, borderColor: "divider" }}>
        <Logo />
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ letterSpacing: 1.2, textTransform: "uppercase", display: "block", mt: 0.5 }}
        >
          Assessment Platform · v2.4
        </Typography>
      </Box>

      <Box sx={{ flex: 1, overflowY: "auto", py: 1 }}>
        {sections.map((sec) => {
          const inSec = items.filter((i) => i.section === sec);
          if (!inSec.length) return null;
          return (
            <Box key={sec} sx={{ mb: 2 }}>
              <Typography
                variant="caption"
                sx={{
                  px: 1.5,
                  py: 0.75,
                  display: "block",
                  letterSpacing: 1.4,
                  textTransform: "uppercase",
                  color: "text.secondary",
                }}
              >
                {sec}
              </Typography>
              <List dense disablePadding>
                {inSec.map((it) => (
                  <ListItem key={it.id} disablePadding>
                    <ListItemButton
                      selected={route === it.id}
                      onClick={() => setRoute(it.id)}
                      sx={{ borderRadius: 1, mx: 0.5 }}
                    >
                      <ListItemIcon sx={{ minWidth: 32 }}>
                        {ICON_MAP[it.icon]}
                      </ListItemIcon>
                      <ListItemText
                        primary={it.label}
                        primaryTypographyProps={{ fontSize: 13 }}
                      />
                    </ListItemButton>
                  </ListItem>
                ))}
              </List>
            </Box>
          );
        })}
      </Box>

      <Divider />
      <Box sx={{ p: 1.75, display: "flex", alignItems: "center", gap: 1.25 }}>
        <Avatar sx={{ width: 32, height: 32, fontSize: 14 }}>{initials}</Avatar>
        <Box sx={{ flex: 1, minWidth: 0 }}>
          <Typography variant="body2" fontWeight={500} noWrap>
            {displayName}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            {user?.role || "Student"} · {cohort}
          </Typography>
        </Box>
        <SettingsOutlinedIcon sx={{ fontSize: 16, color: "text.secondary" }} />
      </Box>
    </Drawer>
  );
}
```

- [ ] **Step 4: Type-check**

```bash
cd web && mise exec -- bun run type-check
```

Expected: no errors referencing `@/components/ui`.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/Logo.tsx web/src/components/Sidebar.tsx
git rm -r web/src/components/ui
git commit -m "feat: move Logo, delete custom ui lib, rewrite Sidebar with MUI Drawer"
```

---

## Task 5: Rewrite Login Page

**Files:**
- Modify: `web/app/login/page.tsx`

- [ ] **Step 1: Rewrite `web/app/login/page.tsx`**

Keep all state, handlers, and business logic unchanged. Replace only the JSX return and imports.

```tsx
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

type Tab = "signup" | "login";
type Role = "learner" | "instructor" | "agent";

export default function LoginPage() {
  const router = useRouter();
  const [tab, setTab] = useState<Tab>("signup");
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
      setError(err instanceof Error ? err.message : "Could not reach the server");
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
            sx={{ letterSpacing: 1.6, textTransform: "uppercase", color: "primary.main", mb: 2.25, display: "block" }}
          >
            Assessment platform · est. 2025
          </Typography>
          <Typography variant="h3" sx={{ fontWeight: 500, letterSpacing: -1.2, lineHeight: 1.04, mb: 2.75 }}>
            Quizzes that learners and agents can both read.
          </Typography>
          <Typography variant="body1" color="text.secondary" sx={{ lineHeight: 1.55, maxWidth: 460 }}>
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
                  <Typography variant="h6" sx={{ fontWeight: 500 }}>{value}</Typography>
                  <Typography variant="caption" color="text.secondary" sx={{ letterSpacing: 1, textTransform: "uppercase" }}>
                    {label}
                  </Typography>
                </Paper>
              </Grid>
            ))}
          </Grid>
        </Box>

        <Typography variant="caption" color="text.secondary" sx={{ letterSpacing: 0.6 }}>
          SSO · SAML &nbsp;·&nbsp; FERPA · GDPR &nbsp;·&nbsp; OpenAPI 3.1 · MCP
        </Typography>
      </Grid>

      {/* Right panel */}
      <Grid size={6} sx={{ p: "56px 64px", display: "flex", alignItems: "center" }}>
        <Box sx={{ width: "100%", maxWidth: 420 }}>
          <Tabs
            value={tab}
            onChange={(_, v) => setTab(v)}
            sx={{ mb: 3.5, borderBottom: 1, borderColor: "divider" }}
          >
            <Tab value="signup" label="Create account" sx={{ textTransform: "none", fontWeight: 500 }} />
            <Tab value="login" label="Sign in" sx={{ textTransform: "none", fontWeight: 500 }} />
          </Tabs>

          <Box component="form" onSubmit={handleSubmit} sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
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
              autoComplete={tab === "signup" ? "new-password" : "current-password"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              fullWidth
              size="small"
            />

            {tab === "signup" && (
              <Box>
                <Typography variant="caption" color="text.secondary" sx={{ display: "block", mb: 1 }}>
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
              {loading ? "Loading…" : tab === "signup" ? "Create account" : "Sign in"}
            </Button>

            <Divider sx={{ my: 1 }}>
              <Typography variant="caption" color="text.secondary">OR</Typography>
            </Divider>

            <Grid container spacing={1}>
              <Grid size={6}>
                <Button variant="text" fullWidth>Continue with SSO</Button>
              </Grid>
              <Grid size={6}>
                <Button variant="text" fullWidth>Use access code</Button>
              </Grid>
            </Grid>
          </Box>

          <Paper
            variant="outlined"
            sx={{ mt: 4.5, p: 1.75, borderStyle: "dashed" }}
          >
            <Box sx={{ display: "flex", alignItems: "center", gap: 1, color: "primary.main", mb: 0.5 }}>
              <AutoFixHighOutlinedIcon sx={{ fontSize: 14 }} />
              <Typography variant="caption" fontWeight={600}>Agent shortcut</Typography>
            </Box>
            <Typography variant="caption" color="text.secondary">
              Programmatic access?{" "}
              <Box component="span" sx={{ fontFamily: "monospace", color: "text.primary" }}>
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
```

- [ ] **Step 2: Type-check**

```bash
cd web && mise exec -- bun run type-check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/app/login/page.tsx
git commit -m "feat: rewrite login page with MUI Grid, TextField, Tabs"
```

---

## Task 6: Rewrite Learner Layout

**Files:**
- Modify: `web/app/(learner)/layout.tsx`

- [ ] **Step 1: Rewrite `web/app/(learner)/layout.tsx`**

Keep all auth redirect logic, theme/statsDepth state, and `TweaksPanel` component. Only replace layout primitives and remove CSS variable references in `TweaksPanel`.

```tsx
"use client";

import { useEffect, useState } from "react";
import { useRouter, usePathname } from "next/navigation";
import Box from "@mui/material/Box";
import Paper from "@mui/material/Paper";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import IconButton from "@mui/material/IconButton";
import CloseIcon from "@mui/icons-material/Close";
import { useAuth } from "@/hooks/useAuth";
import { Sidebar } from "@/components/Sidebar";

const DEMO_MODE = process.env.NEXT_PUBLIC_DEMO_MODE === "1";

export default function LearnerLayout({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    if (!loading && !user) router.push("/login");
  }, [loading, user, router]);

  const [theme, setTheme] = useState("slate");
  const [statsDepth, setStatsDepth] = useState<"minimal" | "standard" | "full">("standard");
  const [showTweaks, setShowTweaks] = useState(false);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    try { localStorage.setItem("harus.theme", theme); } catch { /* ignore */ }
  }, [theme]);

  useEffect(() => {
    try {
      const saved = localStorage.getItem("harus.theme");
      if (saved) setTheme(saved);
    } catch { /* ignore */ }
  }, []);

  const role = user?.role ?? "learner";

  const getRouteId = () => {
    if (pathname.startsWith("/library")) return "library";
    if (pathname.startsWith("/exams")) return "exams";
    if (pathname.startsWith("/practice")) return "quiz";
    if (pathname.startsWith("/results")) return "results";
    if (pathname.startsWith("/progress")) return "dashboard";
    if (pathname.startsWith("/author")) return "author";
    if (pathname.startsWith("/grading")) return "grading";
    if (pathname.startsWith("/agent")) return "agent";
    return "library";
  };

  const currentRoute = getRouteId();

  const handleRouteChange = (route: string) => {
    const routeMap: Record<string, string> = {
      library: "/library",
      exams: "/exams",
      quiz: "/practice",
      results: "/results",
      dashboard: "/progress",
      author: "/author",
      grading: "/grading",
      agent: "/agent",
    };
    router.push(routeMap[route] || "/library");
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar route={currentRoute} setRoute={handleRouteChange} showAgent={role !== "learner"} />
      <Box component="main" sx={{ flex: 1, overflowY: "auto" }}>
        {children}
      </Box>
      {DEMO_MODE && showTweaks && (
        <TweaksPanel
          theme={theme}
          setTheme={setTheme}
          statsDepth={statsDepth}
          setStatsDepth={setStatsDepth}
          onClose={() => setShowTweaks(false)}
        />
      )}
    </Box>
  );
}

function TweaksPanel({
  theme,
  setTheme,
  statsDepth,
  setStatsDepth,
  onClose,
}: {
  theme: string;
  setTheme: (t: string) => void;
  statsDepth: "minimal" | "standard" | "full";
  setStatsDepth: (d: "minimal" | "standard" | "full") => void;
  onClose: () => void;
}) {
  if (process.env.NODE_ENV === "production") {
    throw new Error("TweaksPanel must not be rendered in production.");
  }

  return (
    <Paper
      elevation={8}
      sx={{ position: "fixed", bottom: 24, right: 24, p: 2.5, width: 260, zIndex: 200 }}
    >
      <Box sx={{ display: "flex", justifyContent: "space-between", alignItems: "center", mb: 2 }}>
        <Box>
          <Typography variant="caption" color="primary" sx={{ letterSpacing: 1.5, textTransform: "uppercase", display: "block" }}>
            Demo only
          </Typography>
          <Typography variant="body2" fontWeight={600}>Tweaks</Typography>
        </Box>
        <IconButton size="small" onClick={onClose}><CloseIcon fontSize="small" /></IconButton>
      </Box>

      <Box sx={{ mb: 2 }}>
        <Typography variant="caption" color="text.secondary" sx={{ display: "block", mb: 1, letterSpacing: 1.2, textTransform: "uppercase" }}>
          Theme
        </Typography>
        <Box sx={{ display: "flex", gap: 0.75 }}>
          {["slate", "paper", "cobalt"].map((t) => (
            <Button
              key={t}
              size="small"
              variant={theme === t ? "contained" : "outlined"}
              onClick={() => setTheme(t)}
              sx={{ flex: 1, fontSize: 11 }}
            >
              {t}
            </Button>
          ))}
        </Box>
      </Box>

      <Box>
        <Typography variant="caption" color="text.secondary" sx={{ display: "block", mb: 1, letterSpacing: 1.2, textTransform: "uppercase" }}>
          Stats depth
        </Typography>
        <Box sx={{ display: "flex", gap: 0.75 }}>
          {(["minimal", "standard", "full"] as const).map((d) => (
            <Button
              key={d}
              size="small"
              variant={statsDepth === d ? "contained" : "outlined"}
              onClick={() => setStatsDepth(d)}
              sx={{ flex: 1, fontSize: 10 }}
            >
              {d}
            </Button>
          ))}
        </Box>
      </Box>
    </Paper>
  );
}
```

- [ ] **Step 2: Type-check**

```bash
cd web && mise exec -- bun run type-check
```

- [ ] **Step 3: Commit**

```bash
git add "web/app/(learner)/layout.tsx"
git commit -m "feat: rewrite learner layout with MUI Box + Drawer"
```

---

## Task 7: Rewrite Library Page

**Files:**
- Modify: `web/app/(learner)/library/page.tsx`

- [ ] **Step 1: Update imports in `web/app/(learner)/library/page.tsx`**

Replace all `@/components/ui` imports with MUI:

```tsx
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Divider from "@mui/material/Divider";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import ShareOutlinedIcon from "@mui/icons-material/ShareOutlined";
import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";
// Keep existing imports: useState, useEffect, useRouter, makeClient, useAuth, LearningObjectives, ShareModal
```

- [ ] **Step 2: Replace JSX return in `web/app/(learner)/library/page.tsx`**

Keep all state, `useEffect`, `fetchStatus`, `handleStart`, and `load` logic unchanged. Replace the JSX return statement (everything after the handlers):

```tsx
  if (loading) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", height: "60vh" }}>
        <CircularProgress />
      </Box>
    );
  }

  const { quizzes, total } = allQuizzes[tab];
  const featured = quizzes[0] ?? null;

  return (
    <Box sx={{ p: 4 }}>
      {/* Header */}
      <Typography variant="caption" color="text.secondary" sx={{ letterSpacing: 1.4, textTransform: "uppercase", display: "block" }}>
        Spring 2026 · Active Term
      </Typography>
      <Typography variant="h4" fontWeight={500} sx={{ mb: 3 }}>
        Library
      </Typography>

      {/* Tabs */}
      <Tabs value={tab} onChange={(_, v) => setTab(v)} sx={{ mb: 3, borderBottom: 1, borderColor: "divider" }}>
        <Tab value="all" label={`All quizzes (${allQuizzes.all.total})`} sx={{ textTransform: "none" }} />
        <Tab value="assigned" label={`Assigned to me (${allQuizzes.assigned.total})`} sx={{ textTransform: "none" }} />
        <Tab value="completed" label={`Completed (${allQuizzes.completed.total})`} sx={{ textTransform: "none" }} />
        {allQuizzes.drafts.total > 0 && (
          <Tab value="drafts" label={`Drafts (${allQuizzes.drafts.total})`} sx={{ textTransform: "none" }} />
        )}
      </Tabs>

      {startError && <Alert severity="error" sx={{ mb: 2 }}>{startError}</Alert>}

      {quizzes.length === 0 ? (
        <Typography color="text.secondary">No quizzes in this tab.</Typography>
      ) : (
        <Stack spacing={2}>
          {quizzes.map((quiz) => (
            <Card key={quiz.id} variant="outlined">
              <CardContent>
                <Box sx={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                  <Box sx={{ flex: 1 }}>
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 0.5 }}>
                      <Typography variant="subtitle1" fontWeight={500}>{quiz.title}</Typography>
                      {quiz.course && (
                        <Chip label={quiz.course} size="small" variant="outlined" />
                      )}
                      {quiz.difficulty && (
                        <Chip label={quiz.difficulty} size="small" color="primary" variant="outlined" />
                      )}
                    </Box>
                    {quiz.description && (
                      <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                        {quiz.description}
                      </Typography>
                    )}
                    <Stack direction="row" spacing={2}>
                      {quiz.questionCount != null && (
                        <Typography variant="caption" color="text.secondary">{quiz.questionCount} questions</Typography>
                      )}
                      {quiz.durationMin != null && (
                        <Typography variant="caption" color="text.secondary">{quiz.durationMin} min</Typography>
                      )}
                    </Stack>
                  </Box>
                  <Stack direction="row" spacing={1}>
                    <Button
                      size="small"
                      variant="outlined"
                      startIcon={<ShareOutlinedIcon />}
                      onClick={() => setShareOpen(quiz.id)}
                    >
                      Share
                    </Button>
                    <Button
                      size="small"
                      variant="contained"
                      startIcon={<PlayArrowOutlinedIcon />}
                      disabled={starting === quiz.id}
                      onClick={() => handleStart(quiz.id)}
                    >
                      {starting === quiz.id ? "Starting…" : "Start"}
                    </Button>
                  </Stack>
                </Box>
              </CardContent>
            </Card>
          ))}
        </Stack>
      )}

      {shareOpen && (
        <ShareModal quizId={shareOpen} onClose={() => setShareOpen(null)} />
      )}
    </Box>
  );
```

- [ ] **Step 3: Type-check**

```bash
cd web && mise exec -- bun run type-check
```

- [ ] **Step 4: Commit**

```bash
git add "web/app/(learner)/library/page.tsx"
git commit -m "feat: rewrite library page with MUI Card, Tabs, Chip"
```

---

## Task 8: Rewrite Exams Page

**Files:**
- Modify: `web/app/(learner)/exams/page.tsx`

- [ ] **Step 1: Update imports**

```tsx
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import TextField from "@mui/material/TextField";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import Divider from "@mui/material/Divider";
// Keep: useState, useEffect, useRouter, makeClient, useAuth, LearningObjectives, ShareModal
```

- [ ] **Step 2: Replace JSX return**

Keep all state, handlers (`load`, `startExam`, `handleCompose`, `handlePublish`), and interface definitions. Replace only the return block:

```tsx
  return (
    <Box sx={{ display: "flex", height: "100vh", overflow: "hidden" }}>
      {/* Left: exam list */}
      <Box sx={{ width: 320, borderRight: 1, borderColor: "divider", display: "flex", flexDirection: "column" }}>
        <Box sx={{ p: 2.5, borderBottom: 1, borderColor: "divider" }}>
          <Typography variant="h6" fontWeight={500}>Exams</Typography>
          <Tabs value={tab} onChange={(_, v) => setTab(v)} sx={{ mt: 1 }}>
            {tabs.map((t) => (
              <Tab key={t.id} value={t.id} label={t.label} sx={{ textTransform: "none", minWidth: 0, fontSize: 12 }} />
            ))}
          </Tabs>
        </Box>

        <Box sx={{ flex: 1, overflowY: "auto", p: 1 }}>
          {loading ? (
            <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
              <CircularProgress size={24} />
            </Box>
          ) : filtered.length === 0 ? (
            <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>No exams.</Typography>
          ) : (
            filtered.map((e) => (
              <Card
                key={e.id}
                variant="outlined"
                onClick={() => setSelected(e.id)}
                sx={{ mb: 1, cursor: "pointer", ...(selected === e.id && { borderColor: "primary.main" }) }}
              >
                <CardContent sx={{ pb: "12px !important" }}>
                  <Typography variant="subtitle2" fontWeight={500}>{e.name}</Typography>
                  <Stack direction="row" spacing={0.75} sx={{ mt: 0.75 }}>
                    <Chip label={e.status} size="small" variant="outlined" />
                    {e.durationMin && <Chip label={`${e.durationMin}m`} size="small" variant="outlined" />}
                  </Stack>
                </CardContent>
              </Card>
            ))
          )}
        </Box>

        {isInstructor && (
          <Box sx={{ p: 1.5, borderTop: 1, borderColor: "divider" }}>
            <Button fullWidth variant="outlined" onClick={() => setShowCompose(true)}>
              Compose exam
            </Button>
          </Box>
        )}
      </Box>

      {/* Right: exam detail */}
      <Box sx={{ flex: 1, overflowY: "auto", p: 3 }}>
        {!exam ? (
          <Typography color="text.secondary">Select an exam.</Typography>
        ) : (
          <Stack spacing={2}>
            <Box sx={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
              <Box>
                <Typography variant="h5" fontWeight={500}>{exam.name}</Typography>
                {exam.description && (
                  <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>{exam.description}</Typography>
                )}
              </Box>
              <Stack direction="row" spacing={1}>
                {exam.status === "published" && (
                  <Button variant="contained" disabled={starting} onClick={startExam}>
                    {starting ? "Starting…" : "Start exam"}
                  </Button>
                )}
                {isInstructor && exam.status !== "published" && (
                  <Button variant="outlined" onClick={() => handlePublish(exam.id)}>Publish</Button>
                )}
                <Button variant="outlined" startIcon={<ShareOutlinedIcon />} onClick={() => setShareExam(exam)}>
                  Share
                </Button>
              </Stack>
            </Box>

            <Stack direction="row" spacing={1} flexWrap="wrap">
              <Chip label={exam.status} variant="outlined" />
              {exam.course && <Chip label={exam.course} variant="outlined" />}
              {exam.durationMin && <Chip label={`${exam.durationMin} min`} variant="outlined" />}
            </Stack>

            {exam.objectives.length > 0 && (
              <LearningObjectives objectives={exam.objectives} />
            )}

            {exam.sections && exam.sections.length > 0 && (
              <Box>
                <Typography variant="subtitle2" fontWeight={600} sx={{ mb: 1 }}>Sections</Typography>
                <Stack spacing={1}>
                  {exam.sections.map((s) => (
                    <Card key={s.id} variant="outlined">
                      <CardContent sx={{ pb: "12px !important" }}>
                        <Box sx={{ display: "flex", justifyContent: "space-between" }}>
                          <Typography variant="body2" fontWeight={500}>{s.title}</Typography>
                          <Typography variant="caption" color="text.secondary">{s.items} questions · {s.weight}pts</Typography>
                        </Box>
                      </CardContent>
                    </Card>
                  ))}
                </Stack>
              </Box>
            )}
          </Stack>
        )}
      </Box>

      {/* Compose dialog kept as-is */}
      {showCompose && (
        <Box sx={{ position: "fixed", inset: 0, bgcolor: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1300 }}>
          <Card sx={{ width: 480, maxHeight: "80vh", overflowY: "auto" }}>
            <CardContent>
              <Typography variant="h6" sx={{ mb: 2 }}>Compose exam</Typography>
              <Stack spacing={2}>
                <TextField label="Name" value={composeName} onChange={(e) => setComposeName(e.target.value)} fullWidth size="small" />
                <TextField label="Description" value={composeDesc} onChange={(e) => setComposeDesc(e.target.value)} fullWidth size="small" multiline rows={2} />
                <TextField label="Duration (min)" type="number" value={composeDuration} onChange={(e) => setComposeDuration(Number(e.target.value))} fullWidth size="small" />
                <Divider />
                <Typography variant="subtitle2">Sections</Typography>
                {sections.map((s, i) => (
                  <Stack key={i} spacing={1}>
                    <TextField label="Section title" value={s.title} onChange={(e) => { const ns = [...sections]; ns[i].title = e.target.value; setSections(ns); }} fullWidth size="small" />
                    <TextField label="Question IDs (comma-separated)" value={s.questionIds} onChange={(e) => { const ns = [...sections]; ns[i].questionIds = e.target.value; setSections(ns); }} fullWidth size="small" />
                  </Stack>
                ))}
                <Button onClick={() => setSections([...sections, { title: "", weight: 1, questionIds: "" }])} variant="outlined" size="small">
                  Add section
                </Button>
                {composeError && <Alert severity="error">{composeError}</Alert>}
              </Stack>
              <Stack direction="row" justifyContent="flex-end" spacing={1} sx={{ mt: 2 }}>
                <Button onClick={() => setShowCompose(false)}>Cancel</Button>
                <Button variant="contained" disabled={composing} onClick={handleCompose}>
                  {composing ? "Composing…" : "Compose"}
                </Button>
              </Stack>
            </CardContent>
          </Card>
        </Box>
      )}

      {shareExam && <ShareModal quizId={shareExam.id} onClose={() => setShareExam(null)} />}
    </Box>
  );
```

Note: `ShareOutlinedIcon` import needed — add `import ShareOutlinedIcon from "@mui/icons-material/ShareOutlined";`. `handlePublish` must be defined above the return — check existing code has this handler; if not, add: `async function handlePublish(id: string) { await (makeClient(token!) as any).PATCH(\`/v1/exams/${id}\`, { body: { status: "published" } }); load(); }`.

- [ ] **Step 3: Type-check**

```bash
cd web && mise exec -- bun run type-check
```

- [ ] **Step 4: Commit**

```bash
git add "web/app/(learner)/exams/page.tsx"
git commit -m "feat: rewrite exams page with MUI layout"
```

---

## Task 9: Rewrite Progress Page

**Files:**
- Modify: `web/app/(learner)/progress/page.tsx`

- [ ] **Step 1: Replace imports and rewrite JSX**

Keep all state, `useEffect`, and data-fetching logic. Replace imports and JSX return:

```tsx
// Replace @/components/ui imports with:
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import CircularProgress from "@mui/material/CircularProgress";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import ToggleButton from "@mui/material/ToggleButton";
```

Replace the JSX return:

```tsx
  return (
    <Box sx={{ p: 4 }}>
      <Box sx={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", mb: 3 }}>
        <Box>
          <Typography variant="caption" color="text.secondary" sx={{ letterSpacing: 1.4, textTransform: "uppercase", display: "block" }}>
            All courses
          </Typography>
          <Typography variant="h4" fontWeight={500}>Progress dashboard</Typography>
        </Box>
        <Stack direction="row" spacing={1} alignItems="center">
          <ToggleButtonGroup value={window} exclusive onChange={(_, v) => v && setWindow(v)} size="small">
            <ToggleButton value="4w">4w</ToggleButton>
            <ToggleButton value="12w">12w</ToggleButton>
            <ToggleButton value="all">All</ToggleButton>
          </ToggleButtonGroup>
          <Button variant="outlined" size="small">Export</Button>
        </Stack>
      </Box>

      {statsLoading ? (
        <Typography color="text.secondary" sx={{ mb: 3 }}>Loading…</Typography>
      ) : stats ? (
        <Stack direction="row" spacing={2} sx={{ mb: 3 }}>
          {[
            { label: "Avg score", value: `${Math.round(stats.avg_score ?? 0)}%` },
            { label: "Attempts", value: String(stats.attempts_total ?? 0) },
            { label: "Hours", value: `${stats.hours_spent ?? 0}h` },
            { label: "Streak", value: `${stats.current_streak ?? 0}d` },
          ].map(({ label, value }) => (
            <Card key={label} variant="outlined" sx={{ flex: 1 }}>
              <CardContent>
                <Typography variant="h5" fontWeight={500}>{value}</Typography>
                <Typography variant="caption" color="text.secondary" sx={{ textTransform: "uppercase", letterSpacing: 1 }}>
                  {label}
                </Typography>
              </CardContent>
            </Card>
          ))}
        </Stack>
      ) : null}

      <Stack direction="row" spacing={2}>
        <Card variant="outlined" sx={{ flex: 1 }}>
          <CardContent>
            <Typography variant="subtitle1" fontWeight={500}>Score trend</Typography>
            <Typography variant="body2" color="text.secondary">Weekly rolling average across all subjects</Typography>
            <Box sx={{ height: 200, display: "flex", alignItems: "center", justifyContent: "center" }}>
              {chartLoading ? <CircularProgress size={24} /> : <Typography color="text.secondary">No data</Typography>}
            </Box>
          </CardContent>
        </Card>
        <Card variant="outlined" sx={{ flex: 1 }}>
          <CardContent>
            <Typography variant="subtitle1" fontWeight={500}>By subject</Typography>
            <Typography variant="body2" color="text.secondary">Average score, last N weeks</Typography>
            <Box sx={{ height: 200, display: "flex", alignItems: "center", justifyContent: "center" }}>
              {chartLoading ? <CircularProgress size={24} /> : <Typography color="text.secondary">No data</Typography>}
            </Box>
          </CardContent>
        </Card>
      </Stack>
    </Box>
  );
```

Note: `window` state and `chartLoading`/`statsLoading` vars must be in the existing component. Check that `stats`, `statsLoading`, `chartLoading` are defined in the existing `useEffect` logic — add `const [chartLoading] = useState(false);` if missing.

- [ ] **Step 2: Type-check + commit**

```bash
cd web && mise exec -- bun run type-check
git add "web/app/(learner)/progress/page.tsx"
git commit -m "feat: rewrite progress page with MUI Card, ToggleButtonGroup"
```

---

## Task 10: Rewrite Session Pages

**Files:**
- Modify: `web/app/(learner)/sessions/[id]/page.tsx`
- Modify: `web/app/(learner)/sessions/[id]/results/page.tsx`

### 10a — Active session page

- [ ] **Step 1: Update imports in `web/app/(learner)/sessions/[id]/page.tsx`**

```tsx
// Replace @/components/ui imports with:
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import LinearProgress from "@mui/material/LinearProgress";
import CircularProgress from "@mui/material/CircularProgress";
import Paper from "@mui/material/Paper";
import ArrowBackOutlinedIcon from "@mui/icons-material/ArrowBackOutlined";
import ArrowForwardOutlinedIcon from "@mui/icons-material/ArrowForwardOutlined";
import ExitToAppOutlinedIcon from "@mui/icons-material/ExitToAppOutlined";
// Keep question renderer imports: McqRenderer, ShortRenderer, etc.
```

- [ ] **Step 2: Replace JSX return (session page)**

Keep all state and handlers. Replace return:

```tsx
  if (loading) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", height: "100vh" }}>
        <CircularProgress />
      </Box>
    );
  }

  if (!session || !questions.length) {
    return (
      <Box sx={{ p: 4 }}>
        <Typography color="text.secondary">Session not found.</Typography>
      </Box>
    );
  }

  const current = questions[currentIndex];
  const answered = Object.keys(answers).length;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      {/* Sticky header */}
      <Paper elevation={0} sx={{ borderBottom: 1, borderColor: "divider", px: 3, py: 1.5, display: "flex", alignItems: "center", justifyContent: "space-between", position: "sticky", top: 0, zIndex: 100, bgcolor: "background.default" }}>
        <Typography variant="subtitle2" fontWeight={500}>
          {session.quizTitle || "Quiz"}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {answered}/{questions.length} answered
        </Typography>
        <Button size="small" variant="outlined" startIcon={<ExitToAppOutlinedIcon />} onClick={handleExit}>
          Save &amp; exit
        </Button>
      </Paper>

      {/* Progress bar */}
      <LinearProgress variant="determinate" value={questions.length ? (answered / questions.length) * 100 : 0} />

      {/* Question */}
      <Box sx={{ flex: 1, overflowY: "auto", p: 4, maxWidth: 800, mx: "auto", width: "100%" }}>
        <Typography variant="caption" color="text.secondary" sx={{ display: "block", mb: 1 }}>
          Question {currentIndex + 1} of {questions.length} · {current.points} pt{current.points !== 1 ? "s" : ""}
        </Typography>
        <Typography variant="h6" sx={{ mb: 3 }}>{current.prompt}</Typography>

        {current.kind === "mc" && <McqRenderer question={current} answer={answers[current.questionId]} onAnswer={(a) => handleAnswer(current.questionId, a)} />}
        {current.kind === "tf" && <TfRenderer question={current} answer={answers[current.questionId]} onAnswer={(a) => handleAnswer(current.questionId, a)} />}
        {current.kind === "short" && <ShortRenderer question={current} answer={answers[current.questionId]} onAnswer={(a) => handleAnswer(current.questionId, a)} />}
        {current.kind === "essay" && <EssayRenderer question={current} answer={answers[current.questionId]} onAnswer={(a) => handleAnswer(current.questionId, a)} />}
        {current.kind === "cloze" && <ClozeRenderer question={current} answer={answers[current.questionId]} onAnswer={(a) => handleAnswer(current.questionId, a)} />}
        {current.kind === "code" && <CodeRenderer question={current} answer={answers[current.questionId]} onAnswer={(a) => handleAnswer(current.questionId, a)} />}
      </Box>

      {/* Footer nav */}
      <Paper elevation={0} sx={{ borderTop: 1, borderColor: "divider", px: 3, py: 1.5, display: "flex", justifyContent: "space-between" }}>
        <Button startIcon={<ArrowBackOutlinedIcon />} disabled={currentIndex === 0} onClick={() => setCurrentIndex((i) => i - 1)}>
          Previous
        </Button>
        {currentIndex < questions.length - 1 ? (
          <Button endIcon={<ArrowForwardOutlinedIcon />} variant="contained" onClick={() => setCurrentIndex((i) => i + 1)}>
            Next
          </Button>
        ) : (
          <Button variant="contained" color="success" disabled={submitting} onClick={handleSubmit}>
            {submitting ? "Submitting…" : "Submit"}
          </Button>
        )}
      </Paper>
    </Box>
  );
```

Note: `handleExit`, `handleAnswer`, `handleSubmit`, `submitting`, `answers`, `currentIndex`, `setCurrentIndex` must all be in the existing component — do not remove them.

### 10b — Session results page

- [ ] **Step 3: Update imports in `web/app/(learner)/sessions/[id]/results/page.tsx`**

```tsx
// Replace @/components/ui imports with:
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Divider from "@mui/material/Divider";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline";
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined";
import ShareOutlinedIcon from "@mui/icons-material/ShareOutlined";
// Keep: ShareModal import
```

- [ ] **Step 4: Replace JSX return (results page)**

Keep all state, `useEffect`, `data` fetching, `answers` processing. Replace return:

```tsx
  if (loading) {
    return (
      <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", height: "60vh" }}>
        <CircularProgress />
      </Box>
    );
  }

  if (!data) return <Box sx={{ p: 4 }}><Typography color="text.secondary">Results not found.</Typography></Box>;

  const scorePercent = data.maxPoints > 0 ? Math.round((data.earnedPoints / data.maxPoints) * 100) : 0;
  const passed = data.passed ?? scorePercent >= 70;

  return (
    <Box sx={{ p: 4, maxWidth: 800, mx: "auto" }}>
      <Typography variant="h5" fontWeight={500} sx={{ mb: 3 }}>
        Quiz Results — {data.quizTitle || "Results"}
      </Typography>

      {/* Score card */}
      <Card variant="outlined" sx={{ mb: 3 }}>
        <CardContent>
          <Stack direction="row" justifyContent="space-between" alignItems="center">
            <Box>
              <Typography variant="h3" fontWeight={600}>{scorePercent}%</Typography>
              <Typography variant="body2" color="text.secondary">
                {data.earnedPoints} / {data.maxPoints} points
              </Typography>
            </Box>
            <Chip
              icon={passed ? <CheckCircleOutlineIcon /> : <CancelOutlinedIcon />}
              label={passed ? "Passed" : "Not passed"}
              color={passed ? "success" : "error"}
              variant="outlined"
            />
          </Stack>
        </CardContent>
      </Card>

      {/* Answer review */}
      <Typography variant="subtitle1" fontWeight={600} sx={{ mb: 2 }}>Answer review</Typography>
      <Stack spacing={1.5} sx={{ mb: 4 }}>
        {answers.map((a, i) => (
          <Card key={a.qid} variant="outlined">
            <CardContent>
              <Box sx={{ display: "flex", justifyContent: "space-between", mb: 1 }}>
                <Typography variant="body2" fontWeight={500}>Q{i + 1}. {a.prompt}</Typography>
                <Stack direction="row" spacing={0.75} alignItems="center">
                  {a.gradeStatus === "pending_manual" ? (
                    <Chip label="Pending review" size="small" variant="outlined" />
                  ) : (
                    <Chip
                      label={`${a.points}/${a.max}`}
                      size="small"
                      color={a.correct ? "success" : "error"}
                      variant="outlined"
                    />
                  )}
                </Stack>
              </Box>
              <Typography variant="caption" color="text.secondary">Your answer: {a.given || "—"}</Typography>
              {a.explanation && (
                <>
                  <Divider sx={{ my: 1 }} />
                  <Typography variant="caption" color="text.secondary">{a.explanation}</Typography>
                </>
              )}
            </CardContent>
          </Card>
        ))}
      </Stack>

      {/* Footer */}
      <Stack direction="row" spacing={2} justifyContent="center">
        <Button variant="outlined" onClick={() => router.push("/library")}>Back to library</Button>
        <Button variant="outlined" startIcon={<ShareOutlinedIcon />} onClick={() => setShareOpen(true)}>Share</Button>
      </Stack>

      {shareOpen && <ShareModal quizId={data.quizId ?? ""} onClose={() => setShareOpen(false)} />}
    </Box>
  );
```

- [ ] **Step 5: Type-check + commit**

```bash
cd web && mise exec -- bun run type-check
git add "web/app/(learner)/sessions/[id]/page.tsx" "web/app/(learner)/sessions/[id]/results/page.tsx"
git commit -m "feat: rewrite active session and results pages with MUI"
```

---

## Task 11: Rewrite Remaining Pages

**Files:**
- Modify: `web/app/(learner)/agent/page.tsx`
- Modify: `web/app/(learner)/grading/page.tsx`
- Modify: `web/app/(learner)/plans/[id]/page.tsx`
- Modify: `web/app/(learner)/quizzes/[id]/preview/page.tsx`
- Modify: `web/app/(learner)/practice/page.tsx`
- Modify: `web/app/(learner)/author/[quizId]/page.tsx`

**Pattern for each page:** Replace `@/components/ui` imports with MUI equivalents. Replace inline `style={{...}}` JSX with `Box`/`Stack`/`Typography`/`Card`/`CardContent`/`Button`/`Chip`/`TextField`. Keep all business logic unchanged.

Import mapping reminder:
- `Button` → `@mui/material/Button`
- `Card` → `@mui/material/Card` + `CardContent`
- `Tag` → `@mui/material/Chip`
- `KV` → `Box` + two `Typography` lines (label + value)
- `Icon name="X"` → matching `@mui/icons-material` component

- [ ] **Step 1: Rewrite `web/app/(learner)/agent/page.tsx`**

Update imports (remove `@/components/ui`, add MUI). Replace JSX return — keep all API key fetching/creation, MCP tool listing logic. Key layout:

```tsx
// Imports to add:
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import TextField from "@mui/material/TextField";
import Divider from "@mui/material/Divider";
import CircularProgress from "@mui/material/CircularProgress";
import ContentCopyOutlinedIcon from "@mui/icons-material/ContentCopyOutlined";
import SmartToyOutlinedIcon from "@mui/icons-material/SmartToyOutlined";

// Return structure:
return (
  <Box sx={{ p: 4 }}>
    <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 3 }}>
      <SmartToyOutlinedIcon color="primary" />
      <Typography variant="h5" fontWeight={500}>Agent API</Typography>
    </Box>

    {/* API Keys section */}
    <Typography variant="subtitle1" fontWeight={600} sx={{ mb: 1.5 }}>API keys</Typography>
    <Stack spacing={1.5} sx={{ mb: 3 }}>
      {keys.map((k) => (
        <Card key={k.id} variant="outlined">
          <CardContent sx={{ display: "flex", justifyContent: "space-between", alignItems: "center", pb: "12px !important" }}>
            <Box>
              <Typography variant="body2" fontWeight={500}>{k.name}</Typography>
              <Typography variant="caption" color="text.secondary" sx={{ fontFamily: "monospace" }}>{k.prefix}…</Typography>
            </Box>
            <Stack direction="row" spacing={0.75}>
              {k.scopes.map((s) => <Chip key={s} label={s} size="small" variant="outlined" />)}
            </Stack>
          </CardContent>
        </Card>
      ))}
    </Stack>

    <Stack direction="row" spacing={1.5} sx={{ mb: 4 }}>
      <TextField label="Key name" value={newKeyName} onChange={(e) => setNewKeyName(e.target.value)} size="small" />
      <Button variant="contained" disabled={creatingKey || !newKeyName} onClick={handleCreateKey}>
        {creatingKey ? "Creating…" : "Create key"}
      </Button>
    </Stack>

    {/* MCP tools */}
    <Divider sx={{ mb: 3 }} />
    <Typography variant="subtitle1" fontWeight={600} sx={{ mb: 1.5 }}>MCP tools</Typography>
    {mcpLoading ? <CircularProgress size={20} /> : (
      <Stack spacing={1}>
        {mcpTools.map((t) => (
          <Card key={t.name} variant="outlined">
            <CardContent sx={{ pb: "12px !important" }}>
              <Box sx={{ display: "flex", justifyContent: "space-between" }}>
                <Typography variant="body2" fontWeight={500} sx={{ fontFamily: "monospace" }}>{t.name}</Typography>
                <Button size="small" startIcon={<ContentCopyOutlinedIcon />} onClick={() => navigator.clipboard.writeText(t.name)}>Copy</Button>
              </Box>
              <Typography variant="caption" color="text.secondary">{t.description}</Typography>
            </CardContent>
          </Card>
        ))}
      </Stack>
    )}
  </Box>
);
```

- [ ] **Step 2: Rewrite `web/app/(learner)/grading/page.tsx`**

```tsx
// Imports to add:
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import TextField from "@mui/material/TextField";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";

// Return structure (keep all grading handlers):
return (
  <Box sx={{ p: 4 }}>
    <Typography variant="h5" fontWeight={500} sx={{ mb: 3 }}>Grading queue</Typography>
    {loading ? (
      <CircularProgress size={24} />
    ) : pending.length === 0 ? (
      <Typography color="text.secondary">No attempts pending manual review.</Typography>
    ) : (
      <Stack spacing={2}>
        {pending.map((attempt) => (
          <Card key={attempt.attempt_id} variant="outlined">
            <CardContent>
              <Box sx={{ display: "flex", justifyContent: "space-between", mb: 1 }}>
                <Typography variant="body2" fontWeight={500}>{attempt.user_display_name}</Typography>
                <Typography variant="caption" color="text.secondary">{attempt.response_word_count} words</Typography>
              </Box>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>{attempt.question_prompt}</Typography>
              <Typography variant="body2" sx={{ mb: 2, p: 1.5, bgcolor: "action.hover", borderRadius: 1 }}>
                {attempt.response_body}
              </Typography>
              <Stack direction="row" spacing={1.5} alignItems="center">
                <TextField
                  label="Score"
                  type="number"
                  size="small"
                  value={gradeStates[attempt.attempt_id]?.score ?? ""}
                  onChange={(e) => setGradeStates((s) => ({ ...s, [attempt.attempt_id]: { ...s[attempt.attempt_id], score: Number(e.target.value) } }))}
                  sx={{ width: 80 }}
                />
                <TextField
                  label="Feedback"
                  size="small"
                  value={gradeStates[attempt.attempt_id]?.feedback ?? ""}
                  onChange={(e) => setGradeStates((s) => ({ ...s, [attempt.attempt_id]: { ...s[attempt.attempt_id], feedback: e.target.value } }))}
                  sx={{ flex: 1 }}
                />
                <Button variant="contained" size="small" disabled={submitting === attempt.attempt_id} onClick={() => handleGrade(attempt.attempt_id)}>
                  {submitting === attempt.attempt_id ? "Saving…" : "Submit"}
                </Button>
              </Stack>
              {errors[attempt.attempt_id] && <Alert severity="error" sx={{ mt: 1 }}>{errors[attempt.attempt_id]}</Alert>}
            </CardContent>
          </Card>
        ))}
      </Stack>
    )}
  </Box>
);
```

- [ ] **Step 3: Rewrite `web/app/(learner)/plans/[id]/page.tsx`**

```tsx
// Imports to add (remove @/components/ui):
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import CircularProgress from "@mui/material/CircularProgress";
import ArrowBackOutlinedIcon from "@mui/icons-material/ArrowBackOutlined";

// Return structure:
return (
  <Box sx={{ p: 4, maxWidth: 720, mx: "auto" }}>
    <Button startIcon={<ArrowBackOutlinedIcon />} onClick={() => router.back()} sx={{ mb: 3 }}>
      Back
    </Button>
    {loading ? (
      <CircularProgress />
    ) : !plan ? (
      <Typography color="text.secondary">Plan not found.</Typography>
    ) : (
      <>
        <Typography variant="h5" fontWeight={500} sx={{ mb: 0.5 }}>{plan.goal}</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
          {plan.lookback_days}-day lookback · Generated {new Date(plan.generated_at).toLocaleDateString()}
        </Typography>
        <Stack spacing={2}>
          {plan.weeks.map((week) => (
            <Card key={week.week_num} variant="outlined">
              <CardContent>
                <Box sx={{ display: "flex", justifyContent: "space-between", mb: 1 }}>
                  <Typography variant="subtitle2" fontWeight={600}>Week {week.week_num}</Typography>
                  <Chip label={week.focus} size="small" variant="outlined" />
                </Box>
                <Stack spacing={0.75}>
                  {week.items.map((item, i) => (
                    <Box key={i} sx={{ display: "flex", justifyContent: "space-between" }}>
                      <Typography variant="body2">{item.kind}: {item.ref_id}</Typography>
                      <Typography variant="caption" color="text.secondary">{item.hours_est}h</Typography>
                    </Box>
                  ))}
                </Stack>
              </CardContent>
            </Card>
          ))}
        </Stack>
      </>
    )}
  </Box>
);
```

- [ ] **Step 4: Rewrite `web/app/(learner)/quizzes/[id]/preview/page.tsx`**

```tsx
// Imports (remove @/components/ui):
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import CircularProgress from "@mui/material/CircularProgress";
import ArrowBackOutlinedIcon from "@mui/icons-material/ArrowBackOutlined";
import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";

// Return structure:
return (
  <Box sx={{ p: 4, maxWidth: 800, mx: "auto" }}>
    <Button startIcon={<ArrowBackOutlinedIcon />} onClick={() => router.back()} sx={{ mb: 3 }}>Back</Button>
    {loading ? (
      <CircularProgress />
    ) : !quiz ? (
      <Typography color="text.secondary">Quiz not found.</Typography>
    ) : (
      <>
        <Box sx={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", mb: 3 }}>
          <Box>
            <Typography variant="h5" fontWeight={500}>{quiz.title}</Typography>
            {quiz.course && <Chip label={quiz.course} size="small" variant="outlined" sx={{ mt: 0.75 }} />}
          </Box>
          <Button variant="contained" startIcon={<PlayArrowOutlinedIcon />} onClick={handleStart} disabled={starting}>
            {starting ? "Starting…" : "Start quiz"}
          </Button>
        </Box>
        <Stack spacing={1.5}>
          {quiz.questions.map((q, i) => (
            <Card key={q.id} variant="outlined">
              <CardContent sx={{ pb: "12px !important" }}>
                <Box sx={{ display: "flex", justifyContent: "space-between" }}>
                  <Typography variant="body2">{i + 1}. {q.prompt}</Typography>
                  <Stack direction="row" spacing={0.75}>
                    <Chip label={KIND_LABEL[q.kind] ?? q.kind} size="small" variant="outlined" />
                    <Chip label={`${q.points}pt`} size="small" variant="outlined" />
                  </Stack>
                </Box>
              </CardContent>
            </Card>
          ))}
        </Stack>
      </>
    )}
  </Box>
);
```

- [ ] **Step 5: Rewrite `web/app/(learner)/practice/page.tsx`**

This page has no `@/components/ui` imports — only add MUI layout imports and replace inline `style={{...}}`:

```tsx
// Add imports:
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import TextField from "@mui/material/TextField";
import FormControlLabel from "@mui/material/FormControlLabel";
import Checkbox from "@mui/material/Checkbox";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";

// Return structure (keep all form handlers):
return (
  <Box sx={{ p: 4, maxWidth: 600, mx: "auto" }}>
    <Typography variant="h5" fontWeight={500} sx={{ mb: 3 }}>Practice quiz</Typography>
    <Box component="form" onSubmit={handleSubmit}>
      <Stack spacing={3}>
        <Box>
          <Typography variant="subtitle2" sx={{ mb: 1 }}>Tags</Typography>
          <Stack direction="row" flexWrap="wrap" gap={0.75}>
            {(tags ?? []).map((t) => (
              <Chip
                key={t.name}
                label={t.name}
                onClick={() => setSelectedTags((prev) => prev.includes(t.name) ? prev.filter((x) => x !== t.name) : [...prev, t.name])}
                color={selectedTags.includes(t.name) ? "primary" : "default"}
                variant={selectedTags.includes(t.name) ? "filled" : "outlined"}
              />
            ))}
          </Stack>
        </Box>

        <Box>
          <Typography variant="subtitle2" sx={{ mb: 1 }}>Question types</Typography>
          <Stack direction="row" flexWrap="wrap" gap={0.75}>
            {QUESTION_TYPES.map((qt) => (
              <FormControlLabel
                key={qt.value}
                control={
                  <Checkbox
                    checked={selectedTypes.includes(qt.value)}
                    onChange={(e) => setSelectedTypes((prev) => e.target.checked ? [...prev, qt.value] : prev.filter((x) => x !== qt.value))}
                    size="small"
                  />
                }
                label={qt.label}
              />
            ))}
          </Stack>
        </Box>

        <Stack direction="row" spacing={2}>
          <TextField
            label="Questions"
            type="number"
            value={count}
            onChange={(e) => setCount(Number(e.target.value))}
            size="small"
            sx={{ width: 120 }}
          />
          <TextField
            label="Duration (min)"
            type="number"
            value={duration}
            onChange={(e) => setDuration(e.target.value === "" ? "" : Number(e.target.value))}
            size="small"
            sx={{ width: 140 }}
          />
        </Stack>

        {error && <Alert severity="error">{error}</Alert>}

        <Button type="submit" variant="contained" disabled={loading} sx={{ alignSelf: "flex-start" }}>
          {loading ? "Starting…" : "Start practice"}
        </Button>
      </Stack>
    </Box>
  </Box>
);
```

- [ ] **Step 6: Rewrite `web/app/(learner)/author/[quizId]/page.tsx`**

This is the largest file (970 lines). Keep all state and handlers. Replace `@/components/ui` imports and update the JSX. `KV` replacement: wherever `<KV label="X" value={y} />` appears, replace with:
```tsx
<Box>
  <Typography variant="caption" color="text.secondary">{label}</Typography>
  <Typography variant="body2">{value}</Typography>
</Box>
```

```tsx
// Imports to replace:
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import TextField from "@mui/material/TextField";
import Select from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import FormControl from "@mui/material/FormControl";
import InputLabel from "@mui/material/InputLabel";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import Divider from "@mui/material/Divider";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import AddOutlinedIcon from "@mui/icons-material/AddOutlined";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import AutoFixHighOutlinedIcon from "@mui/icons-material/AutoFixHighOutlined";
```

Top-level layout is a two-column split (quiz metadata left, questions list right). Use:
```tsx
return (
  <Box sx={{ display: "flex", height: "100vh", overflow: "hidden" }}>
    {/* Left: quiz metadata */}
    <Box sx={{ width: 320, borderRight: 1, borderColor: "divider", overflowY: "auto", p: 3 }}>
      {/* quiz title, status chip, metadata KVs, generate button */}
    </Box>
    {/* Right: question list */}
    <Box sx={{ flex: 1, overflowY: "auto", p: 3 }}>
      {/* questions, add question form */}
    </Box>
  </Box>
);
```

Expand the interior of each panel to match the existing inline-style content, replacing all `style={{...}}` with `sx={{...}}` equivalents.

- [ ] **Step 7: Type-check**

```bash
cd web && mise exec -- bun run type-check
```

Fix any remaining import errors. Common ones:
- `@/components/ui` imports that were missed
- `var(--...)` CSS variable references in `sx` props — replace with MUI theme values (`"divider"`, `"text.secondary"`, `"primary.main"`, etc.)

- [ ] **Step 8: Commit**

```bash
git add \
  "web/app/(learner)/agent/page.tsx" \
  "web/app/(learner)/grading/page.tsx" \
  "web/app/(learner)/plans/[id]/page.tsx" \
  "web/app/(learner)/quizzes/[id]/preview/page.tsx" \
  "web/app/(learner)/practice/page.tsx" \
  "web/app/(learner)/author/[quizId]/page.tsx"
git commit -m "feat: rewrite remaining learner pages with MUI"
```

---

## Task 12: After-Checkpoint + Regression Gate

**Files:** none modified

- [ ] **Step 1: Capture after screenshots**

```bash
TOKEN=$(curl -s http://127.0.0.1:8080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"learner@example.com","password":"password123"}' | jq -r .token)

bunx @playwright/cli goto http://localhost:3000/login
bunx @playwright/cli screenshot --filename=/tmp/after-login.png
bunx @playwright/cli cookie-set ame_token "$TOKEN" --domain=localhost
bunx @playwright/cli goto http://localhost:3000/library
bunx @playwright/cli screenshot --filename=/tmp/after-library.png
bunx @playwright/cli goto http://localhost:3000/progress
bunx @playwright/cli screenshot --filename=/tmp/after-progress.png
```

- [ ] **Step 2: Review visual diff**

Open and compare `/tmp/before-login.png` vs `/tmp/after-login.png`, etc. Confirm:
- Material Design components visible (MUI buttons, cards, text fields)
- No white screen / crash
- Sidebar renders with MUI Drawer and nav items

- [ ] **Step 3: Run uiux regression suite**

```bash
make uiux
```

Expected: all tests in `web/e2e/uiux-spec.spec.ts` pass. If any fail, investigate whether it's a missing selector or a broken data flow — fix before proceeding.

- [ ] **Step 4: Run full check**

```bash
make check
```

Expected: fmt-check + lint + unit tests all pass.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: post-migration cleanup — remove stale CSS vars, fix type-check"
```

(Only if there are unstaged changes after the above tasks. If clean, skip.)
