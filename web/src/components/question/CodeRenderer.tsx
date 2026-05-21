"use client";

import { Button } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";

interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
  language?: string;
  filename?: string;
  starter?: string;
}

export function CodeRenderer({
  value,
  onChange,
  disabled = false,
  language = "python",
  filename = "solution.py",
  starter = "",
}: Props) {
  const current = value || starter;

  return (
    <div
      style={{
        background: "var(--surface-2)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        overflow: "hidden",
      }}
    >
      {/* File header */}
      <div
        style={{
          padding: "8px 14px",
          borderBottom: "1px solid var(--border)",
          background: "var(--surface)",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            fontFamily: "var(--mono)",
            fontSize: 11,
            color: "var(--muted)",
            letterSpacing: 0.8,
          }}
        >
          <Icon name="code" size={12} />
          {filename} · {language}
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <Button variant="ghost" size="sm">
            Run tests
          </Button>
          <Button variant="ghost" size="sm">
            Reset
          </Button>
        </div>
      </div>

      {/* Code textarea */}
      <textarea
        value={current}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        spellCheck={false}
        rows={10}
        style={{
          width: "100%",
          background: "transparent",
          color: "var(--text)",
          fontFamily: "var(--mono)",
          fontSize: 13.5,
          lineHeight: 1.6,
          padding: "16px 18px",
          border: "none",
          outline: "none",
          resize: "vertical",
          minHeight: 220,
          boxSizing: "border-box",
        }}
      />

      {/* Test output */}
      <div
        style={{
          padding: "10px 14px",
          borderTop: "1px solid var(--border)",
          background: "var(--surface)",
        }}
      >
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 11,
            color: "var(--muted)",
            marginBottom: 4,
            letterSpacing: 0.8,
          }}
        >
          ◇ Test runner output
        </div>
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 12,
            color: "var(--text-2)",
          }}
        >
          <span style={{ color: "var(--accent)" }}>✓</span> case 1: small graph
          — passed {"  "}
          <span style={{ color: "var(--accent)" }}>✓</span> case 2: disconnected
          — passed {"  "}
          <span style={{ color: "var(--red)" }}>✗</span> case 3: cycle — failed
        </div>
      </div>
    </div>
  );
}
