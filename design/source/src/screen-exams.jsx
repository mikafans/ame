function ExamsScreen({ onStartAssessment }) {
  const [selected, setSelected] = React.useState(EXAMS[0].id);
  const [filter, setFilter] = React.useState("all");
  const exam = EXAMS.find((e) => e.id === selected);

  const filtered = filter === "all" ? EXAMS : EXAMS.filter((e) => e.status === filter);
  const tabs = [
    { id: "all", label: "All", n: EXAMS.length },
    { id: "active", label: "Active", n: EXAMS.filter((e) => e.status === "active").length },
    { id: "scheduled", label: "Scheduled", n: EXAMS.filter((e) => e.status === "scheduled").length },
    { id: "draft", label: "Drafts", n: EXAMS.filter((e) => e.status === "draft").length },
  ];

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      <SectionLabel
        kicker="Composed assessments · multi-assessment · weighted"
        action={
          <div style={{ display: "flex", gap: 8 }}>
            <Button variant="ghost" icon={<Icon name="filter" size={14} />}>Filter</Button>
            <Button variant="solid" icon={<Icon name="plus" size={14} />}>Compose exam</Button>
          </div>
        }>
        Exams
      </SectionLabel>

      <div style={{ display: "flex", gap: 4, borderBottom: "1px solid var(--border)", marginBottom: 22 }}>
        {tabs.map((t) => {
          const active = filter === t.id;
          return (
            <button key={t.id} onClick={() => setFilter(t.id)} style={{
              background: "transparent", border: "none", cursor: "pointer",
              padding: "10px 14px",
              fontSize: 13, fontWeight: active ? 600 : 500,
              color: active ? "var(--text)" : "var(--muted)",
              borderBottom: `2px solid ${active ? "var(--accent)" : "transparent"}`,
              marginBottom: -1,
            }}>
              {t.label} <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", marginLeft: 4 }}>({t.n})</span>
            </button>
          );
        })}
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "340px 1fr", gap: 18 }}>
        {/* List */}
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {filtered.map((e) => <ExamListItem key={e.id} e={e} selected={selected === e.id} onClick={() => setSelected(e.id)} />)}
        </div>

        {/* Detail */}
        <ExamDetail exam={exam} onStart={onStartAssessment} />
      </div>
    </div>
  );
}

function ExamListItem({ e, selected, onClick }) {
  const statusTone = e.status === "active" ? "accent" : e.status === "scheduled" ? "blue" : e.status === "draft" ? "ghost" : "default";
  return (
    <button onClick={onClick} style={{
      width: "100%", textAlign: "left", cursor: "pointer",
      background: selected ? "var(--surface)" : "var(--surface)",
      border: `1px solid ${selected ? "var(--accent-line)" : "var(--border)"}`,
      borderLeft: `3px solid ${selected ? "var(--accent)" : "transparent"}`,
      borderRadius: 6,
      padding: "14px 16px",
      transition: "border-color 140ms",
    }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <span style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)" }}>
          {e.course}
        </span>
        <Tag tone={statusTone}>{e.status}</Tag>
      </div>
      <div style={{ fontFamily: "var(--serif)", fontSize: 16, fontWeight: 500, lineHeight: 1.3, letterSpacing: -0.1, color: "var(--text)" }}>
        {e.title}
      </div>
      <div style={{ marginTop: 10, display: "flex", gap: 14, alignItems: "center", fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.4 }}>
        <span><Icon name="clock" size={11} /> {e.duration}m</span>
        <span><Icon name="results" size={11} /> {e.sections.length} sec</span>
        <span style={{ marginLeft: "auto", color: e.method === "agent" ? "var(--accent)" : "var(--text-2)" }}>
          {e.method === "agent" ? "◇ agent" : "◇ manual"}
        </span>
      </div>
    </button>
  );
}

function ExamDetail({ exam, onStart }) {
  const totalItems = exam.sections.reduce((s, x) => s + x.items, 0);
  const totalWeight = exam.sections.reduce((s, x) => s + x.weight, 0);

  return (
    <Card padding={0}>
      {/* Header */}
      <div style={{ padding: "24px 28px", borderBottom: "1px solid var(--border)" }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <Tag tone={exam.status === "active" ? "accent" : exam.status === "scheduled" ? "blue" : "ghost"}>{exam.status}</Tag>
          <Tag>{exam.course}</Tag>
          {exam.method === "agent" ? <Tag tone="amber">Agent-composed</Tag> : null}
          {exam.tags.map((t) => <Tag key={t}>{t}</Tag>)}
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 24 }}>
          <div>
            <h2 style={{ margin: 0, fontFamily: "var(--serif)", fontSize: 30, fontWeight: 500, letterSpacing: -0.4 }}>
              {exam.title}
            </h2>
            <p style={{ color: "var(--text-2)", fontSize: 13.5, lineHeight: 1.6, margin: "10px 0 0", maxWidth: 600 }}>
              {exam.description}
            </p>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8, alignItems: "flex-end" }}>
            {exam.status === "active" || exam.status === "scheduled" ? (
              <Button variant="primary" size="lg" onClick={onStart}
                disabled={exam.status === "scheduled"}
                icon={<Icon name="arrow" size={14} color="#0b1410" />}>
                {exam.status === "scheduled" ? "Opens Apr 22, 09:00" : "Start exam"}
              </Button>
            ) : (
              <Button variant="solid" size="lg">Edit draft</Button>
            )}
            <ShareButton size="sm" variant="ghost" payload={{
              kind: "exam", id: exam.id, title: exam.title, course: exam.course,
              body: exam.title + " — " + exam.description, attribution: exam.composedBy,
            }} />
            <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.4 }}>
              Composed by {exam.composedBy}
            </span>
          </div>
        </div>
      </div>

      {/* Top stats */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", borderBottom: "1px solid var(--border)" }}>
        {[
          { l: "Duration",    v: exam.duration + " min" },
          { l: "Total items", v: totalItems },
          { l: "Total points", v: exam.totalPoints },
          { l: "Pass mark",   v: exam.passing + "%" },
          { l: "Assigned",    v: exam.assigned },
        ].map((s, i) => (
          <div key={s.l} style={{
            padding: "16px 22px",
            borderRight: i < 4 ? "1px solid var(--border)" : "none",
          }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)", marginBottom: 4 }}>{s.l}</div>
            <div style={{ fontFamily: "var(--serif)", fontSize: 22, fontWeight: 500, letterSpacing: -0.3 }}>{s.v}</div>
          </div>
        ))}
      </div>

      {/* Learning objectives */}
      {exam.objectives && exam.objectives.length ? (
        <div style={{ padding: "22px 28px", borderBottom: "1px solid var(--border)" }}>
          <LearningObjectives items={exam.objectives} />
        </div>
      ) : null}

      {/* Section composition */}
      <div style={{ padding: "22px 28px", borderBottom: "1px solid var(--border)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 14 }}>
          <div style={{ fontFamily: "var(--serif)", fontSize: 17, fontWeight: 500 }}>Composition</div>
          <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.4 }}>
            {exam.sections.length} sections · {totalWeight} pts total
          </div>
        </div>

        {/* Weight bar */}
        <div style={{ display: "flex", height: 8, borderRadius: 4, overflow: "hidden", border: "1px solid var(--border)" }}>
          {exam.sections.map((s, i) => {
            const colors = ["var(--accent)", "var(--blue)", "var(--amber)", "var(--red)"];
            return (
              <div key={s.id} title={`${s.title} — ${s.weight} pts`} style={{
                flex: s.weight,
                background: colors[i % colors.length],
                borderRight: i < exam.sections.length - 1 ? "1px solid var(--bg)" : "none",
              }} />
            );
          })}
        </div>

        {/* Section list */}
        <div style={{ marginTop: 16, display: "flex", flexDirection: "column", gap: 0, border: "1px solid var(--border)", borderRadius: 6, overflow: "hidden" }}>
          <div style={{
            display: "grid", gridTemplateColumns: "44px 1fr 130px 80px 80px",
            padding: "8px 14px",
            background: "var(--surface-2)",
            fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.1, textTransform: "uppercase", color: "var(--muted)",
            borderBottom: "1px solid var(--border)",
          }}>
            <span>§</span><span>Section</span><span>Mix</span><span style={{ textAlign: "right" }}>Items</span><span style={{ textAlign: "right" }}>Weight</span>
          </div>
          {exam.sections.map((s, i) => {
            const colors = ["var(--accent)", "var(--blue)", "var(--amber)", "var(--red)"];
            return (
              <div key={s.id} style={{
                display: "grid", gridTemplateColumns: "44px 1fr 130px 80px 80px",
                padding: "14px 14px",
                borderBottom: i < exam.sections.length - 1 ? "1px solid var(--border)" : "none",
                alignItems: "center",
              }}>
                <span style={{
                  fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500,
                  color: colors[i % colors.length],
                }}>{i + 1}</span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 14, fontWeight: 500, color: "var(--text)" }}>{s.title}</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.4, marginTop: 2 }}>
                    sourced from {s.assessmentId}
                  </div>
                </div>
                <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-2)", letterSpacing: 0.3 }}>{s.mix}</span>
                <span style={{ textAlign: "right", fontFamily: "var(--mono)", fontSize: 13, color: "var(--text)" }}>{s.items}</span>
                <span style={{ textAlign: "right", fontFamily: "var(--serif)", fontSize: 16, fontWeight: 500, color: "var(--text)" }}>{s.weight} <span style={{ color: "var(--muted)", fontSize: 11 }}>pts</span></span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Footer: window, integrity, agent composition info */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr" }}>
        <div style={{ padding: "20px 24px", borderRight: "1px solid var(--border)" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
            Window
          </div>
          <KV k="Opens"  v={exam.open} />
          <KV k="Closes" v={exam.close} />
          <KV k="Attempts allowed" v="1" />
        </div>

        <div style={{ padding: "20px 24px", borderRight: "1px solid var(--border)" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
            Integrity
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8, fontSize: 12.5, color: "var(--text-2)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}><Icon name="check" size={13} color="var(--accent)" /> Browser locked</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}><Icon name="check" size={13} color="var(--accent)" /> Random section order</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}><Icon name="check" size={13} color="var(--accent)" /> Webcam proctor (optional)</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}><Icon name="x" size={13} color="var(--muted)" /> Calculator disabled</div>
          </div>
        </div>

        <div style={{ padding: "20px 24px" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
            Composition trace
          </div>
          {exam.method === "agent" ? (
            <div>
              <div style={{ fontSize: 12.5, color: "var(--text-2)", lineHeight: 1.6, marginBottom: 8 }}>
                Composed by <span style={{ color: "var(--accent)", fontFamily: "var(--mono)" }}>TA-Assist</span> via <span style={{ fontFamily: "var(--mono)" }}>exam.compose</span> on Apr 13.
              </div>
              <div style={{ background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 4, padding: 10, fontFamily: "var(--mono)", fontSize: 11, color: "var(--text-2)", lineHeight: 1.6, letterSpacing: 0.3 }}>
                seed: <span style={{ color: "var(--text)" }}>syllabus-u2</span><br />
                inputs: 3 assessments, 24 items<br />
                strategy: <span style={{ color: "var(--accent)" }}>weighted-by-mastery</span><br />
                confidence: 0.82
              </div>
            </div>
          ) : (
            <div style={{ fontSize: 12.5, color: "var(--text-2)", lineHeight: 1.6 }}>
              Manually composed by <span style={{ color: "var(--text)" }}>{exam.composedBy}</span>. Items locked
              when the exam was published — agent imports cannot modify a published exam.
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}

Object.assign(window, { ExamsScreen });
