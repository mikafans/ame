/* Learning objectives + Share modal — adds knowledge-transfer affordances
   inspired by Udemy ("what you'll learn") and modern share sheets. */

function LearningObjectives({ items, kicker = "What you'll learn", compact, accentBars = true }) {
  if (!items || !items.length) return null;
  if (compact) {
    return (
      <div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
          {kicker}
        </div>
        <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 8 }}>
          {items.map((it, i) => (
            <li key={i} style={{ display: "flex", gap: 10, alignItems: "flex-start", fontSize: 12.5, color: "var(--text-2)", lineHeight: 1.5 }}>
              <Icon name="check" size={13} color="var(--accent)" />
              <span style={{ flex: 1 }}>{it}</span>
            </li>
          ))}
        </ul>
      </div>
    );
  }
  return (
    <div style={{ padding: "18px 20px", background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
        <Icon name="sparkle" size={14} color="var(--accent)" />
        <span style={{ fontFamily: "var(--mono)", fontSize: 10.5, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)" }}>
          {kicker}
        </span>
      </div>
      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
        {items.map((it, i) => (
          <li key={i} style={{
            display: "flex", gap: 10, alignItems: "flex-start",
            paddingLeft: accentBars ? 10 : 0,
            borderLeft: accentBars ? "2px solid var(--accent)" : "none",
            fontSize: 13, color: "var(--text)", lineHeight: 1.5,
          }}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.5, paddingTop: 2 }}>{String(i + 1).padStart(2, "0")}</span>
            <span style={{ flex: 1, color: "var(--text-2)" }}>{it}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/* ---------------- Share modal ---------------- */
// Global imperative API: openShare({ kind, title, course, body, explanation, attribution })
const ShareContext = React.createContext({ open: () => {} });

function ShareProvider({ children }) {
  const [payload, setPayload] = React.useState(null);
  const open = React.useCallback((p) => setPayload(p), []);
  const close = () => setPayload(null);
  return (
    <ShareContext.Provider value={{ open }}>
      {children}
      {payload ? <ShareModal payload={payload} onClose={close} /> : null}
    </ShareContext.Provider>
  );
}

function useShare() {
  return React.useContext(ShareContext);
}

function ShareModal({ payload, onClose }) {
  const [tab, setTab] = React.useState("link"); // link | socials | embed | card
  const [includeExplain, setIncludeExplain] = React.useState(true);
  const [includeAttrib, setIncludeAttrib] = React.useState(true);
  const [copied, setCopied] = React.useState(null);

  const slug = (payload.kind === "item" ? "q" : payload.kind === "exam" ? "x" : "qz") + "/" + (payload.id || "demo");
  const shareUrl = `https://harus.app/${slug}`;
  const utm = "?utm_source=harus&utm_medium=share";

  const explanation = includeExplain ? (payload.explanation || "") : "";
  const attrib = includeAttrib ? `— ${payload.course || "Harus"}${payload.attribution ? " · " + payload.attribution : ""}` : "";

  const composed = [payload.body, explanation, attrib].filter(Boolean).join("\n\n");

  const copy = (text, key) => {
    if (navigator.clipboard) navigator.clipboard.writeText(text).catch(() => {});
    setCopied(key);
    setTimeout(() => setCopied(null), 1400);
  };

  const tabs = [
    { id: "link",    label: "Link",     icon: "key" },
    { id: "socials", label: "Socials",  icon: "user" },
    { id: "card",    label: "Card",     icon: "results" },
    { id: "embed",   label: "Embed",    icon: "code" },
  ];

  return (
    <div onClick={onClose} style={{
      position: "fixed", inset: 0, zIndex: 1000,
      background: "rgba(8, 12, 18, 0.66)",
      backdropFilter: "blur(6px)",
      display: "flex", alignItems: "center", justifyContent: "center",
      padding: 32,
    }}>
      <div onClick={(e) => e.stopPropagation()} style={{
        width: "min(820px, 100%)", maxHeight: "92vh", overflowY: "auto",
        background: "var(--surface)", border: "1px solid var(--border-strong)",
        borderRadius: 10, boxShadow: "0 24px 80px rgba(0,0,0,0.5)",
        display: "grid", gridTemplateColumns: "1fr 280px",
      }}>
        {/* Left: content */}
        <div style={{ borderRight: "1px solid var(--border)" }}>
          {/* Header */}
          <div style={{ padding: "20px 24px 0", display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 16 }}>
            <div>
              <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)" }}>
                Share · {payload.kind === "item" ? "single question" : payload.kind === "exam" ? "exam" : "assessment"}
              </div>
              <h2 style={{ margin: "4px 0 0", fontFamily: "var(--serif)", fontSize: 22, fontWeight: 500, letterSpacing: -0.3 }}>
                {payload.title}
              </h2>
              {payload.course ? (
                <div style={{ marginTop: 2, fontSize: 12, color: "var(--muted)" }}>{payload.course}</div>
              ) : null}
            </div>
            <button onClick={onClose} style={{
              background: "transparent", border: "none", cursor: "pointer",
              color: "var(--muted)", padding: 4,
            }}><Icon name="x" size={18} /></button>
          </div>

          {/* Tabs */}
          <div style={{ display: "flex", gap: 0, padding: "16px 24px 0", borderBottom: "1px solid var(--border)" }}>
            {tabs.map((t) => {
              const active = tab === t.id;
              return (
                <button key={t.id} onClick={() => setTab(t.id)} style={{
                  background: "transparent", border: "none", cursor: "pointer",
                  padding: "10px 12px", marginRight: 4,
                  display: "inline-flex", gap: 6, alignItems: "center",
                  fontSize: 12.5, fontWeight: active ? 600 : 500,
                  color: active ? "var(--text)" : "var(--muted)",
                  borderBottom: `2px solid ${active ? "var(--accent)" : "transparent"}`,
                  marginBottom: -1,
                }}>
                  <Icon name={t.icon} size={13} /> {t.label}
                </button>
              );
            })}
          </div>

          {/* Body */}
          <div style={{ padding: 24 }}>
            {tab === "link" && (
              <LinkTab url={shareUrl + utm} copy={copy} copied={copied} payload={payload} />
            )}
            {tab === "socials" && (
              <SocialsTab url={shareUrl + utm} composed={composed} copy={copy} copied={copied} />
            )}
            {tab === "card" && (
              <CardTab payload={payload} explanation={explanation} attrib={attrib} copy={copy} copied={copied} />
            )}
            {tab === "embed" && (
              <EmbedTab url={shareUrl} copy={copy} copied={copied} />
            )}
          </div>
        </div>

        {/* Right rail: options */}
        <aside style={{ padding: "24px 22px", background: "var(--surface-2)" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 12 }}>
            What travels with it
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
            <ShareToggleRow label="Question prompt" value disabled />
            {payload.kind === "item" ? (
              <ShareToggleRow label="Explanation" value={includeExplain} onChange={setIncludeExplain} />
            ) : null}
            <ShareToggleRow label="Attribution" value={includeAttrib} onChange={setIncludeAttrib} sub="Course + author" />
            <ShareToggleRow label="Your score"  value={false} onChange={() => {}} sub="Off by default" />
          </div>

          <Divider style={{ margin: "18px 0" }} />

          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
            Privacy
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 14 }}>
            {[
              { id: "public", label: "Anyone with the link" },
              { id: "cohort", label: "Cohort only" },
            ].map((s) => (
              <button key={s.id} style={{
                padding: "8px 10px",
                background: s.id === "public" ? "var(--accent-dim)" : "var(--surface)",
                border: `1px solid ${s.id === "public" ? "var(--accent-line)" : "var(--border)"}`,
                color: s.id === "public" ? "var(--accent)" : "var(--text-2)",
                borderRadius: 4, fontSize: 11.5, fontWeight: 500, cursor: "pointer",
              }}>{s.label}</button>
            ))}
          </div>
          <div style={{ fontSize: 11.5, color: "var(--muted)", lineHeight: 1.55 }}>
            Shared links never reveal other learners' attempts. Your own score is opt-in only.
          </div>

          <Divider style={{ margin: "18px 0" }} />

          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
            Agent-equivalent
          </div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-2)", lineHeight: 1.6, background: "var(--bg)", border: "1px solid var(--border)", borderRadius: 4, padding: 10 }}>
            POST /v1/shares<br />
            {`{ "kind": "${payload.kind || "assessment"}",`}<br />
            &nbsp;&nbsp;{`"id": "${payload.id || "demo"}",`}<br />
            &nbsp;&nbsp;{`"visibility": "public" }`}
          </div>
        </aside>
      </div>
    </div>
  );
}

function LinkTab({ url, copy, copied, payload }) {
  return (
    <div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
        Shareable URL
      </div>
      <div style={{
        display: "flex", gap: 8, alignItems: "stretch",
        background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6,
        padding: 4,
      }}>
        <input readOnly value={url} style={{
          flex: 1, background: "transparent", border: "none",
          padding: "10px 12px",
          fontFamily: "var(--mono)", fontSize: 12.5, color: "var(--text)",
          outline: "none", letterSpacing: 0.2,
        }} />
        <button onClick={() => copy(url, "link")} style={{
          background: copied === "link" ? "var(--accent)" : "var(--bg)",
          color: copied === "link" ? "#0b1410" : "var(--text)",
          border: `1px solid ${copied === "link" ? "var(--accent)" : "var(--border)"}`,
          borderRadius: 4, padding: "0 16px",
          fontSize: 12.5, fontWeight: 600,
          cursor: "pointer",
          display: "inline-flex", alignItems: "center", gap: 6,
        }}>
          <Icon name={copied === "link" ? "check" : "copy"} size={13} />
          {copied === "link" ? "Copied" : "Copy link"}
        </button>
      </div>

      <div style={{ marginTop: 24 }}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
          Quick actions
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
          {[
            { l: "Copy as Markdown",  icon: "copy" },
            { l: "Download PDF",      icon: "download" },
            { l: "Print",             icon: "results" },
          ].map((a) => (
            <button key={a.l} style={{
              padding: "12px 10px", textAlign: "left",
              background: "var(--surface)", border: "1px solid var(--border)",
              borderRadius: 6, color: "var(--text-2)",
              cursor: "pointer",
              display: "flex", alignItems: "center", gap: 8,
              fontSize: 12.5,
            }}>
              <Icon name={a.icon} size={14} color="var(--accent)" />
              {a.l}
            </button>
          ))}
        </div>
      </div>

      <div style={{ marginTop: 24, padding: 12, background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, display: "flex", gap: 12, alignItems: "center" }}>
        <div style={{
          width: 64, height: 64,
          background: "var(--bg)",
          border: "1px solid var(--border)",
          borderRadius: 4,
          backgroundImage: "repeating-linear-gradient(45deg, var(--text) 0 2px, transparent 2px 4px), repeating-linear-gradient(-45deg, var(--text) 0 2px, transparent 2px 4px)",
          backgroundSize: "8px 8px",
          flexShrink: 0,
        }} />
        <div style={{ flex: 1 }}>
          <div style={{ fontFamily: "var(--serif)", fontSize: 14, fontWeight: 500 }}>QR for the room</div>
          <div style={{ fontSize: 11.5, color: "var(--muted)", marginTop: 2 }}>
            Show the QR on a projector so the cohort can scan and open the shared item.
          </div>
        </div>
        <Button variant="ghost" size="sm">Generate</Button>
      </div>
    </div>
  );
}

function SocialsTab({ url, composed, copy, copied }) {
  const text = encodeURIComponent(composed);
  const u = encodeURIComponent(url);
  const socials = [
    { id: "x",        label: "X",         color: "#ffffff", bg: "#000", url: `https://twitter.com/intent/tweet?url=${u}&text=${text}` },
    { id: "linkedin", label: "LinkedIn",  color: "#ffffff", bg: "#0a66c2", url: `https://www.linkedin.com/sharing/share-offsite/?url=${u}` },
    { id: "reddit",   label: "Reddit",    color: "#ffffff", bg: "#ff4500", url: `https://reddit.com/submit?url=${u}&title=${text}` },
    { id: "hn",       label: "Hacker News", color: "#ffffff", bg: "#ff6600", url: `https://news.ycombinator.com/submitlink?u=${u}` },
    { id: "email",    label: "Email",     color: "var(--text)", bg: "var(--surface-2)", url: `mailto:?body=${text}%0A%0A${u}` },
    { id: "mastodon", label: "Mastodon",  color: "#ffffff", bg: "#6364ff", url: "https://mastodon.social/share" },
  ];
  return (
    <div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
        Send the question + explanation to
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
        {socials.map((s) => (
          <a key={s.id} href={s.url} target="_blank" rel="noopener noreferrer" style={{
            display: "flex", alignItems: "center", gap: 10,
            padding: "12px 14px",
            background: s.bg, color: s.color,
            border: `1px solid ${s.bg === "var(--surface-2)" ? "var(--border)" : s.bg}`,
            borderRadius: 6, textDecoration: "none",
            fontSize: 13, fontWeight: 500,
          }}>
            <div style={{ width: 18, height: 18, borderRadius: 4, background: "rgba(255,255,255,0.18)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--mono)", fontSize: 10, color: s.color }}>
              {s.label[0]}
            </div>
            {s.label}
          </a>
        ))}
      </div>

      <div style={{ marginTop: 22, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
        Preview · what they'll see
      </div>
      <div style={{ background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, padding: 14, fontFamily: "var(--serif)", fontSize: 14, lineHeight: 1.6, color: "var(--text)", whiteSpace: "pre-wrap" }}>
        {composed || "Question prompt will appear here…"}
        <div style={{ marginTop: 10, fontFamily: "var(--mono)", fontSize: 11, color: "var(--accent)" }}>
          {url}
        </div>
      </div>

      <div style={{ marginTop: 12 }}>
        <Button variant="ghost" size="sm" icon={<Icon name={copied === "compose" ? "check" : "copy"} size={12} />} onClick={() => copy(composed + "\n\n" + url, "compose")}>
          {copied === "compose" ? "Copied" : "Copy composed message"}
        </Button>
      </div>
    </div>
  );
}

function CardTab({ payload, explanation, attrib, copy, copied }) {
  return (
    <div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
        Social card · 1200 × 630 preview
      </div>

      {/* Card preview */}
      <div style={{
        aspectRatio: "1200 / 630",
        background: "linear-gradient(160deg, var(--surface) 0%, var(--bg) 100%)",
        border: "1px solid var(--border-strong)",
        borderRadius: 8,
        padding: "28px 32px",
        display: "flex", flexDirection: "column", justifyContent: "space-between",
        position: "relative", overflow: "hidden",
      }}>
        <div style={{
          position: "absolute", top: -40, right: -40,
          width: 220, height: 220, borderRadius: "50%",
          background: "var(--accent-dim)", filter: "blur(40px)",
          opacity: 0.6,
        }} />

        {/* top */}
        <div style={{ position: "relative", zIndex: 1, display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
          <Logo size={22} />
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, color: "var(--muted)", textTransform: "uppercase" }}>
            {payload.kind === "item" ? "From Algorithms — Graph Traversal" : payload.course || "Harus"}
          </div>
        </div>

        {/* middle */}
        <div style={{ position: "relative", zIndex: 1, maxWidth: "85%" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--accent)", letterSpacing: 1, marginBottom: 8 }}>
            {payload.kind === "item" ? "Q · explained" : "Try this " + (payload.kind || "assessment")}
          </div>
          <div style={{ fontFamily: "var(--serif)", fontSize: 22, fontWeight: 500, lineHeight: 1.3, letterSpacing: -0.2, color: "var(--text)" }}>
            {payload.body || payload.title}
          </div>
          {explanation ? (
            <div style={{ marginTop: 10, fontSize: 12, color: "var(--text-2)", lineHeight: 1.5, maxWidth: "92%" }}>
              {explanation}
            </div>
          ) : null}
        </div>

        {/* bottom */}
        <div style={{ position: "relative", zIndex: 1, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.5 }}>
            harus.app/{payload.kind === "item" ? "q" : payload.kind === "exam" ? "x" : "qz"}/{payload.id || "demo"}
          </div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-2)", letterSpacing: 0.5 }}>
            {attrib}
          </div>
        </div>
      </div>

      <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
        <Button variant="primary" icon={<Icon name="download" size={13} color="#0b1410" />}>Download PNG</Button>
        <Button variant="solid" icon={<Icon name="copy" size={13} />}>Copy image</Button>
        <Button variant="ghost">Edit style</Button>
        <div style={{ marginLeft: "auto", fontSize: 11.5, color: "var(--muted)", alignSelf: "center" }}>
          Open Graph + Twitter Card tags inject automatically
        </div>
      </div>
    </div>
  );
}

function EmbedTab({ url, copy, copied }) {
  const code = `<iframe src="${url}/embed"\n        width="640" height="420"\n        frameborder="0"\n        allow="clipboard-write"\n        title="Harus question"\n        loading="lazy"></iframe>`;
  return (
    <div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
        Embed code
      </div>
      <div style={{ background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, overflow: "hidden" }}>
        <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border)", background: "var(--surface)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 1 }}>embed.html</span>
          <button onClick={() => copy(code, "embed")} style={{
            background: "transparent", border: "none", cursor: "pointer",
            color: copied === "embed" ? "var(--accent)" : "var(--muted)",
            display: "inline-flex", gap: 4, alignItems: "center",
            fontFamily: "var(--mono)", fontSize: 11,
          }}>
            <Icon name={copied === "embed" ? "check" : "copy"} size={12} /> {copied === "embed" ? "Copied" : "Copy"}
          </button>
        </div>
        <pre style={{
          margin: 0, padding: 14,
          fontFamily: "var(--mono)", fontSize: 12, lineHeight: 1.65,
          color: "var(--text-2)", whiteSpace: "pre",
          overflow: "auto",
        }}>{code}</pre>
      </div>

      <div style={{ marginTop: 18, display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
        {[
          { l: "Notion",  hint: "Drop URL"      },
          { l: "Obsidian", hint: "Web embed"    },
          { l: "WordPress", hint: "oEmbed"      },
          { l: "MDX / Docs", hint: "<Assessment id>"  },
          { l: "Slack",    hint: "Unfurled"     },
          { l: "Discord",  hint: "Unfurled"     },
        ].map((p) => (
          <div key={p.l} style={{ padding: 12, background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6 }}>
            <div style={{ fontSize: 13, fontWeight: 500 }}>{p.l}</div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 0.5, marginTop: 4 }}>{p.hint}</div>
          </div>
        ))}
      </div>

      <div style={{ marginTop: 16, fontSize: 11.5, color: "var(--muted)", lineHeight: 1.55 }}>
        Embedded items are read-only by default. Add <code style={{ fontFamily: "var(--mono)", background: "var(--surface-2)", padding: "1px 4px", borderRadius: 3 }}>?interactive=1</code> to let viewers attempt the question; results are not saved to your cohort.
      </div>
    </div>
  );
}

function ShareToggleRow({ label, sub, value, onChange, disabled }) {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 0", borderBottom: "1px dashed var(--border)", opacity: disabled ? 0.55 : 1 }}>
      <div>
        <div style={{ fontSize: 12.5, color: "var(--text)" }}>{label}</div>
        {sub ? <div style={{ fontSize: 11, color: "var(--muted)", marginTop: 2 }}>{sub}</div> : null}
      </div>
      <button onClick={disabled ? undefined : () => onChange(!value)} disabled={disabled} style={{
        width: 32, height: 18, padding: 0, position: "relative",
        background: value ? "var(--accent)" : "var(--bg)",
        border: `1px solid ${value ? "var(--accent)" : "var(--border)"}`,
        borderRadius: 999, cursor: disabled ? "not-allowed" : "pointer",
      }}>
        <span style={{
          position: "absolute", top: 1, left: value ? 15 : 1,
          width: 14, height: 14, borderRadius: "50%",
          background: value ? "#0b1410" : "var(--text-2)",
          transition: "left 140ms",
        }} />
      </button>
    </div>
  );
}

// Small inline share button — used inside cards and review rows
function ShareButton({ payload, size = "sm", variant = "ghost", children }) {
  const { open } = useShare();
  return (
    <Button variant={variant} size={size} onClick={() => open(payload)} icon={<Icon name="upload" size={size === "sm" ? 12 : 14} />}>
      {children || "Share"}
    </Button>
  );
}

Object.assign(window, { LearningObjectives, ShareProvider, ShareModal, ShareButton, useShare });
