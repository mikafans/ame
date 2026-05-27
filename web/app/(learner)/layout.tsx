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

export default function LearnerLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { user, loading } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    if (!loading && !user) router.push("/login");
  }, [loading, user, router]);

  const [theme, setTheme] = useState("slate");
  const [statsDepth, setStatsDepth] = useState<"minimal" | "standard" | "full">(
    "standard",
  );
  const [showTweaks, setShowTweaks] = useState(false);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    try {
      localStorage.setItem("harus.theme", theme);
    } catch {
      /* ignore */
    }
  }, [theme]);

  useEffect(() => {
    try {
      const saved = localStorage.getItem("harus.theme");
      if (saved) setTheme(saved);
    } catch {
      /* ignore */
    }
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
      <Sidebar
        route={currentRoute}
        setRoute={handleRouteChange}
        showAgent={role !== "learner"}
      />
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
      sx={{
        position: "fixed",
        bottom: 24,
        right: 24,
        p: 2.5,
        width: 260,
        zIndex: 200,
      }}
    >
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          mb: 2,
        }}
      >
        <Box>
          <Typography
            variant="caption"
            color="primary"
            sx={{
              letterSpacing: 1.5,
              textTransform: "uppercase",
              display: "block",
            }}
          >
            Demo only
          </Typography>
          <Typography variant="body2" sx={{ fontWeight: 600 }}>
            Tweaks
          </Typography>
        </Box>
        <IconButton size="small" onClick={onClose}>
          <CloseIcon fontSize="small" />
        </IconButton>
      </Box>

      <Box sx={{ mb: 2 }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            display: "block",
            mb: 1,
            letterSpacing: 1.2,
            textTransform: "uppercase",
          }}
        >
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
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            display: "block",
            mb: 1,
            letterSpacing: 1.2,
            textTransform: "uppercase",
          }}
        >
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
