function AgentScreen() {
  const [section, setSection] = React.useState("overview");
  const [importText, setImportText] = React.useState(SAMPLE_IMPORT);
  const [importResponse, setImportResponse] = React.useState(null);
  const [activeTool, setActiveTool] = React.useState(AGENT_TOOLS[0].name);

  const runImport = () => {
    let warnings = [];
    let questionCount = 0;
    try {
      const parsed = JSON.parse(importText);
      questionCount = (parsed.questions || []).length;
      if (questionCount === 0) warnings.push("No questions found in payload.");
      if (!parsed.course) warnings.push("Missing field 'course' — defaulted to 'UNCATEGORIZED'.");
    } catch (e) {
      warnings.push("JSON parse error: " + e.message);
    }
    setImportResponse({
      ts: new Date().toISOString().slice(11, 19) + "Z",
      ok: warnings.length === 0,
      body: {
        assessmentId: "qz_" + Math.random().toString(36).slice(2, 6),
        questionCount,
        warnings,
        url: "https://harus.app/q/qz_imported",
      },
    });
  };

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      <SectionLabel
        kicker="Programmatic surface · OpenAPI 3.1 · MCP-compatible"
        action={
          <div style={{ display: "flex", gap: 8 }}>
            <Button variant="ghost" icon={<Icon name="download" size={14} />}>OpenAPI</Button>
            <Button variant="ghost" icon={<Icon name="download" size={14} />}>MCP manifest</Button>
            <Button variant="solid" icon={<Icon name="key" size={14} />}>New API key</Button>
          </div>
        }>
        Agent integration
      </SectionLabel>

      {/* Top: explainer band */}
      <Card padding={0} style={{ marginBottom: 18, overflow: "hidden" }}>
        <div style={{ display: "grid", gridTemplateColumns: "1.2fr 1fr" }}>
          <div style={{ padding: "26px 30px" }}>
            <Tag tone="accent">Two interfaces, one model</Tag>
            <h3 style={{ margin: "12px 0 8px", fontFamily: "var(--serif)", fontSize: 24, fontWeight: 500, letterSpacing: -0.3 }}>
              Assessments, attempts, and rubrics are first-class API objects.
            </h3>
            <p style={{ color: "var(--text-2)", fontSize: 14, lineHeight: 1.55, margin: 0, maxWidth: 540 }}>
              Every screen a learner or instructor sees is backed by the same REST surface that agents
              use. A grading agent reads a learner's attempt with one call; an authoring agent imports
              a new assessment with another. No scraping, no duplicate state.
            </p>
            <div style={{ marginTop: 22, display: "flex", gap: 22 }}>
              <KV2 k="Endpoints" v="34" />
              <KV2 k="Auth" v="Bearer + scopes" />
              <KV2 k="Rate limit" v="120 / min" />
              <KV2 k="SDKs" v="ts · py · go" />
            </div>
          </div>

          <div style={{ padding: "22px 26px", background: "var(--surface-2)", borderLeft: "1px solid var(--border)" }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
              Hello, world
            </div>
            <CodeBlock label="curl"
              lines={[
                `curl https://api.harus.app/v1/assessments \\`,
                `  -H "Authorization: Bearer hk_live_3fY9…ax2P" \\`,
                `  -H "Content-Type: application/json"`,
                ``,
                `→ 200 OK · 24 assessments`,
              ]}
              dim={[4]}
            />
          </div>
        </div>
      </Card>

      {/* Section tabs */}
      <div style={{ display: "flex", gap: 4, marginBottom: 18, borderBottom: "1px solid var(--border)" }}>
        {[
          { id: "overview", label: "API keys" },
          { id: "tools",    label: "MCP tools" },
          { id: "import",   label: "Import demo" },
          { id: "log",      label: "Recent activity" },
        ].map((t) => {
          const active = section === t.id;
          return (
            <button key={t.id} onClick={() => setSection(t.id)} style={{
              background: "transparent", border: "none", cursor: "pointer",
              padding: "10px 14px",
              fontSize: 13, fontWeight: active ? 600 : 500,
              color: active ? "var(--text)" : "var(--muted)",
              borderBottom: `2px solid ${active ? "var(--accent)" : "transparent"}`,
              marginBottom: -1,
            }}>{t.label}</button>
          );
        })}
      </div>

      {section === "overview" && <KeysSection />}
      {section === "tools"    && <ToolsSection active={activeTool} setActive={setActiveTool} />}
      {section === "import"   && <ImportSection text={importText} setText={setImportText} response={importResponse} run={runImport} />}
      {section === "log"      && <LogSection />}
    </div>
  );
}

function KV2({ k, v }) {
  return (
    <div>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, color: "var(--muted)", textTransform: "uppercase", marginBottom: 2 }}>{k}</div>
      <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500 }}>{v}</div>
    </div>
  );
}

function KeysSection() {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1fr", gap: 18 }}>
      <Card padding={0}>
        <div style={{ padding: "14px 20px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ fontFamily: "var(--serif)", fontSize: 16, fontWeight: 500 }}>API keys</div>
          <Button variant="ghost" size="sm" icon={<Icon name="plus" size={12} />}>Create</Button>
        </div>
        <div>
          {API_KEYS.map((k, i) => (
            <div key={k.id} style={{
              padding: "16px 20px",
              borderBottom: i < API_KEYS.length - 1 ? "1px solid var(--border)" : "none",
            }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontSize: 13.5, fontWeight: 600, color: "var(--text)" }}>{k.label}</div>
                  <div style={{ marginTop: 6, fontFamily: "var(--mono)", fontSize: 12, color: "var(--text-2)", letterSpacing: 0.4 }}>
                    {k.prefix}
                  </div>
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <Button variant="ghost" size="sm" icon={<Icon name="copy" size={12} />}>Copy</Button>
                  <Button variant="quiet" size="sm">Rotate</Button>
                  <Button variant="danger" size="sm">Revoke</Button>
                </div>
              </div>
              <div style={{ marginTop: 10, display: "flex", gap: 16, alignItems: "center" }}>
                <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.5 }}>
                  Created {k.created} · Last used {k.lastUsed}
                </div>
                <div style={{ display: "flex", gap: 4, marginLeft: "auto" }}>
                  {k.scopes.map((s) => <Tag key={s} tone={s === "*" ? "amber" : "default"}>{s}</Tag>)}
                </div>
              </div>
            </div>
          ))}
        </div>
      </Card>

      <Card padding={22}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
          Authentication
        </div>
        <div style={{ fontFamily: "var(--serif)", fontSize: 16, fontWeight: 500, marginBottom: 8 }}>
          Header-based bearer token
        </div>
        <p style={{ color: "var(--text-2)", fontSize: 13, lineHeight: 1.6, margin: 0 }}>
          Send the key in an <code style={cd}>Authorization</code> header. Scopes are checked per
          endpoint; a key with <code style={cd}>attempt.read</code> cannot create assessments.
        </p>
        <CodeBlock label="request" style={{ marginTop: 14 }}
          lines={[
            `GET /v1/assessments/qz_8sd1/stats`,
            `Authorization: Bearer hk_live_3fY9…ax2P`,
            `Accept: application/json`,
            `X-Cohort: spring-2026`,
          ]}
        />
        <div style={{ marginTop: 18, paddingTop: 18, borderTop: "1px solid var(--border)" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
            Scope reference
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, fontSize: 12, color: "var(--text-2)" }}>
            {["assessment.read", "assessment.write", "attempt.read", "stats.read", "feedback.write", "plan.write"].map((s) => (
              <div key={s} style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <Icon name="check" size={11} color="var(--accent)" />
                <code style={cd}>{s}</code>
              </div>
            ))}
          </div>
        </div>
      </Card>
    </div>
  );
}

function ToolsSection({ active, setActive }) {
  const tool = AGENT_TOOLS.find((t) => t.name === active);
  return (
    <div style={{ display: "grid", gridTemplateColumns: "300px 1fr", gap: 18 }}>
      <Card padding={0}>
        <div style={{ padding: "14px 18px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)" }}>Tools</div>
            <div style={{ fontFamily: "var(--serif)", fontSize: 16, fontWeight: 500, marginTop: 2 }}>MCP descriptors</div>
          </div>
          <Tag tone="accent">{AGENT_TOOLS.length}</Tag>
        </div>
        {AGENT_TOOLS.map((t) => {
          const sel = active === t.name;
          return (
            <button key={t.name} onClick={() => setActive(t.name)} style={{
              width: "100%", padding: "12px 18px",
              background: sel ? "var(--accent-dim)" : "transparent",
              border: "none", borderLeft: `2px solid ${sel ? "var(--accent)" : "transparent"}`,
              borderBottom: "1px solid var(--border)",
              textAlign: "left", cursor: "pointer",
              display: "block",
            }}>
              <div style={{ fontFamily: "var(--mono)", fontSize: 13, fontWeight: 600, color: sel ? "var(--accent)" : "var(--text)" }}>
                {t.name}
              </div>
              <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 4, lineHeight: 1.4 }}>
                {t.desc}
              </div>
            </button>
          );
        })}
      </Card>

      <Card padding={0}>
        <div style={{ padding: "18px 22px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--accent)", letterSpacing: 0.5 }}>{tool.name}</div>
            <div style={{ fontFamily: "var(--serif)", fontSize: 20, fontWeight: 500, marginTop: 4 }}>{tool.desc}</div>
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <Tag tone={tool.method === "POST" ? "amber" : "default"}>{tool.method}</Tag>
            <Tag>{tool.path}</Tag>
          </div>
        </div>
        <div style={{ padding: 22, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 22 }}>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>Inputs</div>
            <div style={{ background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, padding: 12 }}>
              {tool.inputs.map((inp) => (
                <div key={inp} style={{ fontFamily: "var(--mono)", fontSize: 12, color: "var(--text-2)", lineHeight: 1.7 }}>
                  <span style={{ color: "var(--accent)" }}>•</span> {inp}
                </div>
              ))}
            </div>
            <div style={{ marginTop: 18, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>Returns</div>
            <div style={{ background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, padding: 12, fontFamily: "var(--mono)", fontSize: 12, color: "var(--text-2)" }}>
              {tool.returns}
            </div>
          </div>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>Example call</div>
            <CodeBlock label="curl" lines={exampleFor(tool)} />
            <div style={{ marginTop: 18, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>MCP descriptor</div>
            <CodeBlock label="mcp.json" mono lines={mcpDescriptor(tool)} />
          </div>
        </div>
      </Card>
    </div>
  );
}

function ImportSection({ text, setText, response, run }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1.1fr 1fr", gap: 18 }}>
      <Card padding={0}>
        <div style={{ padding: "14px 20px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--accent)" }}>assessment.import</div>
            <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, marginTop: 2 }}>Try the import endpoint</div>
            <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 2 }}>Paste JSON or Markdown — validation runs locally and a mock response is returned.</div>
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <Button variant="ghost" size="sm">Markdown</Button>
            <Button variant="solid" size="sm">JSON</Button>
          </div>
        </div>
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          spellCheck={false}
          style={{
            width: "100%", minHeight: 380,
            background: "var(--surface-2)",
            border: "none", borderTop: "1px solid var(--border)",
            color: "var(--text)",
            fontFamily: "var(--mono)", fontSize: 12.5, lineHeight: 1.6,
            padding: "14px 18px",
            outline: "none", resize: "vertical",
          }}
        />
        <div style={{ padding: "12px 20px", borderTop: "1px solid var(--border)", background: "var(--surface)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.5 }}>
            POST /v1/assessments · Authorization: Bearer hk_live_3fY9…
          </div>
          <Button variant="primary" onClick={run} icon={<Icon name="arrow" size={13} color="#0b1410" />}>
            Send request
          </Button>
        </div>
      </Card>

      <Card padding={0}>
        <div style={{ padding: "14px 20px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500 }}>Response</div>
          {response ? (
            <Tag tone={response.ok ? "accent" : "amber"}>{response.ok ? "200 OK" : "200 OK · warnings"}</Tag>
          ) : <Tag tone="ghost">awaiting request</Tag>}
        </div>
        {response ? (
          <div style={{ padding: 20 }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", marginBottom: 10, letterSpacing: 0.5 }}>
              {response.ts} · 142ms · 4 hops
            </div>
            <CodeBlock label="response.json" lines={JSON.stringify(response.body, null, 2).split("\n")} />
            {response.body.warnings.length ? (
              <div style={{ marginTop: 14, padding: 12, background: "var(--amber-dim)", border: "1px solid var(--amber)", borderRadius: 6 }}>
                <div style={{ color: "var(--amber)", fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 0.6, marginBottom: 4 }}>WARNINGS</div>
                {response.body.warnings.map((w) => (
                  <div key={w} style={{ fontSize: 12.5, color: "var(--text-2)", lineHeight: 1.5 }}>• {w}</div>
                ))}
              </div>
            ) : (
              <div style={{ marginTop: 14, padding: 12, background: "var(--accent-dim)", border: "1px solid var(--accent-line)", borderRadius: 6, display: "flex", gap: 10, alignItems: "center" }}>
                <Icon name="check" size={14} color="var(--accent)" />
                <span style={{ fontSize: 12.5, color: "var(--text)" }}>
                  Assessment created. Visit <code style={cd}>{response.body.url}</code> or assign via <code style={cd}>POST /v1/assignments</code>.
                </span>
              </div>
            )}
          </div>
        ) : (
          <div style={{ padding: 40, textAlign: "center" }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.5 }}>
              Click <strong>Send request</strong> to see the parsed response
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}

function LogSection() {
  const events = [
    { t: "14:31:02", tool: "stats.cohort",  agent: "GraderBot",  status: "200", note: "qz_8sd1 · spring-2026 · 71 records" },
    { t: "14:30:51", tool: "attempt.get",   agent: "GraderBot",  status: "200", note: "att_9k4 · jordan.tahir@" },
    { t: "14:28:14", tool: "feedback.send", agent: "TA-Assist",  status: "201", note: "→ 14 learners · in-app" },
    { t: "14:22:08", tool: "assessment.import",   agent: "TA-Assist",  status: "201", note: "qz_imported · BIO 110 · 12 Qs" },
    { t: "14:14:42", tool: "plan.create",   agent: "TA-Assist",  status: "201", note: "jordan.tahir@ · 6-week" },
    { t: "13:59:30", tool: "stats.cohort",  agent: "GraderBot",  status: "200", note: "qz_3kf2 · all · 142 records" },
    { t: "13:48:12", tool: "assessment.generate", agent: "Local dev",  status: "200", note: "from-pdf · 8 Qs · CHEM 220" },
    { t: "13:36:55", tool: "attempt.get",   agent: "Local dev",  status: "403", note: "missing scope attempt.read" },
  ];
  return (
    <Card padding={0}>
      <div style={{ padding: "14px 20px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500 }}>Recent activity</div>
        <div style={{ display: "flex", gap: 6 }}>
          <Tag tone="default">last 30 min</Tag>
          <Button variant="ghost" size="sm" icon={<Icon name="filter" size={12} />}>Filter</Button>
        </div>
      </div>
      <div style={{ fontFamily: "var(--mono)" }}>
        <div style={{
          display: "grid", gridTemplateColumns: "100px 160px 130px 80px 1fr",
          padding: "8px 20px", borderBottom: "1px solid var(--border)",
          background: "var(--surface-2)", fontSize: 10, letterSpacing: 1.1, textTransform: "uppercase", color: "var(--muted)",
        }}>
          <span>Time</span><span>Tool</span><span>Agent</span><span>Status</span><span>Note</span>
        </div>
        {events.map((e, i) => (
          <div key={i} style={{
            display: "grid", gridTemplateColumns: "100px 160px 130px 80px 1fr",
            padding: "12px 20px",
            borderBottom: i < events.length - 1 ? "1px solid var(--border)" : "none",
            fontSize: 12, alignItems: "center",
          }}>
            <span style={{ color: "var(--muted)", letterSpacing: 0.5 }}>{e.t}</span>
            <span style={{ color: "var(--accent)" }}>{e.tool}</span>
            <span style={{ color: "var(--text)" }}>{e.agent}</span>
            <span style={{ color: e.status.startsWith("2") ? "var(--accent)" : "var(--red)", fontWeight: 600 }}>{e.status}</span>
            <span style={{ color: "var(--text-2)" }}>{e.note}</span>
          </div>
        ))}
      </div>
    </Card>
  );
}

function CodeBlock({ label, lines, style, dim = [] }) {
  return (
    <div style={{ background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, overflow: "hidden", ...style }}>
      {label ? (
        <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border)", background: "var(--surface)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: 1 }}>{label}</span>
          <button style={{ background: "transparent", border: "none", color: "var(--muted)", cursor: "pointer", padding: 2 }}>
            <Icon name="copy" size={11} />
          </button>
        </div>
      ) : null}
      <pre style={{
        margin: 0, padding: "12px 14px",
        fontFamily: "var(--mono)", fontSize: 12, lineHeight: 1.65,
        color: "var(--text-2)", whiteSpace: "pre-wrap", wordBreak: "break-word",
      }}>{lines.map((l, i) => (
        <div key={i} style={{ color: dim.includes(i) ? "var(--accent)" : undefined }}>{l || "\u00A0"}</div>
      ))}</pre>
    </div>
  );
}

function exampleFor(tool) {
  const base = `curl https://api.harus.app${tool.path.replace("{id}", "qz_8sd1")} \\`;
  if (tool.method === "GET") {
    return [
      base,
      `  -H "Authorization: Bearer hk_live_3fY9…"`,
    ];
  }
  return [
    `curl -X POST https://api.harus.app${tool.path} \\`,
    `  -H "Authorization: Bearer hk_live_3fY9…" \\`,
    `  -H "Content-Type: application/json" \\`,
    `  -d '{ ... }'`,
  ];
}

function mcpDescriptor(tool) {
  return [
    `{`,
    `  "name": "${tool.name}",`,
    `  "description": ${JSON.stringify(tool.desc)},`,
    `  "input_schema": {`,
    `    "type": "object",`,
    `    "required": [${tool.inputs.filter((i) => !i.includes("?")).map((i) => `"${i.split(":")[0].trim()}"`).join(", ")}]`,
    `  }`,
    `}`,
  ];
}

const cd = {
  fontFamily: "var(--mono)", fontSize: 12,
  background: "var(--surface-2)",
  border: "1px solid var(--border)",
  borderRadius: 4, padding: "1px 6px",
  color: "var(--text)",
};

Object.assign(window, { AgentScreen });
