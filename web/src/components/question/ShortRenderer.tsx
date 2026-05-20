"use client";

interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}

export function ShortRenderer({ value, onChange, disabled = false }: Props) {
  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      placeholder="Your answer…"
      style={{
        width: "100%",
        padding: "12px 14px",
        background: "var(--surface-2)",
        border: "1px solid var(--border)",
        borderRadius: 6,
        color: "var(--text)",
        fontSize: 14,
        boxSizing: "border-box",
        outline: "none",
      }}
    />
  );
}
