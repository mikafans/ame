"use client";

import React from "react";
import { Logo, Icon } from "@/components/ui";
import { useAuth } from "@/hooks/useAuth";

interface SidebarProps {
  route: string;
  setRoute: (route: string) => void;
  showAgent?: boolean;
}

export function Sidebar({ route, setRoute, showAgent = false }: SidebarProps) {
  const { user } = useAuth();

  const items = [
    { id: "library", label: "Library", icon: "library", section: "Learn" },
    { id: "exams", label: "Exams", icon: "stack", section: "Learn" },
    { id: "quiz", label: "Take quiz", icon: "take", section: "Learn" },
    { id: "results", label: "Last results", icon: "results", section: "Learn" },
    { id: "dashboard", label: "Progress", icon: "dashboard", section: "Learn" },
    { id: "author", label: "Author studio", icon: "author", section: "Teach" },
    ...(showAgent
      ? [
          {
            id: "agent",
            label: "Agent API",
            icon: "agent",
            section: "Integrate",
          },
        ]
      : []),
  ];

  const sections = ["Learn", "Teach", "Integrate"];

  // Extract initials from displayName
  const initials =
    user?.displayName
      ?.split(" ")
      .map((n) => n[0])
      .join("")
      .toUpperCase()
      .slice(0, 2) || "JT";

  const displayName = user?.displayName || "Jordan Tahir";
  const role = user?.role || "Student";
  const cohort = "CS '27"; // Default cohort

  return (
    <aside
      style={{
        width: 232,
        flex: "0 0 232px",
        borderRight: "1px solid var(--border)",
        background: "var(--surface)",
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        position: "sticky",
        top: 0,
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "20px 20px 16px",
          borderBottom: "1px solid var(--border)",
        }}
      >
        <Logo />
        <div
          style={{
            marginTop: 4,
            fontFamily: "var(--mono)",
            fontSize: 10,
            color: "var(--muted)",
            letterSpacing: 1.2,
            textTransform: "uppercase",
          }}
        >
          Assessment Platform · v2.4
        </div>
      </div>

      {/* Navigation */}
      <nav style={{ flex: 1, overflowY: "auto", padding: "16px 12px" }}>
        {sections.map((sec) => {
          const inSec = items.filter((i) => i.section === sec);
          if (!inSec.length) return null;

          return (
            <div key={sec} style={{ marginBottom: 18 }}>
              <div
                style={{
                  padding: "6px 10px 8px",
                  fontFamily: "var(--mono)",
                  fontSize: 10,
                  letterSpacing: 1.4,
                  color: "var(--muted)",
                  textTransform: "uppercase",
                }}
              >
                {sec}
              </div>

              {inSec.map((it) => {
                const active = route === it.id;
                return (
                  <button
                    key={it.id}
                    onClick={() => setRoute(it.id)}
                    style={{
                      width: "100%",
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      padding: "9px 10px",
                      background: active ? "var(--accent-dim)" : "transparent",
                      border: "1px solid",
                      borderColor: active
                        ? "var(--accent-line)"
                        : "transparent",
                      color: active ? "var(--accent)" : "var(--text-2)",
                      fontSize: 13,
                      fontWeight: active ? 600 : 500,
                      borderRadius: 6,
                      textAlign: "left",
                      marginBottom: 2,
                      cursor: "pointer",
                    }}
                  >
                    <Icon name={it.icon} size={16} />
                    <span>{it.label}</span>
                  </button>
                );
              })}
            </div>
          );
        })}
      </nav>

      {/* Footer */}
      <div
        style={{
          borderTop: "1px solid var(--border)",
          padding: 14,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
          }}
        >
          <div
            style={{
              width: 32,
              height: 32,
              borderRadius: "50%",
              background: "var(--surface-3)",
              border: "1px solid var(--border)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontFamily: "var(--serif)",
              fontSize: 14,
              fontWeight: 500,
              color: "var(--text)",
            }}
          >
            {initials}
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              style={{
                fontSize: 12.5,
                fontWeight: 500,
                color: "var(--text)",
              }}
            >
              {displayName}
            </div>
            <div
              style={{
                fontSize: 11,
                color: "var(--muted)",
              }}
            >
              {role} · {cohort}
            </div>
          </div>
          <Icon name="settings" size={14} color="var(--muted)" />
        </div>
      </div>
    </aside>
  );
}
