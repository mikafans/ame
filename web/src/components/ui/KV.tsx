"use client";

import React from "react";

interface KVProps {
  label: string;
  value: React.ReactNode;
  style?: React.CSSProperties;
}

export function KV({ label, value, style }: KVProps) {
  const baseStyles: React.CSSProperties = {
    display: "flex",
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "baseline",
    ...style,
  };

  return (
    <div style={baseStyles}>
      <span
        style={{
          fontFamily: "var(--mono)",
          fontSize: 10,
          letterSpacing: 1,
          textTransform: "uppercase",
          color: "var(--muted)",
        }}
      >
        {label}
      </span>
      <span
        style={{
          fontSize: 13,
          color: "var(--text)",
          fontWeight: 500,
        }}
      >
        {value}
      </span>
    </div>
  );
}
