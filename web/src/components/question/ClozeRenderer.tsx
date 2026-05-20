"use client";

interface Props {
  prompt: string;
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}

export function ClozeRenderer({
  prompt,
  value,
  onChange,
  disabled = false,
}: Props) {
  // Split prompt on ___ to render inline blanks
  const parts = prompt.split("___");

  if (parts.length <= 1) {
    return (
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        placeholder="Fill in the blank…"
        style={{
          width: "100%",
          padding: "12px 14px",
          background: "var(--surface-2)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          color: "var(--text)",
          fontSize: 14,
          boxSizing: "border-box",
        }}
      />
    );
  }

  return (
    <div style={{ fontSize: 15, lineHeight: 2, color: "var(--text)" }}>
      {parts.map((part, i) => (
        <span key={i}>
          {part}
          {i < parts.length - 1 && (
            <input
              type="text"
              value={value}
              onChange={(e) => onChange(e.target.value)}
              disabled={disabled}
              placeholder="___"
              style={{
                display: "inline-block",
                width: 160,
                margin: "0 6px",
                padding: "4px 10px",
                background: "var(--surface-2)",
                border: "none",
                borderBottom: "2px solid var(--accent)",
                color: "var(--text)",
                fontSize: 14,
                textAlign: "center",
                outline: "none",
              }}
            />
          )}
        </span>
      ))}
    </div>
  );
}
