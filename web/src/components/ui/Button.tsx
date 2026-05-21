"use client";

import React from "react";

interface ButtonProps {
  variant?: "primary" | "ghost" | "outline";
  size?: "sm" | "md" | "lg";
  icon?: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  style?: React.CSSProperties;
  children?: React.ReactNode;
  type?: "button" | "submit";
}

export function Button({
  variant = "outline",
  size = "md",
  icon,
  onClick,
  disabled = false,
  style,
  children,
  type = "button",
}: ButtonProps) {
  const sizeStyles: Record<string, { padding: string; fontSize: number }> = {
    sm: { padding: "6px 10px", fontSize: 12 },
    md: { padding: "10px 16px", fontSize: 13 },
    lg: { padding: "12px 20px", fontSize: 14 },
  };

  const variantStyles: Record<string, React.CSSProperties> = {
    primary: {
      background: "var(--accent)",
      color: "#0b1410",
      border: "1px solid transparent",
    },
    ghost: {
      background: "transparent",
      color: "var(--text-2)",
      border: "1px solid transparent",
    },
    outline: {
      background: "var(--surface)",
      color: "var(--text)",
      border: "1px solid var(--border)",
    },
  };

  const baseStyles: React.CSSProperties = {
    fontFamily: "var(--sans)",
    fontWeight: 500,
    borderRadius: 6,
    display: "inline-flex",
    alignItems: "center",
    gap: 8,
    letterSpacing: 0.1,
    transition: "background 120ms, border-color 120ms, color 120ms",
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.55 : 1,
    ...sizeStyles[size],
    ...variantStyles[variant],
    ...style,
  };

  return (
    <button
      type={type}
      disabled={disabled}
      onClick={onClick}
      style={baseStyles}
    >
      {icon && <span style={{ display: "inline-flex" }}>{icon}</span>}
      {children}
    </button>
  );
}
