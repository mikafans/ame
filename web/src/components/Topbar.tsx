"use client";

import React from "react";
import { Icon } from "@/components/ui";

interface TopbarProps {
  title: string;
  subtitle?: string;
  breadcrumb?: string;
  actions?: React.ReactNode;
}

export function Topbar({ title, subtitle, breadcrumb, actions }: TopbarProps) {
  return (
    <header
      style={{
        padding: "20px 36px",
        borderBottom: "1px solid var(--border)",
        background: "var(--bg)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        position: "sticky",
        top: 0,
        zIndex: 5,
      }}
    >
      <div>
        {breadcrumb ? (
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 10,
              letterSpacing: 1.3,
              textTransform: "uppercase",
              color: "var(--muted)",
              marginBottom: 6,
            }}
          >
            {breadcrumb}
          </div>
        ) : null}
        <h1
          style={{
            margin: 0,
            fontFamily: "var(--serif)",
            fontWeight: 500,
            fontSize: 26,
            letterSpacing: -0.3,
            color: "var(--text)",
          }}
        >
          {title}
        </h1>
        {subtitle ? (
          <div
            style={{
              marginTop: 4,
              color: "var(--muted)",
              fontSize: 13,
            }}
          >
            {subtitle}
          </div>
        ) : null}
      </div>

      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}
      >
        {actions}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            paddingLeft: 14,
            borderLeft: "1px solid var(--border)",
          }}
        >
          <Icon name="bell" size={16} color="var(--muted)" />
          <Icon name="search" size={16} color="var(--muted)" />
        </div>
      </div>
    </header>
  );
}
