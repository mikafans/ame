"use client";

interface McqOption {
  text: string;
  index: number;
}

interface Props {
  options: McqOption[];
  value: number | null;
  onChange: (position: number) => void;
  disabled?: boolean;
}

export function McqRenderer({
  options,
  value,
  onChange,
  disabled = false,
}: Props) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {options.map((opt) => {
        const selected = value === opt.index;
        return (
          <button
            key={opt.index}
            type="button"
            onClick={() => !disabled && onChange(opt.index)}
            style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 12,
              padding: "14px 16px",
              background: selected ? "var(--accent-dim)" : "var(--surface-2)",
              border: `1px solid ${selected ? "var(--accent-line)" : "var(--border)"}`,
              borderRadius: 6,
              cursor: disabled ? "not-allowed" : "pointer",
              textAlign: "left",
              width: "100%",
            }}
          >
            <span
              style={{
                flexShrink: 0,
                width: 22,
                height: 22,
                borderRadius: "50%",
                border: `2px solid ${selected ? "var(--accent)" : "var(--border-strong)"}`,
                background: selected ? "var(--accent)" : "transparent",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {selected && (
                <span
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    background: "#000",
                  }}
                />
              )}
            </span>
            <span
              style={{ color: "var(--text)", fontSize: 14, lineHeight: 1.5 }}
            >
              {opt.text}
            </span>
          </button>
        );
      })}
    </div>
  );
}
