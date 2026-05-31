function SignupScreen({ onEnter }) {
  const [tab, setTab] = React.useState("signup");
  const [email, setEmail] = React.useState("jordan.tahir@stanford.edu");
  const [name, setName] = React.useState("Jordan Tahir");
  const [role, setRole] = React.useState("learner");

  return (
    <div style={{
      minHeight: "100vh",
      display: "grid", gridTemplateColumns: "1.05fr 1fr",
      background: "var(--bg)",
    }}>
      {/* LEFT: marketing */}
      <div style={{
        padding: "56px 64px",
        borderRight: "1px solid var(--border)",
        background: "linear-gradient(180deg, var(--surface) 0%, var(--bg) 70%)",
        display: "flex", flexDirection: "column", justifyContent: "space-between",
      }}>
        <Logo size={28} />

        <div style={{ maxWidth: 520 }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 1.6, textTransform: "uppercase", color: "var(--accent)", marginBottom: 18 }}>
            Assessment platform · est. 2025
          </div>
          <h1 style={{
            fontFamily: "var(--serif)", fontSize: 56, lineHeight: 1.04,
            margin: 0, fontWeight: 500, letterSpacing: -1.2,
          }}>
            Assessments that learners and agents can both read.
          </h1>
          <p style={{ color: "var(--text-2)", fontSize: 16, lineHeight: 1.55, marginTop: 22, maxWidth: 460 }}>
            Harus is an assessment platform built for two audiences at once. Students get a focused
            test-taking experience and a real progress dashboard. Authors and AI agents share the same
            structured surface — every assessment, attempt, and rubric is addressable, importable, and
            queryable through a single API.
          </p>

          <div style={{ marginTop: 36, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, maxWidth: 460 }}>
            {[
              ["18,402", "active learners"],
              ["1,243", "instructors"],
              ["94", "institutions"],
              ["6.1M", "graded attempts"],
            ].map(([v, l]) => (
              <div key={l} style={{ padding: "12px 14px", border: "1px solid var(--border)", borderRadius: 6 }}>
                <div style={{ fontFamily: "var(--serif)", fontSize: 22, fontWeight: 500 }}>{v}</div>
                <div style={{ fontSize: 11, color: "var(--muted)", fontFamily: "var(--mono)", letterSpacing: 1, textTransform: "uppercase", marginTop: 2 }}>{l}</div>
              </div>
            ))}
          </div>
        </div>

        <div style={{ display: "flex", gap: 18, alignItems: "center", color: "var(--muted)", fontSize: 12, fontFamily: "var(--mono)", letterSpacing: 0.6 }}>
          <span>SSO · SAML</span>
          <span>·</span>
          <span>FERPA · GDPR</span>
          <span>·</span>
          <span>OpenAPI 3.1 · MCP</span>
        </div>
      </div>

      {/* RIGHT: form */}
      <div style={{ padding: "56px 64px", display: "flex", alignItems: "center" }}>
        <div style={{ width: "100%", maxWidth: 420 }}>
          <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border)", marginBottom: 28 }}>
            {["signup", "login"].map((t) => (
              <button key={t} onClick={() => setTab(t)} style={{
                background: "transparent", border: "none",
                padding: "10px 0", marginRight: 24,
                color: tab === t ? "var(--text)" : "var(--muted)",
                borderBottom: `2px solid ${tab === t ? "var(--accent)" : "transparent"}`,
                fontWeight: tab === t ? 600 : 500,
                fontSize: 13, letterSpacing: 0.3,
                cursor: "pointer",
              }}>{t === "signup" ? "Create account" : "Sign in"}</button>
            ))}
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            {tab === "signup" && (
              <Field label="Full name">
                <input value={name} onChange={(e) => setName(e.target.value)} style={inputStyle} />
              </Field>
            )}
            <Field label="Institutional email">
              <input value={email} onChange={(e) => setEmail(e.target.value)} style={inputStyle} />
              <div style={{ fontSize: 11, color: "var(--muted)", marginTop: 6, fontFamily: "var(--mono)" }}>
                Recognized: stanford.edu · SSO available
              </div>
            </Field>
            <Field label="Password">
              <input type="password" defaultValue="••••••••••" style={inputStyle} />
            </Field>

            {tab === "signup" && (
              <Field label="Role">
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
                  {["learner", "instructor", "agent"].map((r) => (
                    <button key={r} onClick={() => setRole(r)} style={{
                      padding: "10px 8px",
                      background: role === r ? "var(--accent-dim)" : "var(--surface)",
                      border: `1px solid ${role === r ? "var(--accent-line)" : "var(--border)"}`,
                      color: role === r ? "var(--accent)" : "var(--text-2)",
                      borderRadius: 6, fontSize: 12, fontWeight: 500,
                      textTransform: "capitalize", cursor: "pointer",
                    }}>{r}</button>
                  ))}
                </div>
                <div style={{ fontSize: 11, color: "var(--muted)", marginTop: 8, lineHeight: 1.5 }}>
                  {role === "learner"     && "Take assigned assessments and track your progress."}
                  {role === "instructor"  && "Author assessments, manage cohorts, and review attempts."}
                  {role === "agent"       && "Get an API key, an OpenAPI schema, and MCP tool descriptors."}
                </div>
              </Field>
            )}

            <div style={{ marginTop: 12, display: "flex", gap: 10 }}>
              <Button variant="primary" size="lg" onClick={onEnter} style={{ flex: 1, justifyContent: "center" }} icon={<Icon name="arrow" size={14} color="#0b1410" />}>
                {tab === "signup" ? "Create account" : "Sign in"}
              </Button>
            </div>

            <div style={{ display: "flex", alignItems: "center", gap: 12, color: "var(--muted)", fontSize: 11, margin: "8px 0" }}>
              <div style={{ flex: 1, height: 1, background: "var(--border)" }} />
              <span style={{ fontFamily: "var(--mono)", letterSpacing: 1.2 }}>OR</span>
              <div style={{ flex: 1, height: 1, background: "var(--border)" }} />
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <Button variant="ghost" onClick={onEnter}>Continue with SSO</Button>
              <Button variant="ghost" onClick={onEnter}>Use access code</Button>
            </div>
          </div>

          <div style={{ marginTop: 36, padding: 14, border: "1px dashed var(--border-strong)", borderRadius: 6, background: "var(--surface)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, color: "var(--accent)", fontSize: 12, fontWeight: 600, marginBottom: 4 }}>
              <Icon name="sparkle" size={14} /> Agent shortcut
            </div>
            <div style={{ fontSize: 12, color: "var(--text-2)", lineHeight: 1.5 }}>
              Programmatic access? <span style={{ fontFamily: "var(--mono)", color: "var(--text)" }}>POST /v1/agents/register</span> returns a key,
              an OpenAPI schema, and an MCP manifest in one call.
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

const inputStyle = {
  width: "100%",
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 6,
  padding: "10px 12px",
  fontSize: 14,
  color: "var(--text)",
  outline: "none",
  fontFamily: "var(--sans)",
};

function Field({ label, children }) {
  return (
    <label style={{ display: "block" }}>
      <div style={{ fontSize: 11, fontFamily: "var(--mono)", letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{label}</div>
      {children}
    </label>
  );
}

Object.assign(window, { SignupScreen });
