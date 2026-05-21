"use client";

import React from "react";

interface DividerProps {
  label?: string;
  style?: React.CSSProperties;
}

export function Divider({ label, style }: DividerProps) {
  if (!label) {
    return (
      <div
        style={{
          height: 1,
          background: "var(--border)",
          width: "100%",
          ...style,
        }}
      />
    );
  }

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        ...style,
      }}
    >
      <div
        style={{
          flex: 1,
          height: 1,
          background: "var(--border)",
        }}
      />
      <span
        style={{
          fontFamily: "var(--mono)",
          fontSize: 10,
          letterSpacing: 1,
          textTransform: "uppercase",
          color: "var(--muted)",
          whiteSpace: "nowrap",
        }}
      >
        {label}
      </span>
      <div
        style={{
          flex: 1,
          height: 1,
          background: "var(--border)",
        }}
      />
    </div>
  );
}
