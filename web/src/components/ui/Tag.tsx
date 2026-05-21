"use client";

import React from "react";

interface TagProps {
  color?: "accent" | "amber" | "red" | "blue" | "muted";
  style?: React.CSSProperties;
  children?: React.ReactNode;
}

export function Tag({ color = "muted", style, children }: TagProps) {
  const colorStyles: Record<string, React.CSSProperties> = {
    accent: {
      background: "var(--accent-dim)",
      color: "var(--accent)",
      border: "1px solid var(--accent-line)",
    },
    amber: {
      background: "var(--amber-dim)",
      color: "var(--amber)",
      border: "1px solid var(--amber)",
    },
    red: {
      background: "var(--red-dim)",
      color: "var(--red)",
      border: "1px solid var(--red)",
    },
    blue: {
      background: "var(--blue-dim)",
      color: "var(--blue)",
      border: "1px solid var(--blue)",
    },
    muted: {
      background: "var(--surface-2)",
      color: "var(--muted)",
      border: "1px solid var(--border)",
    },
  };

  const baseStyles: React.CSSProperties = {
    display: "inline-flex",
    alignItems: "center",
    gap: 4,
    padding: "3px 7px",
    fontSize: 10,
    fontFamily: "var(--mono)",
    fontWeight: 500,
    letterSpacing: 1,
    textTransform: "uppercase",
    borderRadius: 4,
    ...colorStyles[color],
    ...style,
  };

  return <span style={baseStyles}>{children}</span>;
}
