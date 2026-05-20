function ResultsScreen({ onContinue, onReview }) {
  const r = LAST_ATTEMPT;
  const { open } = useShare();

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      <SectionLabel
        kicker={`Submitted ${r.submitted} · ${r.course}`}
        action={
          <div style={{ display: "flex", gap: 8 }}>
            <Button variant="ghost" icon={<Icon name="download" size={14} />}>Export PDF</Button>
            <Button variant="solid" icon={<Icon name="upload" size={14} />} onClick={() => open({
              kind: "quiz", id: r.quizId, title: r.quizTitle, course: r.course,
              body: `I just attempted ${r.quizTitle}. Try it yourself.`,
              attribution: "shared from Harus",
            })}>Share quiz</Button>
          </div>
        }>
        {r.quizTitle} — Results
      </SectionLabel>

      <div style={{ display: "grid", gridTemplateColumns: "1.3fr 1fr", gap: 18, marginBottom: 24 }}>
        {/* Big score panel */}
        <Card padding={28}>
          <div style={{ display: "flex", gap: 28, alignItems: "center" }}>
            <DonutChart correct={r.score} total={r.total} size={140} />
            <div style={{ flex: 1 }}>
              <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 10 }}>
                <Tag tone="accent">Pass</Tag>
                <Tag>{r.percent.toFixed(1)}%</Tag>
                <Tag tone="ghost">Above class avg</Tag>
              </div>
              <div style={{ fontFamily: "var(--serif)", fontSize: 38, fontWeight: 500, letterSpacing: -0.6, lineHeight: 1 }}>
                {r.score} <span style={{ color: "var(--muted)", fontSize: 22 }}>/ {r.total} pts</span>
              </div>
              <div style={{ marginTop: 14, color: "var(--text-2)", fontSize: 13.5, lineHeight: 1.6, maxWidth: 460 }}>
                Strong on conceptual items. Two graded items need work: the iterative DFS implementation
                missed cycle handling, and your essay could use concrete examples for each approach.
              </div>
            </div>
          </div>

          <div style={{ marginTop: 24, paddingTop: 20, borderTop: "1px solid var(--border)", display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 24 }}>
            <Stat label="Duration" value="31:04" sub="14 min under limit" tone="up" />
            <Stat label="Cohort avg" value="68.5%" sub="+4.8 above" tone="up" />
            <Stat label="Percentile" value="72nd" sub="of 71 completers" />
            <Stat label="Topic mastery" value="B+" sub="up from B" tone="up" />
          </div>
        </Card>

        {/* Cohort context */}
        <Card padding={24}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
            Cohort distribution
          </div>
          <Histogram data={COHORT_DISTRIBUTION} highlightBin="70-80" height={170} />
          <div style={{ marginTop: 12, fontSize: 12, color: "var(--muted)", display: "flex", gap: 16, fontFamily: "var(--mono)", letterSpacing: 0.5 }}>
            <span><span style={{ display: "inline-block", width: 8, height: 8, background: "var(--accent)", marginRight: 4 }} /> Your bin</span>
            <span><span style={{ display: "inline-block", width: 8, height: 8, background: "var(--surface-3)", marginRight: 4 }} /> Cohort</span>
          </div>
        </Card>
      </div>

      {/* Per-question review */}
      <Card padding={0}>
        <div style={{ padding: "16px 22px", borderBottom: "1px solid var(--border)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)" }}>Per-item</div>
            <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, marginTop: 2 }}>Answer review</div>
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <Tag tone="accent">3 correct</Tag>
            <Tag tone="red">2 needs work</Tag>
          </div>
        </div>
        <div>
          {r.answers.map((a, i) => (
            <div key={a.qid} style={{
              padding: "18px 22px",
              borderBottom: i < r.answers.length - 1 ? "1px solid var(--border)" : "none",
              display: "grid", gridTemplateColumns: "48px 1fr 110px",
              gap: 18, alignItems: "start",
            }}>
              <div style={{
                width: 40, height: 40,
                background: a.correct ? "var(--accent-dim)" : "var(--red-dim)",
                border: `1px solid ${a.correct ? "var(--accent-line)" : "var(--red)"}`,
                color: a.correct ? "var(--accent)" : "var(--red)",
                borderRadius: 6,
                display: "flex", alignItems: "center", justifyContent: "center",
              }}>
                <Icon name={a.correct ? "check" : "x"} size={16} />
              </div>
              <div style={{ minWidth: 0 }}>
                <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 1.1, textTransform: "uppercase" }}>
                    Q{i + 1} · {questionTypeLabel(a.type)}
                  </span>
                </div>
                <div style={{ fontFamily: "var(--serif)", fontSize: 15, lineHeight: 1.5, color: "var(--text)" }}>
                  {ACTIVE_QUIZ.questions[i].prompt}
                </div>
                <div style={{
                  marginTop: 10,
                  display: "flex", flexDirection: "column", gap: 4,
                  fontSize: 13, color: "var(--text-2)",
                }}>
                  <div>
                    <span style={{ fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 0.5, color: "var(--muted)", marginRight: 8, textTransform: "uppercase" }}>Your answer</span>
                    <span style={{ fontFamily: a.type === "code" || a.type === "short" ? "var(--mono)" : "inherit" }}>{a.given}</span>
                  </div>
                  <div style={{ fontSize: 12.5, color: "var(--muted)", lineHeight: 1.5, marginTop: 2 }}>
                    {a.note}
                  </div>
                </div>
              </div>
              <div style={{ textAlign: "right" }}>
                <div style={{ fontFamily: "var(--serif)", fontSize: 22, fontWeight: 500, color: a.correct ? "var(--text)" : "var(--text-2)" }}>
                  {a.points}<span style={{ color: "var(--muted)", fontSize: 14 }}> / {a.max}</span>
                </div>
                <div style={{ marginTop: 6, display: "flex", gap: 10, justifyContent: "flex-end" }}>
                  <button onClick={() => open({
                    kind: "item",
                    id: a.qid,
                    title: `Q${i + 1} · ${r.quizTitle}`,
                    course: r.course,
                    body: ACTIVE_QUIZ.questions[i].prompt,
                    explanation: ACTIVE_QUIZ.questions[i].explanation || a.note,
                    attribution: "Harus",
                  })} style={{ background: "transparent", border: "none", color: "var(--muted)", fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 0.8, cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 4 }}>
                    <Icon name="upload" size={11} /> Share
                  </button>
                  <button style={{ background: "transparent", border: "none", color: "var(--accent)", fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 0.8, cursor: "pointer" }}>
                    See solution →
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
        <div style={{ padding: "18px 22px", display: "flex", justifyContent: "space-between", alignItems: "center", background: "var(--surface-2)", borderTop: "1px solid var(--border)" }}>
          <div style={{ fontSize: 12.5, color: "var(--text-2)" }}>
            Generated study plan available — focuses on cycle handling and complexity proofs.
          </div>
          <div style={{ display: "flex", gap: 10 }}>
            <Button variant="ghost" onClick={onContinue}>Back to library</Button>
            <Button variant="primary" onClick={onReview} icon={<Icon name="arrow" size={14} color="#0b1410" />}>View 6-week plan</Button>
          </div>
        </div>
      </Card>
    </div>
  );
}

Object.assign(window, { ResultsScreen });
