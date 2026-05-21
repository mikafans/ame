"use client";

import { useEffect, useState } from "react";
import { useRouter, usePathname } from "next/navigation";
import { useAuth } from "@/hooks/useAuth";

const DEMO_MODE = process.env.NEXT_PUBLIC_DEMO_MODE === "1";

interface NavItem {
  href: string;
  label: string;
  section: string;
  roles?: string[];
}

const NAV: NavItem[] = [
  // Learn
  { href: "/library", label: "Library", section: "Learn" },
  { href: "/exams", label: "Exams", section: "Learn" },
  { href: "/practice", label: "Practice", section: "Learn" },
  { href: "/progress", label: "Progress", section: "Learn" },
  // Teach — instructor / admin only
  {
    href: "/author",
    label: "Author studio",
    section: "Teach",
    roles: ["instructor", "admin"],
  },
  // Integrate — instructor / admin only
  {
    href: "/agent",
    label: "Agent API",
    section: "Integrate",
    roles: ["instructor", "admin"],
  },
];

export default function LearnerLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { user, loading } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  // Tweaks panel state (demo-only)
  const [theme, setTheme] = useState("slate");
  const [statsDepth, setStatsDepth] = useState<"minimal" | "standard" | "full">(
    "standard",
  );
  const [showTweaks, setShowTweaks] = useState(false);

  useEffect(() => {
    if (!loading && !user) {
      router.replace("/login");
    }
  }, [user, loading, router]);

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

  if (loading) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Loading…
      </div>
    );
  }

  if (!user) return null;

  const role = user.role;
  const canSeeItem = (item: NavItem) =>
    !item.roles || item.roles.includes(role);

  const sections = ["Learn", "Teach", "Integrate"];

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "220px 1fr",
        minHeight: "100vh",
      }}
    >
      {/* Sidebar */}
      <nav
        style={{
          background: "var(--surface)",
          borderRight: "1px solid var(--border)",
          padding: "20px 12px",
          display: "flex",
          flexDirection: "column",
          gap: 0,
          height: "100vh",
          position: "sticky",
          top: 0,
        }}
      >
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 11,
            letterSpacing: 1.5,
            textTransform: "uppercase",
            color: "var(--accent)",
            marginBottom: 20,
            padding: "0 10px",
          }}
        >
          Harus
        </div>

        {sections.map((sec) => {
          const items = NAV.filter((n) => n.section === sec && canSeeItem(n));
          if (items.length === 0) return null;
          return (
            <div key={sec} style={{ marginBottom: 16 }}>
              <div
                style={{
                  padding: "4px 10px 6px",
                  fontFamily: "var(--mono)",
                  fontSize: 10,
                  letterSpacing: 1.4,
                  color: "var(--muted)",
                  textTransform: "uppercase",
                }}
              >
                {sec}
              </div>
              {items.map((item) => {
                const active = pathname.startsWith(item.href);
                return (
                  <a
                    key={item.href}
                    href={item.href}
                    style={{
                      display: "block",
                      padding: "8px 10px",
                      borderRadius: 6,
                      color: active ? "var(--accent)" : "var(--text-2)",
                      textDecoration: "none",
                      fontSize: 13,
                      fontWeight: active ? 600 : 500,
                      background: active
                        ? "var(--accent-dim, rgba(0,200,100,0.08))"
                        : "transparent",
                      borderLeft: `2px solid ${active ? "var(--accent)" : "transparent"}`,
                      marginBottom: 2,
                    }}
                  >
                    {item.label}
                  </a>
                );
              })}
            </div>
          );
        })}

        {/* Footer */}
        <div
          style={{
            marginTop: "auto",
            borderTop: "1px solid var(--border)",
            paddingTop: 14,
          }}
        >
          <div
            style={{
              padding: "0 10px",
              color: "var(--muted)",
              fontSize: 12,
              marginBottom: 4,
            }}
          >
            {user.displayName}
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 10,
                marginLeft: 6,
                textTransform: "uppercase",
                letterSpacing: 0.8,
              }}
            >
              {role}
            </span>
          </div>
          {DEMO_MODE && (
            <button
              onClick={() => setShowTweaks(true)}
              style={{
                display: "block",
                width: "100%",
                padding: "5px 10px",
                textAlign: "left",
                background: "transparent",
                border: "none",
                color: "var(--muted)",
                fontSize: 11,
                fontFamily: "var(--mono)",
                cursor: "pointer",
                marginBottom: 4,
              }}
            >
              ⚙ Tweaks (demo)
            </button>
          )}
          <a
            href="/login"
            onClick={() => {
              document.cookie = "ame_token=; path=/; max-age=0";
            }}
            style={{
              display: "block",
              padding: "5px 10px",
              color: "var(--muted)",
              fontSize: 12,
              textDecoration: "none",
            }}
          >
            Sign out
          </a>
        </div>
      </nav>

      {/* Main */}
      <main style={{ overflowY: "auto" }}>{children}</main>

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
