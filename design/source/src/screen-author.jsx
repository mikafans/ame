function AuthorScreen() {
  const [selected, setSelected] = React.useState("q1");
  const draftQuestions = [
    { id: "q1", n: 1, type: "mc",    points: 2, prompt: "Which complex splits water in PSII?", status: "ready" },
    { id: "q2", n: 2, type: "tf",    points: 1, prompt: "ATP synthase is found in the thylakoid membrane.", status: "ready" },
    { id: "q3", n: 3, type: "short", points: 2, prompt: "Name the final electron acceptor in the light reactions.", status: "review" },
    { id: "q4", n: 4, type: "essay", points: 6, prompt: "Explain why oxygen is a byproduct of the light reactions, not the Calvin cycle.", status: "draft" },
    { id: "q5", n: 5, type: "code",  points: 4, prompt: "Write a function that returns the redox potential of a given electron carrier.", status: "draft" },
  ];

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      <SectionLabel
        kicker="Editing draft · BIO 110 · autosaved"
        action={
          <div style={{ display: "flex", gap: 8 }}>
            <Button variant="ghost" icon={<Icon name="upload" size={14} />}>Import</Button>
            <Button variant="ghost">Preview</Button>
            <Button variant="solid">Save draft</Button>
            <Button variant="primary" icon={<Icon name="arrow" size={14} color="#0b1410" />}>Publish</Button>
          </div>
        }>
        Author studio
      </SectionLabel>

      {/* Metadata strip */}
      <Card padding={0} style={{ marginBottom: 18 }}>
        <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr 1fr 1fr 1fr", borderBottom: "1px solid var(--border)" }}>
          <Field2 label="Title">
            <input defaultValue="Photosynthesis — Light Reactions" style={inlineInput} />
          </Field2>
          <Field2 label="Course"><input defaultValue="BIO 110" style={inlineInput} /></Field2>
          <Field2 label="Duration">
            <select style={inlineInput} defaultValue="35">
              <option>20</option><option>25</option><option>30</option><option>35</option><option>45</option><option>60</option>
            </select>
          </Field2>
          <Field2 label="Difficulty">
            <select style={inlineInput} defaultValue="Intermediate">
              <option>Introductory</option><option>Intermediate</option><option>Advanced</option>
            </select>
          </Field2>
          <Field2 label="Attempts" last>
            <select style={inlineInput} defaultValue="2">
              <option>1</option><option>2</option><option>3</option><option>Unlimited</option>
            </select>
          </Field2>
        </div>
        <div style={{ padding: "14px 22px", display: "flex", gap: 28, alignItems: "center", fontSize: 12.5, color: "var(--text-2)" }}>
          <Pill icon="check" tone="ok" label="Outline complete" />
          <Pill icon="x" tone="warn" label="2 questions need review" />
          <Pill icon="results" tone="muted" label="5 questions · 15 pts" />
          <span style={{ marginLeft: "auto", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 0.5 }}>
            Last edit · 4 minutes ago · by you
          </span>
        </div>
      </Card>

      <div style={{ display: "grid", gridTemplateColumns: "320px 1fr 280px", gap: 18 }}>
        {/* Questions list */}
        <Card padding={0}>
          <div style={{ padding: "14px 16px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)" }}>Questions</div>
            <Button variant="ghost" size="sm" icon={<Icon name="plus" size={12} />}>Add</Button>
          </div>
          {draftQuestions.map((q) => {
            const sel = selected === q.id;
            return (
              <button key={q.id} onClick={() => setSelected(q.id)} style={{
                width: "100%", padding: "12px 16px",
                background: sel ? "var(--accent-dim)" : "transparent",
                border: "none", borderLeft: `2px solid ${sel ? "var(--accent)" : "transparent"}`,
                borderBottom: "1px solid var(--border)",
                textAlign: "left", cursor: "pointer",
                display: "grid", gridTemplateColumns: "28px 1fr 56px", gap: 10, alignItems: "center",
              }}>
                <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)" }}>Q{q.n}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: sel ? "var(--text)" : "var(--text-2)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", fontWeight: sel ? 500 : 400 }}>
                    {q.prompt}
                  </div>
                  <div style={{ marginTop: 3, display: "flex", gap: 4, alignItems: "center" }}>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: 0.5, textTransform: "uppercase" }}>
                      {questionTypeLabel(q.type)}
                    </span>
                    {q.status === "review" ? <Tag tone="amber">Review</Tag> : q.status === "draft" ? <Tag tone="ghost">Draft</Tag> : null}
                  </div>
                </div>
                <div style={{ textAlign: "right", fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-2)" }}>{q.points}pt</div>
              </button>
            );
          })}
          <div style={{ padding: 14, background: "var(--surface-2)" }}>
            <div style={{ fontSize: 12, color: "var(--text-2)", marginBottom: 8, lineHeight: 1.5 }}>
              Generate from source — paste lecture notes, a PDF, or a reading.
            </div>
            <Button variant="solid" size="sm" icon={<Icon name="sparkle" size={12} />}>Generate questions</Button>
          </div>
        </Card>

        {/* Editor */}
        <Card padding={0}>
          <div style={{ padding: "16px 22px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, color: "var(--muted)", textTransform: "uppercase" }}>Editing Q1</div>
              <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, marginTop: 2 }}>Multiple choice</div>
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              <Button variant="ghost" size="sm">Duplicate</Button>
              <Button variant="ghost" size="sm">Move</Button>
              <Button variant="danger" size="sm">Delete</Button>
            </div>
          </div>

          <div style={{ padding: 22 }}>
            <Field2 label="Question prompt" inline>
              <textarea defaultValue="Which complex splits water in Photosystem II, producing oxygen as a byproduct?" rows={2} style={{ ...inlineInput, padding: 12, fontFamily: "var(--serif)", fontSize: 16, lineHeight: 1.5, resize: "vertical" }} />
            </Field2>

            <div style={{ marginTop: 18 }}>
              <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
                Options · mark the correct answer
              </div>
              {[
                { c: true,  text: "Oxygen-evolving complex (OEC)" },
                { c: false, text: "Cytochrome b6f" },
                { c: false, text: "ATP synthase" },
                { c: false, text: "Plastocyanin" },
              ].map((o, i) => (
                <div key={i} style={{
                  display: "grid", gridTemplateColumns: "30px 1fr 80px",
                  gap: 10, alignItems: "center", marginBottom: 8,
                }}>
                  <button style={{
                    width: 22, height: 22, borderRadius: "50%",
                    background: o.c ? "var(--accent)" : "transparent",
                    border: `1px solid ${o.c ? "var(--accent)" : "var(--border-strong)"}`,
                    cursor: "pointer", justifySelf: "end",
                  }}>{o.c ? <Icon name="check" size={12} color="#0b1410" /> : null}</button>
                  <input defaultValue={o.text} style={inlineInput} />
                  <Button variant="quiet" size="sm">Remove</Button>
                </div>
              ))}
              <Button variant="ghost" size="sm" icon={<Icon name="plus" size={12} />}>Add option</Button>
            </div>

            <div style={{ marginTop: 24, paddingTop: 22, borderTop: "1px solid var(--border)", display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 18 }}>
              <Field2 label="Points" inline><input defaultValue="2" style={inlineInput} /></Field2>
              <Field2 label="Tag" inline><input defaultValue="photosynthesis · psii" style={inlineInput} /></Field2>
              <Field2 label="Difficulty" inline>
                <select style={inlineInput} defaultValue="Intermediate">
                  <option>Introductory</option><option>Intermediate</option><option>Advanced</option>
                </select>
              </Field2>
            </div>

            <div style={{ marginTop: 22 }}>
              <Field2 label="Explanation shown after answering" inline>
                <textarea defaultValue="The OEC, a Mn4CaO5 cluster on the lumenal side of PSII, catalyzes the oxidation of water to O2." rows={2} style={{ ...inlineInput, padding: 12, resize: "vertical" }} />
              </Field2>
            </div>
          </div>
        </Card>

        {/* Right rail */}
        <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
          <Card padding={20}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 12 }}>
              Distribute
            </div>
            <KV k="Assigned cohort" v="BIO 110 · §A" />
            <KV k="Enrolled students" v="64" />
            <KV k="Open" v="Apr 12 → Apr 19" />
            <KV k="Late penalty" v="-10% / day" />
            <KV k="Visibility" v="After submission" />
            <Button variant="ghost" size="sm" style={{ marginTop: 14, width: "100%", justifyContent: "center" }}>Edit distribution</Button>
          </Card>

          <Card padding={20}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 12 }}>
              Rubric · auto-grading
            </div>
            <div style={{ fontSize: 12, color: "var(--text-2)", lineHeight: 1.6 }}>
              MC and TF are graded instantly. Short answers compare against accepted strings. Essays use a 4-criterion rubric — flagged for instructor review when confidence &lt; 0.6.
            </div>
            <div style={{ marginTop: 14, padding: "10px 12px", background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-2)", lineHeight: 1.55, letterSpacing: 0.3 }}>
              criteria:<br />
              &nbsp;&nbsp;clarity: 0–2<br />
              &nbsp;&nbsp;evidence: 0–2<br />
              &nbsp;&nbsp;mechanism: 0–1<br />
              &nbsp;&nbsp;link: 0–1
            </div>
          </Card>

          <Card padding={20}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
              Recent activity
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 10, fontSize: 12 }}>
              {[
                ["TA-Assist", "regenerated Q5 starter code", "4m"],
                ["You", "edited Q1 distractor C", "12m"],
                ["GraderBot", "flagged Q3 as ambiguous", "1h"],
                ["You", "added 2 questions via import", "2h"],
              ].map(([who, what, when]) => (
                <div key={what} style={{ display: "flex", gap: 8 }}>
                  <span style={{ width: 4, alignSelf: "stretch", background: who === "You" ? "var(--accent)" : "var(--border-strong)", borderRadius: 2 }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ color: "var(--text)" }}><span style={{ color: who === "You" ? "var(--accent)" : "var(--text-2)" }}>{who}</span> {what}</div>
                    <div style={{ color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 10.5, marginTop: 2, letterSpacing: 0.5 }}>{when} ago</div>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

const inlineInput = {
  width: "100%",
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 6, padding: "9px 12px",
  color: "var(--text)", fontFamily: "var(--sans)", fontSize: 13.5,
  outline: "none",
};

function Field2({ label, children, last, inline }) {
  if (inline) {
    return (
      <label style={{ display: "block" }}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{label}</div>
        {children}
      </label>
    );
  }
  return (
    <div style={{ padding: "14px 22px", borderRight: last ? "none" : "1px solid var(--border)" }}>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>{label}</div>
      {children}
    </div>
  );
}

function Pill({ icon, tone, label }) {
  const color = tone === "ok" ? "var(--accent)" : tone === "warn" ? "var(--amber)" : "var(--muted)";
  return (
    <div style={{ display: "inline-flex", alignItems: "center", gap: 6, color }}>
      <Icon name={icon} size={13} />
      <span style={{ color: "var(--text-2)", fontSize: 12.5 }}>{label}</span>
    </div>
  );
}

Object.assign(window, { AuthorScreen });
