"use client";

interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}

export function EssayRenderer({ value, onChange, disabled = false }: Props) {
  const wordCount = value.trim() ? value.trim().split(/\s+/).length : 0;

  return (
    <div>
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        rows={8}
        placeholder="Write your answer here…"
        style={{
          width: "100%",
          padding: "12px 14px",
          background: "var(--surface-2)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          color: "var(--text)",
          fontSize: 14,
          lineHeight: 1.6,
          resize: "vertical",
          boxSizing: "border-box",
          outline: "none",
          fontFamily: "inherit",
        }}
      />
      <div
        style={{
          textAlign: "right",
          fontFamily: "var(--mono)",
          fontSize: 11,
          color: "var(--muted)",
          marginTop: 4,
        }}
      >
        {wordCount} word{wordCount !== 1 ? "s" : ""}
      </div>
    </div>
  );
}
