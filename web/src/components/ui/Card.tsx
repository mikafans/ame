"use client";

import React from "react";

interface CardProps {
  accent?: string;
  style?: React.CSSProperties;
  children?: React.ReactNode;
}

export function Card({ accent, style, children }: CardProps) {
  const baseStyles: React.CSSProperties = {
    background: "var(--surface)",
    border: "1px solid var(--border)",
    borderRadius: "var(--radius)",
    padding: 16,
    ...(accent && { borderLeft: `3px solid ${accent}` }),
    ...style,
  };

  return <div style={baseStyles}>{children}</div>;
}
