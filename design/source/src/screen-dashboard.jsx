function DashboardScreen({ statsDepth }) {
  // depth: minimal | standard | full
  const showAll = statsDepth === "full";
  const showStandard = statsDepth === "standard" || showAll;

  return (
    <div style={{ padding: "28px 36px 56px" }}>
      <SectionLabel
        kicker="Spring 2026 · last 12 weeks"
        action={
          <div style={{ display: "flex", gap: 8 }}>
            <Button variant="ghost">12 weeks</Button>
            <Button variant="ghost" icon={<Icon name="download" size={14} />}>Export</Button>
          </div>
        }>
        Progress dashboard
      </SectionLabel>

      {/* Top stat strip */}
      <Card padding={0} style={{ marginBottom: 18 }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)" }}>
          {[
            { l: "Avg score (12w)", v: "76.8%", s: "+9.2 vs prior",  t: "up" },
            { l: "Attempts",         v: "49",     s: "12 this week",   t: "" },
            { l: "Hours spent",      v: "31.4",   s: "↓ 2.1 vs avg",   t: "down" },
            { l: "Current streak",   v: "11 d",   s: "best: 18 d",     t: "" },
            { l: "Mastered topics",  v: "18 / 27",s: "+3 since W8",    t: "up" },
          ].map((s, i) => (
            <div key={s.l} style={{
              padding: "22px 24px",
              borderRight: i < 4 ? "1px solid var(--border)" : "none",
            }}>
              <Stat label={s.l} value={s.v} sub={s.s} tone={s.t} />
            </div>
          ))}
        </div>
      </Card>

      {/* Trend + subject grid */}
      <div style={{ display: "grid", gridTemplateColumns: "1.5fr 1fr", gap: 18, marginBottom: 18 }}>
        <Card padding={24}>
          <SectionHeader title="Score trend" sub="Weekly rolling average across all subjects" />
          <LineChart data={PROGRESS_OVER_TIME} width={760} height={220} />
          <div style={{ display: "flex", gap: 24, marginTop: 14, fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.5 }}>
            <span><span style={{ display: "inline-block", width: 10, height: 2, background: "var(--accent)", marginRight: 4, verticalAlign: "middle" }} /> Your weekly avg</span>
            <span>Highest: W12 · 84%</span>
            <span>Lowest: W1 · 58%</span>
            <span>Trajectory: +26 over term</span>
          </div>
        </Card>

        <Card padding={24}>
          <SectionHeader title="By subject" sub="Average score, last 12 weeks" />
          <BarChart data={SUBJECT_BREAKDOWN} height={232} width={500} />
        </Card>
      </div>

      {showStandard && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 18, marginBottom: 18 }}>
          <Card padding={24}>
            <SectionHeader title="Cohort comparison" sub="Algorithms — Graph Traversal · 71 completers" />
            <Histogram data={COHORT_DISTRIBUTION} highlightBin="70-80" width={500} height={200} />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16, marginTop: 20, paddingTop: 18, borderTop: "1px solid var(--border)" }}>
              <Stat label="Your score" value="73.3%" />
              <Stat label="Cohort median" value="68.5%" />
              <Stat label="Percentile" value="72nd" sub="↑ from 61st" tone="up" />
            </div>
          </Card>

          <Card padding={24}>
            <SectionHeader title="Upcoming" sub="Assigned across courses" />
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {[
                { q: "Macroeconomics: Monetary Policy", c: "ECON 204", d: "in 2 days", color: "var(--blue)" },
                { q: "Organic Chemistry: Stereochemistry", c: "CHEM 220", d: "in 4 days", color: "var(--amber)" },
                { q: "Linear Algebra Review", c: "MATH 110", d: "in 6 days", color: "var(--accent)" },
                { q: "Political Theory — Locke", c: "POL 150", d: "in 9 days", color: "var(--blue)" },
              ].map((u, i) => (
                <div key={i} style={{
                  display: "grid", gridTemplateColumns: "8px 1fr auto",
                  gap: 14, padding: "12px 14px",
                  background: "var(--surface-2)",
                  border: "1px solid var(--border)",
                  borderRadius: 6,
                  alignItems: "center",
                }}>
                  <div style={{ width: 8, height: 8, borderRadius: "50%", background: u.color }} />
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontSize: 13, fontWeight: 500 }}>{u.q}</div>
                    <div style={{ fontSize: 11, color: "var(--muted)", fontFamily: "var(--mono)", letterSpacing: 0.5, textTransform: "uppercase", marginTop: 2 }}>{u.c} · due {u.d}</div>
                  </div>
                  <Button variant="ghost" size="sm">Open</Button>
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}

      {showAll && (
        <Card padding={24} style={{ marginBottom: 18 }}>
          <SectionHeader title="Item analysis" sub="Graph Traversal · difficulty vs. discrimination index (item response theory)" />
          <ScatterChart data={ITEM_ANALYSIS} width={900} height={280} />
          <div style={{
            marginTop: 18, paddingTop: 18, borderTop: "1px solid var(--border)",
            display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 16,
          }}>
            {ITEM_ANALYSIS.map((it) => (
              <div key={it.q} style={{ padding: "12px 14px", background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 1 }}>{it.q}</span>
                  <Tag tone={it.correct < 40 ? "red" : it.correct < 70 ? "amber" : "accent"}>{it.correct}%</Tag>
                </div>
                <div style={{ marginTop: 6, fontSize: 12.5, color: "var(--text)", lineHeight: 1.4 }}>{it.topic}</div>
                <div style={{ marginTop: 6, fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 0.4 }}>
                  diff {it.difficulty.toFixed(2)} · disc {it.discrim.toFixed(2)}
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {showAll && (
        <div style={{ display: "grid", gridTemplateColumns: "1.3fr 1fr", gap: 18 }}>
          <Card padding={24}>
            <SectionHeader title="Study plan — generated by TA-Assist" sub="6-week recovery plan derived from last 4 attempts" />
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {[
                { w: "Week 1", t: "Graph representations", d: "Read § 22.1, practice adjacency-list construction", h: "2.5h" },
                { w: "Week 2", t: "Iterative DFS w/ visited", d: "Implement on 5 test graphs incl. cycles", h: "3.0h" },
                { w: "Week 3", t: "Topological sort proofs", d: "Two proof exercises + practice quiz", h: "2.0h" },
                { w: "Week 4", t: "Complexity practice",     d: "Drill big-O for 10 traversal variants", h: "1.5h" },
                { w: "Week 5", t: "Comparative writing",     d: "Essay rewrites with rubric coaching", h: "2.5h" },
                { w: "Week 6", t: "Mock midterm",            d: "Timed full-length attempt + review", h: "2.0h" },
              ].map((s, i, arr) => (
                <div key={s.w} style={{ display: "grid", gridTemplateColumns: "70px 1fr 60px", gap: 14, padding: "10px 0", borderBottom: i < arr.length - 1 ? "1px dashed var(--border)" : "none" }}>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 1, color: "var(--accent)", textTransform: "uppercase", paddingTop: 2 }}>{s.w}</div>
                  <div>
                    <div style={{ fontSize: 14, fontWeight: 500 }}>{s.t}</div>
                    <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 2 }}>{s.d}</div>
                  </div>
                  <div style={{ textAlign: "right", fontFamily: "var(--mono)", fontSize: 12, color: "var(--text-2)" }}>{s.h}</div>
                </div>
              ))}
            </div>
            <div style={{ marginTop: 16, padding: 12, background: "var(--accent-dim)", border: "1px solid var(--accent-line)", borderRadius: 6, display: "flex", alignItems: "center", gap: 10 }}>
              <Icon name="sparkle" size={14} color="var(--accent)" />
              <span style={{ fontSize: 12.5, color: "var(--text)" }}>
                Generated by <span style={{ fontFamily: "var(--mono)" }}>plan.create</span> using your last 30 days of attempts.
              </span>
              <span style={{ marginLeft: "auto" }}><Button variant="ghost" size="sm">Customize</Button></span>
            </div>
          </Card>

          <Card padding={24}>
            <SectionHeader title="Mastery map" sub="Topic-level proficiency · last 30 days" />
            <div style={{ display: "grid", gridTemplateColumns: "repeat(6, 1fr)", gap: 6 }}>
              {Array.from({ length: 30 }).map((_, i) => {
                const v = Math.max(0, Math.min(1, 0.35 + Math.sin(i * 0.6) * 0.4 + (i / 60)));
                return (
                  <div key={i} style={{
                    aspectRatio: "1",
                    background: `color-mix(in oklch, var(--accent) ${Math.round(v * 100)}%, var(--surface-2))`,
                    border: "1px solid var(--border)",
                    borderRadius: 3,
                  }} />
                );
              })}
            </div>
            <div style={{ marginTop: 12, fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 0.5, display: "flex", justifyContent: "space-between" }}>
              <span>30 topics tracked</span>
              <span>Low → High</span>
            </div>
            <div style={{ marginTop: 20, paddingTop: 18, borderTop: "1px solid var(--border)", display: "flex", flexDirection: "column", gap: 8 }}>
              {[
                ["Strong", "Linear algebra · Stereochemistry · Membranes"],
                ["Working", "Monetary policy · Political theory"],
                ["Needs review", "Graph traversal · Recursion proofs"],
              ].map(([k, v]) => (
                <div key={k}>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)", marginBottom: 2 }}>{k}</div>
                  <div style={{ fontSize: 12.5, color: "var(--text-2)" }}>{v}</div>
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}

function SectionHeader({ title, sub }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, letterSpacing: -0.2 }}>{title}</div>
      {sub ? <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 2 }}>{sub}</div> : null}
    </div>
  );
}

Object.assign(window, { DashboardScreen });
