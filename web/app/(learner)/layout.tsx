"use client";

import { useEffect, useState } from "react";
import { useRouter, usePathname } from "next/navigation";
import { useAuth } from "@/hooks/useAuth";
import { Sidebar } from "@/components/Sidebar";

const DEMO_MODE = process.env.NEXT_PUBLIC_DEMO_MODE === "1";

export default function LearnerLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { user } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  // Tweaks panel state (demo-only)
  const [theme, setTheme] = useState("slate");
  const [statsDepth, setStatsDepth] = useState<"minimal" | "standard" | "full">(
    "standard",
  );
  const [showTweaks, setShowTweaks] = useState(false);

  // Apply theme
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    try {
      localStorage.setItem("harus.theme", theme);
    } catch {
      // ignore
    }
  }, [theme]);

  // Read persisted theme on mount
  useEffect(() => {
    try {
      const saved = localStorage.getItem("harus.theme");
      if (saved) setTheme(saved);
    } catch {
      // ignore
    }
  }, []);

  const role = user?.role ?? "learner";

  // Map pathname to route ID for Sidebar
  const getRouteId = () => {
    if (pathname.startsWith("/library")) return "library";
    if (pathname.startsWith("/exams")) return "exams";
    if (pathname.startsWith("/practice")) return "quiz";
    if (pathname.startsWith("/results")) return "results";
    if (pathname.startsWith("/progress")) return "dashboard";
    if (pathname.startsWith("/author")) return "author";
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
      agent: "/agent",
    };
    router.push(routeMap[route] || "/library");
  };

  return (
    <div
      style={{
        display: "flex",
        minHeight: "100vh",
      }}
    >
      <Sidebar
        route={currentRoute}
        setRoute={handleRouteChange}
        showAgent={role !== "learner"}
      />

      {/* Main */}
      <main style={{ flex: 1, overflowY: "auto" }}>{children}</main>

      {/* Tweaks panel (demo-only, never in production) */}
      {DEMO_MODE && showTweaks && (
        <TweaksPanel
          theme={theme}
          setTheme={setTheme}
          statsDepth={statsDepth}
          setStatsDepth={setStatsDepth}
          onClose={() => setShowTweaks(false)}
        />
      )}
    </div>
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
  // Runtime assertion — this component must never mount in production.
  if (process.env.NODE_ENV === "production") {
    throw new Error(
      "TweaksPanel must not be rendered in production. Check NEXT_PUBLIC_DEMO_MODE.",
    );
  }

  return (
    <div
      style={{
        position: "fixed",
        bottom: 24,
        right: 24,
        background: "var(--surface)",
        border: "1px solid var(--accent)",
        borderRadius: 8,
        padding: 20,
        width: 260,
        zIndex: 200,
        boxShadow: "0 8px 32px rgba(0,0,0,0.4)",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 16,
        }}
      >
        <div>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 9,
              letterSpacing: 1.5,
              textTransform: "uppercase",
              color: "var(--accent)",
              marginBottom: 2,
            }}
          >
            Demo only
          </div>
          <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text)" }}>
            Tweaks
          </div>
        </div>
        <button
          onClick={onClose}
          style={{
            background: "transparent",
            border: "none",
            color: "var(--muted)",
            fontSize: 16,
            cursor: "pointer",
            lineHeight: 1,
          }}
        >
          ×
        </button>
      </div>

      <div style={{ marginBottom: 16 }}>
        <div style={tweakLabel}>Theme</div>
        <div style={{ display: "flex", gap: 6 }}>
          {["slate", "paper", "cobalt"].map((t) => (
            <button
              key={t}
              onClick={() => setTheme(t)}
              style={{
                flex: 1,
                padding: "6px 4px",
                background:
                  theme === t
                    ? "var(--accent-dim, rgba(0,200,100,0.15))"
                    : "var(--surface-2)",
                border: `1px solid ${theme === t ? "var(--accent)" : "var(--border)"}`,
                borderRadius: 4,
                color: theme === t ? "var(--accent)" : "var(--text-2)",
                fontSize: 11,
                cursor: "pointer",
                fontFamily: "var(--mono)",
              }}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      <div>
        <div style={tweakLabel}>Stats depth</div>
        <div style={{ display: "flex", gap: 6 }}>
          {(["minimal", "standard", "full"] as const).map((d) => (
            <button
              key={d}
              onClick={() => setStatsDepth(d)}
              style={{
                flex: 1,
                padding: "6px 4px",
                background:
                  statsDepth === d
                    ? "var(--accent-dim, rgba(0,200,100,0.15))"
                    : "var(--surface-2)",
                border: `1px solid ${statsDepth === d ? "var(--accent)" : "var(--border)"}`,
                borderRadius: 4,
                color: statsDepth === d ? "var(--accent)" : "var(--text-2)",
                fontSize: 10,
                cursor: "pointer",
                fontFamily: "var(--mono)",
              }}
            >
              {d}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

const tweakLabel: React.CSSProperties = {
  fontFamily: "var(--mono)",
  fontSize: 10,
  letterSpacing: 1.2,
  textTransform: "uppercase",
  color: "var(--muted)",
  marginBottom: 8,
};
