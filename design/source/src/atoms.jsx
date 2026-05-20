// Shared UI atoms for Harus
const atomStyles = {
  panel: {
    background: "var(--surface)",
    border: "1px solid var(--border)",
    borderRadius: "var(--radius-lg)",
  },
};

function Button({ variant = "primary", size = "md", icon, children, onClick, type = "button", style, disabled }) {
  const base = {
    fontFamily: "var(--sans)",
    fontWeight: 500,
    fontSize: size === "sm" ? 12 : 13,
    padding: size === "sm" ? "6px 10px" : size === "lg" ? "11px 18px" : "8px 14px",
    borderRadius: 6,
    border: "1px solid transparent",
    display: "inline-flex",
    alignItems: "center",
    gap: 6,
    letterSpacing: 0.1,
    transition: "background 120ms, border-color 120ms, color 120ms",
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.55 : 1,
  };
  const variants = {
    primary: { background: "var(--accent)", color: "#0b1410", borderColor: "var(--accent)" },
    ghost:   { background: "transparent", color: "var(--text-2)", borderColor: "var(--border)" },
    solid:   { background: "var(--surface-2)", color: "var(--text)", borderColor: "var(--border)" },
    danger:  { background: "transparent", color: "var(--red)", borderColor: "var(--border)" },
    quiet:   { background: "transparent", color: "var(--muted)", borderColor: "transparent" },
  };
  return (
    <button type={type} disabled={disabled} onClick={onClick}
      style={{ ...base, ...variants[variant], ...style }}>
      {icon ? <span style={{ display: "inline-flex" }}>{icon}</span> : null}
      {children}
    </button>
  );
}

function Tag({ children, tone = "default", style }) {
  const tones = {
    default: { bg: "var(--surface-2)", fg: "var(--text-2)", border: "var(--border)" },
    accent:  { bg: "var(--accent-dim)", fg: "var(--accent)", border: "var(--accent-line)" },
    amber:   { bg: "var(--amber-dim)", fg: "var(--amber)", border: "var(--amber)" },
    red:     { bg: "var(--red-dim)", fg: "var(--red)", border: "var(--red)" },
    blue:    { bg: "var(--blue-dim)", fg: "var(--blue)", border: "var(--blue)" },
    ghost:   { bg: "transparent", fg: "var(--muted)", border: "var(--border)" },
  };
  const t = tones[tone] || tones.default;
  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: 4,
      padding: "2px 7px",
      fontSize: 11, fontWeight: 500,
      letterSpacing: 0.3,
      textTransform: "uppercase",
      borderRadius: 4,
      background: t.bg, color: t.fg,
      border: `1px solid ${t.border === "var(--border)" ? "var(--border)" : t.border}`,
      ...style,
    }}>{children}</span>
  );
}

function Card({ children, style, padding = 20, hoverable }) {
  const [hover, setHover] = React.useState(false);
  return (
    <div
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        ...atomStyles.panel,
        padding,
        transition: "border-color 140ms, transform 140ms",
        borderColor: hoverable && hover ? "var(--border-strong)" : "var(--border)",
        ...style,
      }}>
      {children}
    </div>
  );
}

function SectionLabel({ children, kicker, action, style }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 14, ...style }}>
      <div>
        {kicker ? (
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.4, textTransform: "uppercase", color: "var(--muted)", marginBottom: 4 }}>
            {kicker}
          </div>
        ) : null}
        <h2 style={{ margin: 0, fontFamily: "var(--serif)", fontWeight: 500, fontSize: 22, letterSpacing: -0.2, color: "var(--text)" }}>
          {children}
        </h2>
      </div>
      {action ? <div>{action}</div> : null}
    </div>
  );
}

function KV({ k, v, mono }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", padding: "8px 0", borderBottom: "1px dashed var(--border)" }}>
      <span style={{ color: "var(--muted)", fontSize: 12 }}>{k}</span>
      <span style={{ color: "var(--text)", fontSize: 13, fontFamily: mono ? "var(--mono)" : "inherit", fontWeight: 500 }}>{v}</span>
    </div>
  );
}

function Stat({ label, value, sub, tone }) {
  const toneFg = tone === "up" ? "var(--accent)" : tone === "down" ? "var(--red)" : "var(--muted)";
  return (
    <div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{label}</div>
      <div style={{ fontFamily: "var(--serif)", fontSize: 30, fontWeight: 500, lineHeight: 1, color: "var(--text)", letterSpacing: -0.5 }}>{value}</div>
      {sub ? (
        <div style={{ marginTop: 6, fontSize: 12, color: toneFg }}>
          {sub}
        </div>
      ) : null}
    </div>
  );
}

function Divider({ vertical, style }) {
  if (vertical) return <div style={{ width: 1, background: "var(--border)", alignSelf: "stretch", ...style }} />;
  return <div style={{ height: 1, background: "var(--border)", width: "100%", ...style }} />;
}

// Tiny SVG icons (line style)
function Icon({ name, size = 16, stroke = 1.6, color = "currentColor" }) {
  const p = { width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: color, strokeWidth: stroke, strokeLinecap: "round", strokeLinejoin: "round" };
  const I = {
    library:   (<svg {...p}><path d="M4 4v16M9 4v16M14 6l2 14M19 5l2 14"/></svg>),
    take:      (<svg {...p}><path d="M4 6h16M4 12h10M4 18h7"/><circle cx="18" cy="16" r="3"/></svg>),
    results:   (<svg {...p}><path d="M4 19V5M4 19h16M8 15v-4M12 15V7M16 15v-2"/></svg>),
    dashboard: (<svg {...p}><rect x="3" y="3" width="7" height="9"/><rect x="14" y="3" width="7" height="5"/><rect x="14" y="12" width="7" height="9"/><rect x="3" y="16" width="7" height="5"/></svg>),
    author:    (<svg {...p}><path d="M4 20h4l10-10-4-4L4 16v4z"/><path d="M14 6l4 4"/></svg>),
    agent:     (<svg {...p}><rect x="3" y="6" width="18" height="14" rx="2"/><path d="M8 6V3M16 6V3M3 12h18"/><circle cx="9" cy="16" r="1"/><circle cx="15" cy="16" r="1"/></svg>),
    settings:  (<svg {...p}><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4.9a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.5a7 7 0 0 0-2 1.2l-2.4-.9-2 3.4 2 1.6A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.6 2 3.4 2.4-.9c.6.5 1.3.9 2 1.2L10 21h4l.5-2.5c.7-.3 1.4-.7 2-1.2l2.4.9 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z"/></svg>),
    arrow:     (<svg {...p}><path d="M5 12h14M13 6l6 6-6 6"/></svg>),
    arrowL:    (<svg {...p}><path d="M19 12H5M11 6l-6 6 6 6"/></svg>),
    check:     (<svg {...p}><path d="M5 12l4 4 10-10"/></svg>),
    x:         (<svg {...p}><path d="M6 6l12 12M18 6L6 18"/></svg>),
    plus:      (<svg {...p}><path d="M12 5v14M5 12h14"/></svg>),
    clock:     (<svg {...p}><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>),
    search:    (<svg {...p}><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></svg>),
    filter:    (<svg {...p}><path d="M3 5h18M6 12h12M10 19h4"/></svg>),
    bell:      (<svg {...p}><path d="M6 17V11a6 6 0 1 1 12 0v6l2 2H4l2-2zM10 21a2 2 0 0 0 4 0"/></svg>),
    key:       (<svg {...p}><circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3"/></svg>),
    copy:      (<svg {...p}><rect x="9" y="9" width="11" height="11" rx="1"/><path d="M5 15V5a1 1 0 0 1 1-1h10"/></svg>),
    download:  (<svg {...p}><path d="M12 4v12M6 12l6 6 6-6M4 20h16"/></svg>),
    upload:    (<svg {...p}><path d="M12 20V8M6 12l6-6 6 6M4 4h16"/></svg>),
    flag:      (<svg {...p}><path d="M5 21V4l14 4-14 4"/></svg>),
    user:      (<svg {...p}><circle cx="12" cy="8" r="4"/><path d="M4 21c1.5-4 4.5-6 8-6s6.5 2 8 6"/></svg>),
    sparkle:   (<svg {...p}><path d="M12 4v4M12 16v4M4 12h4M16 12h4M6 6l3 3M15 15l3 3M18 6l-3 3M9 15l-3 3"/></svg>),
    book:      (<svg {...p}><path d="M4 5a2 2 0 0 1 2-2h12v18H6a2 2 0 0 1-2-2V5z"/><path d="M4 19a2 2 0 0 1 2-2h12"/></svg>),
    code:      (<svg {...p}><path d="M9 18l-6-6 6-6M15 6l6 6-6 6"/></svg>),
    exam:      (<svg {...p}><rect x="5" y="3" width="14" height="18" rx="1"/><path d="M9 8h6M9 12h6M9 16h4"/></svg>),
    stack:     (<svg {...p}><path d="M12 3l9 5-9 5-9-5 9-5z"/><path d="M3 13l9 5 9-5M3 18l9 5 9-5"/></svg>),
  };
  return I[name] || null;
}

// Logo
function Logo({ size = 22 }) {
  return (
    <div style={{ display: "inline-flex", alignItems: "center", gap: 9 }}>
      <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
        <rect x="2" y="2" width="28" height="28" rx="6" fill="var(--accent)" />
        <path d="M10 9v14M22 9v14M10 16h12" stroke="#0b1410" strokeWidth="2.5" strokeLinecap="round" />
      </svg>
      <span style={{ fontFamily: "var(--serif)", fontSize: 19, fontWeight: 600, letterSpacing: -0.3, color: "var(--text)" }}>
        Harus
      </span>
    </div>
  );
}

// Decorative striped placeholder
function Placeholder({ label, height = 140, style }) {
  return (
    <div style={{
      height, width: "100%",
      backgroundImage:
        "repeating-linear-gradient(135deg, var(--surface-2) 0 8px, var(--surface) 8px 16px)",
      border: "1px dashed var(--border-strong)",
      borderRadius: 6,
      display: "flex", alignItems: "center", justifyContent: "center",
      ...style,
    }}>
      <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.6 }}>{label}</span>
    </div>
  );
}

Object.assign(window, { Button, Tag, Card, SectionLabel, KV, Stat, Divider, Icon, Logo, Placeholder });
