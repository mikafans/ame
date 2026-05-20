function LibraryScreen({ onOpen }) {
  const [filter, setFilter] = React.useState("all");
  const tabs = [
    { id: "all", label: "All quizzes", count: 24 },
    { id: "assigned", label: "Assigned to me", count: 4 },
    { id: "completed", label: "Completed", count: 11 },
    { id: "drafts", label: "Drafts", count: 3 },
  ];

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      <SectionLabel kicker="Spring 2026 · Active term"
        action={
          <div style={{ display: "flex", gap: 8 }}>
            <Button variant="ghost" icon={<Icon name="filter" size={14} />}>Filter</Button>
            <Button variant="solid" icon={<Icon name="plus" size={14} />}>New quiz</Button>
          </div>
        }>
        Library
      </SectionLabel>

      {/* tabs */}
      <div style={{ display: "flex", gap: 4, borderBottom: "1px solid var(--border)", marginBottom: 24 }}>
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
              {t.label} <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", marginLeft: 4 }}>({t.count})</span>
            </button>
          );
        })}
      </div>

      {/* Featured pinned quiz */}
      <Card padding={0} style={{ marginBottom: 24, overflow: "hidden" }}>
        <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1fr" }}>
          <div style={{ padding: "28px 32px", borderRight: "1px solid var(--border)" }}>
            <div style={{ display: "flex", gap: 8, marginBottom: 14 }}>
              <Tag tone="accent">Up next</Tag>
              <Tag>CS 311 · Algorithms</Tag>
              <Tag tone="amber">Due in 2 days</Tag>
            </div>
            <h3 style={{ margin: 0, fontFamily: "var(--serif)", fontSize: 28, fontWeight: 500, letterSpacing: -0.3 }}>
              Algorithms — Graph Traversal
            </h3>
            <p style={{ color: "var(--text-2)", fontSize: 14, lineHeight: 1.55, marginTop: 10, maxWidth: 540 }}>
              A 45-minute checkpoint covering BFS, DFS, topological sort, and complexity analysis.
              Mixed format: multiple choice, short answer, and one coding question.
            </p>

            <div style={{ marginTop: 18 }}>
              <LearningObjectives items={ACTIVE_QUIZ.objectives} />
            </div>
            <div style={{ display: "flex", gap: 22, marginTop: 22, alignItems: "center" }}>
              <div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: 1.2, textTransform: "uppercase" }}>Questions</div>
                <div style={{ fontFamily: "var(--serif)", fontSize: 20, fontWeight: 500 }}>12</div>
              </div>
              <Divider vertical style={{ height: 32 }} />
              <div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: 1.2, textTransform: "uppercase" }}>Duration</div>
                <div style={{ fontFamily: "var(--serif)", fontSize: 20, fontWeight: 500 }}>45 min</div>
              </div>
              <Divider vertical style={{ height: 32 }} />
              <div>
                <div style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: 1.2, textTransform: "uppercase" }}>Attempts</div>
                <div style={{ fontFamily: "var(--serif)", fontSize: 20, fontWeight: 500 }}>0 / 2</div>
              </div>
            </div>
            <div style={{ display: "flex", gap: 10, marginTop: 24 }}>
              <Button variant="primary" size="lg" onClick={() => onOpen("quiz")} icon={<Icon name="arrow" size={14} color="#0b1410" />}>
                Start attempt
              </Button>
              <Button variant="ghost" size="lg">Preview questions</Button>
              <ShareButton size="md" variant="ghost" payload={{
                kind: "quiz",
                id: ACTIVE_QUIZ.id,
                title: ACTIVE_QUIZ.title,
                course: ACTIVE_QUIZ.course,
                body: ACTIVE_QUIZ.title + " — a 45-minute checkpoint on graph traversal.",
                attribution: ACTIVE_QUIZ.author || "Prof. M. Iwata",
              }} />
            </div>
          </div>
          <div style={{ padding: "28px 32px", background: "var(--surface-2)" }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 14 }}>
              Cohort context
            </div>
            <KV k="Class average" v="68.5%" mono />
            <KV k="Class completion" v="71 / 86" mono />
            <KV k="Hardest item" v="Q4 — iterative DFS" />
            <KV k="Topic mastery" v="Intermediate" />
            <KV k="Your last related score" v="74.0%" mono />

            <div style={{ marginTop: 20, fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
              Recommended prep
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {["Lecture 8 — Graph representations", "Worksheet — BFS trace", "Reading § 22.2 — DFS"].map((r) => (
                <div key={r} style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 13, color: "var(--text-2)" }}>
                  <Icon name="book" size={13} color="var(--accent)" />
                  {r}
                </div>
              ))}
            </div>
          </div>
        </div>
      </Card>

      {/* Grid of quizzes */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(330px, 1fr))", gap: 16 }}>
        {QUIZZES.map((q) => <QuizCard key={q.id} q={q} onOpen={onOpen} />)}
      </div>
    </div>
  );
}

function QuizCard({ q, onOpen }) {
  const accentColor = q.color === "amber" ? "var(--amber)" : q.color === "blue" ? "var(--blue)" : "var(--accent)";
  const { open } = useShare();
  return (
    <Card hoverable padding={0} style={{ overflow: "hidden", display: "flex", flexDirection: "column" }}>
      <div style={{
        padding: "16px 18px",
        borderBottom: "1px solid var(--border)",
        display: "flex", alignItems: "center", justifyContent: "space-between",
      }}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, color: "var(--muted)", textTransform: "uppercase" }}>
          {q.course} · {q.difficulty}
        </div>
        {q.status === "draft" ? <Tag tone="ghost">Draft</Tag> : <Tag tone="default">{q.updated}</Tag>}
      </div>
      <div style={{ padding: "18px 18px 0", flex: 1 }}>
        <div style={{ display: "flex", gap: 12, alignItems: "flex-start" }}>
          <div style={{
            width: 4, alignSelf: "stretch",
            background: accentColor,
            borderRadius: 2,
          }} />
          <div style={{ flex: 1, minWidth: 0 }}>
            <h3 style={{ margin: 0, fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, lineHeight: 1.25, letterSpacing: -0.2 }}>
              {q.title}
            </h3>
            <p style={{ color: "var(--muted)", fontSize: 12.5, lineHeight: 1.5, margin: "8px 0 0" }}>{q.description}</p>
          </div>
        </div>
      </div>
      <div style={{ padding: "16px 18px", display: "flex", gap: 14, alignItems: "center", color: "var(--muted)", fontSize: 12, fontFamily: "var(--mono)", letterSpacing: 0.4 }}>
        <span><Icon name="results" size={12} /> {q.questions} Qs</span>
        <span><Icon name="clock" size={12} /> {q.duration}m</span>
        <span style={{ marginLeft: "auto", color: "var(--text-2)" }}>{q.completed}/{q.assigned}</span>
      </div>
      <div style={{
        padding: "12px 18px",
        borderTop: "1px solid var(--border)",
        background: "var(--surface-2)",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <div style={{ display: "flex", gap: 6 }}>
          {q.tags.slice(0, 2).map((t) => <Tag key={t}>{t}</Tag>)}
        </div>
        <div style={{ display: "flex", gap: 14, alignItems: "center" }}>
          <button onClick={() => open({
            kind: "quiz", id: q.id, title: q.title, course: q.course,
            body: q.title + " — " + q.description, attribution: q.author,
          })} style={{
            background: "transparent", border: "none", cursor: "pointer",
            color: "var(--muted)", padding: 0,
            display: "inline-flex", alignItems: "center", gap: 4,
            fontSize: 11, fontFamily: "var(--mono)", letterSpacing: 0.5,
          }}>
            <Icon name="upload" size={12} /> Share
          </button>
          <button onClick={() => onOpen("quiz")} style={{
            background: "transparent", border: "none",
            color: "var(--accent)", fontWeight: 600, fontSize: 12,
            display: "inline-flex", alignItems: "center", gap: 4,
            cursor: "pointer",
          }}>
            Open <Icon name="arrow" size={12} color="var(--accent)" />
          </button>
        </div>
      </div>
    </Card>
  );
}

Object.assign(window, { LibraryScreen });
