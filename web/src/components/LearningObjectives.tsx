"use client";

interface Props {
  items: string[];
  kicker?: string;
  compact?: boolean;
  accentBars?: boolean;
}

export function LearningObjectives({
  items,
  kicker = "What you'll learn",
  compact = false,
  accentBars = true,
}: Props) {
  if (!items.length) return null;

  const overLimit = items.length > 6;

  if (compact) {
    return (
      <div>
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.3,
            textTransform: "uppercase",
            color: "var(--muted)",
            marginBottom: 8,
          }}
        >
          {kicker}
        </div>
        <ul
          style={{
            listStyle: "none",
            padding: 0,
            margin: 0,
            display: "flex",
            flexDirection: "column",
            gap: 8,
          }}
        >
          {items.map((it, i) => (
            <li
              key={i}
              style={{
                display: "flex",
                gap: 10,
                alignItems: "flex-start",
                fontSize: 12.5,
                color: "var(--text-2)",
                lineHeight: 1.5,
              }}
            >
              <span style={{ color: "var(--accent)", flexShrink: 0 }}>✓</span>
              <span style={{ flex: 1 }}>{it}</span>
            </li>
          ))}
        </ul>
        {overLimit && (
          <span
            style={{
              display: "inline-block",
              marginTop: 8,
              padding: "2px 8px",
              background: "var(--amber-dim)",
              color: "var(--amber)",
              border: "1px solid var(--amber)",
              borderRadius: 4,
              fontSize: 11,
              fontFamily: "var(--mono)",
            }}
          >
            {items.length} objectives — consider splitting
          </span>
        )}
      </div>
    );
  }

  return (
    <div
      style={{
        padding: "18px 20px",
        background: "var(--surface-2)",
        border: "1px solid var(--border)",
        borderRadius: "var(--radius)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          marginBottom: 12,
        }}
      >
        <span style={{ color: "var(--accent)" }}>✦</span>
        <span
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10.5,
            letterSpacing: 1.3,
            textTransform: "uppercase",
            color: "var(--muted)",
          }}
        >
          {kicker}
        </span>
        {overLimit && (
          <span
            style={{
              marginLeft: "auto",
              padding: "2px 8px",
              background: "var(--amber-dim)",
              color: "var(--amber)",
              border: "1px solid var(--amber)",
              borderRadius: 4,
              fontSize: 11,
              fontFamily: "var(--mono)",
            }}
          >
            {items.length} objectives
          </span>
        )}
      </div>
      <ul
        style={{
          listStyle: "none",
          padding: 0,
          margin: 0,
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 10,
        }}
      >
        {items.map((it, i) => (
          <li
            key={i}
            style={{
              display: "flex",
              gap: 10,
              alignItems: "flex-start",
              paddingLeft: accentBars ? 10 : 0,
              borderLeft: accentBars ? "2px solid var(--accent)" : "none",
              fontSize: 13,
              lineHeight: 1.5,
            }}
          >
            <span
              style={{
                fontFamily: "var(--mono)",
                fontSize: 11,
                color: "var(--muted)",
                letterSpacing: 0.5,
                paddingTop: 2,
                flexShrink: 0,
              }}
            >
              {String(i + 1).padStart(2, "0")}
            </span>
            <span style={{ flex: 1, color: "var(--text-2)" }}>{it}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
