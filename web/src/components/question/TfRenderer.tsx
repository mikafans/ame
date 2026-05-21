"use client";

interface Props {
  value: boolean | null;
  onChange: (v: boolean | null) => void;
  disabled?: boolean;
}

export function TfRenderer({ value, onChange, disabled = false }: Props) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "1fr 1fr",
        gap: 12,
      }}
    >
      {[
        { v: true, label: "True" },
        { v: false, label: "False" },
      ].map(({ v, label }) => {
        const selected = value === v;
        return (
          <button
            key={label}
            type="button"
            onClick={() => !disabled && onChange(v)}
            style={{
              padding: "26px 18px",
              background: selected ? "var(--accent-dim)" : "var(--surface-2)",
              border: `1px solid ${selected ? "var(--accent-line)" : "var(--border)"}`,
              color: selected ? "var(--accent)" : "var(--text-2)",
              borderRadius: 6,
              fontFamily: "var(--serif)",
              fontSize: 24,
              fontWeight: 500,
              cursor: disabled ? "not-allowed" : "pointer",
            }}
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}
