"use client";

import React from "react";

interface StatProps {
  label: string;
  value: React.ReactNode;
  delta?: string;
  deltaPositive?: boolean;
  style?: React.CSSProperties;
}

export function Stat({ label, value, delta, deltaPositive, style }: StatProps) {
  const deltaColor = deltaPositive ? "var(--accent)" : "var(--red)";

  const baseStyles: React.CSSProperties = {
    ...style,
  };

  return (
    <div style={baseStyles}>
      <div
        style={{
          fontFamily: "var(--mono)",
          fontSize: 10,
          letterSpacing: 1,
          textTransform: "uppercase",
          color: "var(--muted)",
          marginBottom: 6,
        }}
      >
        {label}
      </div>
      <div
        style={{
          fontFamily: "var(--serif)",
          fontSize: 22,
          fontWeight: 500,
          lineHeight: 1,
          color: "var(--text)",
          letterSpacing: -0.5,
          marginBottom: delta ? 6 : 0,
        }}
      >
        {value}
      </div>
      {delta && (
        <div
          style={{
            marginTop: 6,
            fontSize: 12,
            color: deltaColor,
          }}
        >
          {delta}
        </div>
      )}
    </div>
  );
}
