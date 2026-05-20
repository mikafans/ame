function QuizScreen({ onSubmit }) {
  const quiz = ACTIVE_QUIZ;
  const [stage, setStage] = React.useState("setup"); // setup | active
  const [idx, setIdx] = React.useState(0);
  const [answers, setAnswers] = React.useState({});
  const [flagged, setFlagged] = React.useState({});
  const [timeLeft, setTimeLeft] = React.useState(31 * 60 + 4); // 31:04
  const [config, setConfig] = React.useState(null);

  React.useEffect(() => {
    if (stage !== "active") return;
    const t = setInterval(() => setTimeLeft((s) => Math.max(0, s - 1)), 1000);
    return () => clearInterval(t);
  }, [stage]);

  if (stage === "setup") {
    return <QuizSetup onStart={(cfg) => { setConfig(cfg); setStage("active"); setTimeLeft(cfg.duration * 60); }} />;
  }

  const cur = quiz.questions[idx];
  const total = quiz.questions.length;
  const answered = Object.keys(answers).length;

  const setAns = (val) => setAnswers((a) => ({ ...a, [cur.id]: val }));
  const toggleFlag = () => setFlagged((f) => ({ ...f, [cur.id]: !f[cur.id] }));

  const mm = Math.floor(timeLeft / 60);
  const ss = String(timeLeft % 60).padStart(2, "0");

  return (
    <div style={{ minHeight: "calc(100vh)", display: "grid", gridTemplateColumns: "1fr 280px" }}>
      {/* Main quiz body */}
      <div style={{ padding: "0 0 56px", borderRight: "1px solid var(--border)" }}>
        {/* sticky exam header */}
        <div style={{
          position: "sticky", top: 0, zIndex: 4,
          background: "var(--bg)",
          borderBottom: "1px solid var(--border)",
          padding: "16px 40px",
          display: "flex", alignItems: "center", justifyContent: "space-between",
        }}>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, color: "var(--muted)", textTransform: "uppercase", marginBottom: 4 }}>
              Attempt 1 of 2 · {quiz.course}
            </div>
            <div style={{ fontFamily: "var(--serif)", fontSize: 18, fontWeight: 500, letterSpacing: -0.2 }}>{quiz.title}</div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
            <div style={{
              padding: "8px 14px",
              border: `1px solid ${timeLeft < 300 ? "var(--red)" : "var(--border)"}`,
              borderRadius: 6,
              background: timeLeft < 300 ? "var(--red-dim)" : "var(--surface)",
              fontFamily: "var(--mono)", fontSize: 14, fontWeight: 600,
              color: timeLeft < 300 ? "var(--red)" : "var(--text)",
              display: "flex", gap: 8, alignItems: "center",
              letterSpacing: 0.5,
            }}>
              <Icon name="clock" size={14} />
              {mm}:{ss}
            </div>
            <Button variant="ghost">Save & exit</Button>
          </div>
        </div>

        {/* progress bar */}
        <div style={{ height: 3, background: "var(--surface-2)", position: "relative" }}>
          <div style={{
            width: `${(answered / total) * 100}%`,
            height: "100%", background: "var(--accent)",
            transition: "width 200ms",
          }} />
        </div>

        {/* Question content */}
        <div style={{ padding: "44px 56px", maxWidth: 820 }}>
          <div style={{
            display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 10,
          }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 1.3, color: "var(--muted)", textTransform: "uppercase" }}>
              Question {idx + 1} of {total} · {questionTypeLabel(cur.type)} · {cur.points} {cur.points === 1 ? "pt" : "pts"}
            </div>
            <button onClick={toggleFlag} style={{
              background: "transparent", border: "none", cursor: "pointer",
              display: "inline-flex", alignItems: "center", gap: 4,
              fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 1,
              color: flagged[cur.id] ? "var(--amber)" : "var(--muted)",
            }}>
              <Icon name="flag" size={12} /> {flagged[cur.id] ? "Flagged" : "Flag for review"}
            </button>
          </div>

          <h2 style={{
            fontFamily: "var(--serif)", fontSize: 26, lineHeight: 1.35,
            fontWeight: 500, letterSpacing: -0.2, margin: 0, color: "var(--text)",
          }}>
            {cur.prompt}
          </h2>

          <div style={{ marginTop: 32 }}>
            {cur.type === "mc"    && <MCInput     q={cur} value={answers[cur.id]} onChange={setAns} />}
            {cur.type === "tf"    && <TFInput     q={cur} value={answers[cur.id]} onChange={setAns} />}
            {cur.type === "short" && <ShortInput  q={cur} value={answers[cur.id]} onChange={setAns} />}
            {cur.type === "code"  && <CodeInput   q={cur} value={answers[cur.id]} onChange={setAns} />}
            {cur.type === "essay" && <EssayInput  q={cur} value={answers[cur.id]} onChange={setAns} />}
          </div>

          {/* Footer nav */}
          <div style={{
            marginTop: 48, paddingTop: 24, borderTop: "1px solid var(--border)",
            display: "flex", justifyContent: "space-between", alignItems: "center",
          }}>
            <Button variant="ghost" onClick={() => setIdx((i) => Math.max(0, i - 1))} disabled={idx === 0} icon={<Icon name="arrowL" size={14} />}>
              Previous
            </Button>
            {idx < total - 1 ? (
              <Button variant="primary" onClick={() => setIdx((i) => Math.min(total - 1, i + 1))} icon={<Icon name="arrow" size={14} color="#0b1410" />}>
                Next question
              </Button>
            ) : (
              <Button variant="primary" onClick={onSubmit} icon={<Icon name="check" size={14} color="#0b1410" />}>
                Submit attempt
              </Button>
            )}
          </div>
        </div>
      </div>

      {/* Right rail: question palette */}
      <aside style={{ padding: "20px 22px", background: "var(--surface)" }}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
          Question palette
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 6, marginBottom: 18 }}>
          {quiz.questions.map((q, i) => {
            const status = answers[q.id] !== undefined ? "answered" : "unanswered";
            const isCur = i === idx;
            const isFlag = flagged[q.id];
            return (
              <button key={q.id} onClick={() => setIdx(i)} style={{
                height: 36, position: "relative",
                background: status === "answered" ? "var(--accent-dim)" : "var(--surface-2)",
                color: status === "answered" ? "var(--accent)" : "var(--text-2)",
                border: `1px solid ${isCur ? "var(--accent)" : status === "answered" ? "var(--accent-line)" : "var(--border)"}`,
                fontFamily: "var(--mono)", fontSize: 12, fontWeight: 600,
                borderRadius: 4, cursor: "pointer",
              }}>
                {i + 1}
                {isFlag ? <span style={{
                  position: "absolute", top: 2, right: 3,
                  width: 5, height: 5, borderRadius: "50%", background: "var(--amber)",
                }} /> : null}
              </button>
            );
          })}
        </div>

        <Divider />

        <div style={{ marginTop: 18, display: "flex", flexDirection: "column", gap: 8, fontSize: 12 }}>
          <Legend swatch="var(--accent-dim)" border="var(--accent-line)" label={`Answered · ${answered}`} />
          <Legend swatch="var(--surface-2)" border="var(--border)" label={`Unanswered · ${total - answered}`} />
          <Legend dot="var(--amber)" label={`Flagged · ${Object.values(flagged).filter(Boolean).length}`} />
        </div>

        <Divider style={{ marginTop: 18 }} />

        <div style={{ marginTop: 18 }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
            Integrity
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8, fontSize: 12, color: "var(--text-2)" }}>
            <Row icon="check" tone="ok" text="Browser locked" />
            <Row icon="check" tone="ok" text="Single tab session" />
            <Row icon="check" tone="ok" text="Autosave every 8 s" />
          </div>
        </div>

        <div style={{ marginTop: 26, padding: 14, background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6 }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.2, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>Allowed</div>
          <ul style={{ margin: 0, paddingLeft: 16, color: "var(--text-2)", fontSize: 12, lineHeight: 1.7 }}>
            <li>One sheet of notes (any)</li>
            <li>Class textbook (printed)</li>
            <li>Standard calculator</li>
          </ul>
        </div>
      </aside>
    </div>
  );
}

function questionTypeLabel(t) {
  return { mc: "Multiple choice", tf: "True / false", short: "Short answer", essay: "Essay", code: "Code" }[t] || t;
}

function Legend({ swatch, border, dot, label }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, color: "var(--text-2)" }}>
      {swatch ? (
        <div style={{ width: 14, height: 14, background: swatch, border: `1px solid ${border}`, borderRadius: 3 }} />
      ) : (
        <div style={{ width: 14, display: "flex", justifyContent: "center" }}>
          <div style={{ width: 6, height: 6, borderRadius: "50%", background: dot }} />
        </div>
      )}
      <span>{label}</span>
    </div>
  );
}

function Row({ icon, tone, text }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <Icon name={icon} size={13} color={tone === "ok" ? "var(--accent)" : "var(--muted)"} />
      <span>{text}</span>
    </div>
  );
}

/* ---------- Inputs ---------- */

function MCInput({ q, value, onChange }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {q.options.map((o, i) => {
        const sel = value === o.id;
        const letter = String.fromCharCode(65 + i);
        return (
          <button key={o.id} onClick={() => onChange(o.id)} style={{
            display: "flex", gap: 16, alignItems: "center",
            padding: "16px 18px",
            background: sel ? "var(--accent-dim)" : "var(--surface)",
            border: `1px solid ${sel ? "var(--accent-line)" : "var(--border)"}`,
            color: sel ? "var(--text)" : "var(--text-2)",
            borderRadius: 6, textAlign: "left",
            cursor: "pointer", fontSize: 15,
            transition: "background 140ms, border-color 140ms",
          }}>
            <div style={{
              width: 30, height: 30, flex: "0 0 30px",
              borderRadius: 4,
              background: sel ? "var(--accent)" : "var(--surface-2)",
              border: `1px solid ${sel ? "var(--accent)" : "var(--border)"}`,
              color: sel ? "#0b1410" : "var(--muted)",
              fontFamily: "var(--mono)", fontSize: 13, fontWeight: 600,
              display: "flex", alignItems: "center", justifyContent: "center",
            }}>{letter}</div>
            <span style={{ flex: 1 }}>{o.text}</span>
            {sel ? <Icon name="check" size={16} color="var(--accent)" /> : null}
          </button>
        );
      })}
    </div>
  );
}

function TFInput({ q, value, onChange }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
      {[
        { v: true,  label: "True"  },
        { v: false, label: "False" },
      ].map(({ v, label }) => {
        const sel = value === v;
        return (
          <button key={label} onClick={() => onChange(v)} style={{
            padding: "26px 18px",
            background: sel ? "var(--accent-dim)" : "var(--surface)",
            border: `1px solid ${sel ? "var(--accent-line)" : "var(--border)"}`,
            color: sel ? "var(--accent)" : "var(--text-2)",
            borderRadius: 6,
            fontFamily: "var(--serif)", fontSize: 24, fontWeight: 500,
            cursor: "pointer",
          }}>
            {label}
          </button>
        );
      })}
    </div>
  );
}

function ShortInput({ q, value, onChange }) {
  return (
    <div>
      <input
        autoFocus
        value={value || ""}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Type your answer…"
        style={{
          width: "100%",
          padding: "16px 18px",
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6, color: "var(--text)",
          fontSize: 18, fontFamily: "var(--mono)",
          outline: "none",
        }}
      />
      <div style={{ marginTop: 8, fontSize: 11, fontFamily: "var(--mono)", color: "var(--muted)", letterSpacing: 0.5 }}>
        Accepted format examples: O(V+E), O(|V|+|E|)
      </div>
    </div>
  );
}

function CodeInput({ q, value, onChange }) {
  const cur = value || q.starter;
  return (
    <div style={{ background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6, overflow: "hidden" }}>
      <div style={{
        padding: "8px 14px", borderBottom: "1px solid var(--border)",
        background: "var(--surface)",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.8 }}>
          <Icon name="code" size={12} /> solution.py · python 3.11
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <Button variant="ghost" size="sm">Run tests</Button>
          <Button variant="quiet" size="sm">Reset</Button>
        </div>
      </div>
      <textarea
        value={cur}
        onChange={(e) => onChange(e.target.value)}
        spellCheck={false}
        rows={10}
        style={{
          width: "100%",
          background: "transparent",
          color: "var(--text)",
          fontFamily: "var(--mono)", fontSize: 13.5, lineHeight: 1.6,
          padding: "16px 18px",
          border: "none", outline: "none",
          resize: "vertical",
          minHeight: 220,
        }}
      />
      <div style={{ padding: "10px 14px", borderTop: "1px solid var(--border)", background: "var(--surface)" }}>
        <div style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", marginBottom: 4, letterSpacing: 0.8 }}>
          ◇ Test runner output
        </div>
        <div style={{ fontFamily: "var(--mono)", fontSize: 12, color: "var(--text-2)" }}>
          <span style={{ color: "var(--accent)" }}>✓</span> case 1: small graph — passed{"  "}
          <span style={{ color: "var(--accent)" }}>✓</span> case 2: disconnected — passed{"  "}
          <span style={{ color: "var(--red)" }}>✗</span> case 3: cycle — failed
        </div>
      </div>
    </div>
  );
}

function EssayInput({ q, value, onChange }) {
  const v = value || "";
  const words = v.trim().split(/\s+/).filter(Boolean).length;
  return (
    <div>
      <textarea
        value={v}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Begin your response…"
        rows={10}
        style={{
          width: "100%",
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 6,
          padding: "16px 18px",
          color: "var(--text)",
          fontFamily: "var(--serif)", fontSize: 16, lineHeight: 1.65,
          outline: "none", resize: "vertical",
          minHeight: 260,
        }}
      />
      <div style={{ marginTop: 10, display: "flex", gap: 16, fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.5 }}>
        <span style={{ color: words >= q.minWords ? "var(--accent)" : "var(--muted)" }}>
          {words} / {q.minWords} min words
        </span>
        <span>· autosaved 6 s ago</span>
        <span>· rubric: clarity · evidence · comparison</span>
      </div>
    </div>
  );
}

/* ---------- Setup / configurator ---------- */

const CATEGORIES = [
  { id: "algorithms",  label: "Algorithms",     count: 142, color: "var(--accent)" },
  { id: "math",        label: "Mathematics",    count: 218, color: "var(--blue)" },
  { id: "economics",   label: "Economics",      count: 96,  color: "var(--blue)" },
  { id: "chemistry",   label: "Chemistry",      count: 184, color: "var(--amber)" },
  { id: "biology",     label: "Biology",        count: 120, color: "var(--accent)" },
  { id: "politics",    label: "Political sci.", count: 64,  color: "var(--blue)" },
];

const TAG_POOL = [
  "graphs", "complexity", "vectors", "matrices", "eigenvalues",
  "policy", "inflation", "fiscal", "stereo", "isomers",
  "cells", "membranes", "theory", "essay", "code",
  "midterm-prep", "exam-prep", "review", "proofs",
];

const QTYPES = [
  { id: "mc",    label: "Multiple choice" },
  { id: "tf",    label: "True / false" },
  { id: "short", label: "Short answer" },
  { id: "essay", label: "Essay" },
  { id: "code",  label: "Code" },
];

const DIFFICULTIES = [
  { id: "intro", label: "Introductory", pct: 25 },
  { id: "inter", label: "Intermediate", pct: 50 },
  { id: "adv",   label: "Advanced",     pct: 25 },
];

function QuizSetup({ onStart }) {
  const [cats, setCats]       = React.useState(["algorithms"]);
  const [tags, setTags]       = React.useState(["graphs", "complexity"]);
  const [types, setTypes]     = React.useState(["mc", "short", "code"]);
  const [diff, setDiff]       = React.useState("mixed");
  const [count, setCount]     = React.useState(12);
  const [duration, setDuration] = React.useState(25);
  const [mode, setMode]       = React.useState("practice"); // practice | timed | adaptive
  const [source, setSource]   = React.useState("quiz"); // quiz | bank
  const [shuffle, setShuffle] = React.useState(true);
  const [explain, setExplain] = React.useState(true);

  const toggle = (arr, setArr, id) =>
    setArr(arr.includes(id) ? arr.filter((x) => x !== id) : [...arr, id]);

  // Pool size estimate based on selected cats/tags/types
  const poolBase = cats.reduce((s, c) => s + (CATEGORIES.find((x) => x.id === c)?.count || 0), 0);
  const tagFactor = tags.length === 0 ? 1 : Math.max(0.15, Math.min(1, 0.15 + tags.length * 0.10));
  const typeFactor = types.length === 0 ? 0 : types.length / QTYPES.length;
  const pool = Math.max(0, Math.round(poolBase * tagFactor * typeFactor));
  const canStart = pool >= count && types.length > 0 && cats.length > 0;

  return (
    <div style={{ padding: "0 0 56px" }}>
      {/* Header */}
      <div style={{
        padding: "24px 40px",
        borderBottom: "1px solid var(--border)",
        background: "var(--surface)",
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-end", gap: 24 }}>
          <div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 6 }}>
              Practice session · configure
            </div>
            <h1 style={{ margin: 0, fontFamily: "var(--serif)", fontSize: 30, fontWeight: 500, letterSpacing: -0.4 }}>
              Compose your quiz
            </h1>
            <p style={{ color: "var(--text-2)", fontSize: 13.5, lineHeight: 1.55, margin: "8px 0 0", maxWidth: 620 }}>
              Pick categories, tags, and question types — Harus pulls a fresh mix from the question bank.
              Pre-built quizzes from your assignments are also available below.
            </p>
          </div>
          <Tag tone="ghost">{pool.toLocaleString()} items match</Tag>
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 340px", gap: 0 }}>
        {/* Left form */}
        <div style={{ padding: "28px 40px", borderRight: "1px solid var(--border)" }}>

          {/* Mode pills */}
          <SetupBlock label="Session mode" kicker="01">
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
              {[
                { id: "practice", label: "Practice", sub: "No clock · explanations as you go" },
                { id: "timed",    label: "Timed",    sub: "Clock + autosubmit · proctor-friendly" },
                { id: "adaptive", label: "Adaptive", sub: "IRT-driven · difficulty adjusts" },
              ].map((m) => {
                const sel = mode === m.id;
                return (
                  <button key={m.id} onClick={() => setMode(m.id)} style={{
                    padding: "14px 16px", textAlign: "left", cursor: "pointer",
                    background: sel ? "var(--accent-dim)" : "var(--surface)",
                    border: `1px solid ${sel ? "var(--accent-line)" : "var(--border)"}`,
                    borderRadius: 6, color: "var(--text)",
                  }}>
                    <div style={{ fontFamily: "var(--serif)", fontSize: 17, fontWeight: 500, color: sel ? "var(--accent)" : "var(--text)" }}>
                      {m.label}
                    </div>
                    <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 4, lineHeight: 1.5 }}>{m.sub}</div>
                  </button>
                );
              })}
            </div>
          </SetupBlock>

          {/* Categories */}
          <SetupBlock label="Categories" kicker="02" hint={`${cats.length} selected`}>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
              {CATEGORIES.map((c) => {
                const sel = cats.includes(c.id);
                return (
                  <button key={c.id} onClick={() => toggle(cats, setCats, c.id)} style={{
                    padding: "9px 14px",
                    background: sel ? "var(--accent-dim)" : "var(--surface)",
                    border: `1px solid ${sel ? "var(--accent-line)" : "var(--border)"}`,
                    color: sel ? "var(--text)" : "var(--text-2)",
                    borderRadius: 999,
                    fontSize: 13, fontWeight: sel ? 600 : 500,
                    cursor: "pointer",
                    display: "inline-flex", gap: 8, alignItems: "center",
                  }}>
                    <span style={{ width: 8, height: 8, borderRadius: "50%", background: c.color, opacity: sel ? 1 : 0.45 }} />
                    {c.label}
                    <span style={{ fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 0.4 }}>{c.count}</span>
                  </button>
                );
              })}
            </div>
          </SetupBlock>

          {/* Tags */}
          <SetupBlock label="Tags" kicker="03" hint={`${tags.length} selected · narrows the pool`}>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {TAG_POOL.map((t) => {
                const sel = tags.includes(t);
                return (
                  <button key={t} onClick={() => toggle(tags, setTags, t)} style={{
                    padding: "5px 10px",
                    background: sel ? "var(--accent)" : "transparent",
                    border: `1px solid ${sel ? "var(--accent)" : "var(--border)"}`,
                    color: sel ? "#0b1410" : "var(--text-2)",
                    fontFamily: "var(--mono)", fontSize: 11, letterSpacing: 0.3,
                    borderRadius: 4, cursor: "pointer",
                  }}>
                    {sel ? "✓ " : "+ "}{t}
                  </button>
                );
              })}
            </div>
          </SetupBlock>

          {/* Difficulty */}
          <SetupBlock label="Difficulty" kicker="04">
            <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8 }}>
              {[
                { id: "intro", label: "Introductory" },
                { id: "inter", label: "Intermediate" },
                { id: "adv",   label: "Advanced" },
                { id: "mixed", label: "Mixed (auto)" },
              ].map((d) => {
                const sel = diff === d.id;
                return (
                  <button key={d.id} onClick={() => setDiff(d.id)} style={{
                    padding: "11px 12px",
                    background: sel ? "var(--accent-dim)" : "var(--surface)",
                    border: `1px solid ${sel ? "var(--accent-line)" : "var(--border)"}`,
                    color: sel ? "var(--accent)" : "var(--text-2)",
                    borderRadius: 6, fontSize: 13, fontWeight: sel ? 600 : 500,
                    cursor: "pointer",
                  }}>{d.label}</button>
                );
              })}
            </div>
            {diff === "mixed" && (
              <div style={{ marginTop: 14, padding: 12, background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6 }}>
                <div style={{ display: "flex", height: 22, borderRadius: 4, overflow: "hidden", border: "1px solid var(--border)" }}>
                  {DIFFICULTIES.map((d, i) => {
                    const colors = ["var(--accent)", "var(--blue)", "var(--amber)"];
                    return (
                      <div key={d.id} style={{
                        flex: d.pct,
                        background: colors[i], opacity: 0.85,
                        display: "flex", alignItems: "center", justifyContent: "center",
                        fontFamily: "var(--mono)", fontSize: 10.5, color: "#0b1410", letterSpacing: 0.6, fontWeight: 600,
                        borderRight: i < 2 ? "1px solid var(--bg)" : "none",
                      }}>{d.pct}%</div>
                    );
                  })}
                </div>
                <div style={{ marginTop: 8, fontSize: 11.5, color: "var(--muted)", fontFamily: "var(--mono)", letterSpacing: 0.3, display: "flex", justifyContent: "space-between" }}>
                  <span>Introductory · Intermediate · Advanced</span>
                  <span>auto-balanced from your mastery map</span>
                </div>
              </div>
            )}
          </SetupBlock>

          {/* Types */}
          <SetupBlock label="Question types" kicker="05">
            <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 8 }}>
              {QTYPES.map((q) => {
                const sel = types.includes(q.id);
                return (
                  <button key={q.id} onClick={() => toggle(types, setTypes, q.id)} style={{
                    padding: "12px 8px",
                    background: sel ? "var(--accent-dim)" : "var(--surface)",
                    border: `1px solid ${sel ? "var(--accent-line)" : "var(--border)"}`,
                    color: sel ? "var(--text)" : "var(--text-2)",
                    borderRadius: 6, fontSize: 12.5,
                    cursor: "pointer", fontWeight: sel ? 600 : 500,
                    display: "flex", alignItems: "center", justifyContent: "center", gap: 6,
                  }}>
                    {sel ? <Icon name="check" size={12} color="var(--accent)" /> : null}
                    {q.label}
                  </button>
                );
              })}
            </div>
          </SetupBlock>

          {/* Count + duration */}
          <SetupBlock label="Length" kicker="06">
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 18 }}>
              <NumberStep label="Questions" value={count} onChange={setCount} min={5} max={50} step={1} />
              <NumberStep label="Time limit" value={duration} onChange={setDuration} min={5} max={120} step={5} unit="min" disabled={mode === "practice"} />
            </div>
          </SetupBlock>

          {/* Source */}
          <SetupBlock label="Source" kicker="07">
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
              {[
                { id: "bank", label: "Item bank", sub: `Fresh selection from ${pool.toLocaleString()} items` },
                { id: "quiz", label: "Specific quiz",  sub: "Pull from an existing quiz in your library" },
              ].map((s) => {
                const sel = source === s.id;
                return (
                  <button key={s.id} onClick={() => setSource(s.id)} style={{
                    padding: "14px 16px", textAlign: "left",
                    background: sel ? "var(--accent-dim)" : "var(--surface)",
                    border: `1px solid ${sel ? "var(--accent-line)" : "var(--border)"}`,
                    color: "var(--text)", borderRadius: 6, cursor: "pointer",
                  }}>
                    <div style={{ fontWeight: 500, fontSize: 14, color: sel ? "var(--accent)" : "var(--text)" }}>{s.label}</div>
                    <div style={{ fontSize: 11.5, color: "var(--muted)", marginTop: 4 }}>{s.sub}</div>
                  </button>
                );
              })}
            </div>

            {source === "quiz" && (
              <div style={{ marginTop: 12, border: "1px solid var(--border)", borderRadius: 6, background: "var(--surface)" }}>
                {QUIZZES.slice(0, 3).map((q, i, arr) => (
                  <div key={q.id} style={{
                    display: "grid", gridTemplateColumns: "1fr auto auto",
                    padding: "10px 14px", gap: 12, alignItems: "center",
                    borderBottom: i < arr.length - 1 ? "1px solid var(--border)" : "none",
                  }}>
                    <div>
                      <div style={{ fontSize: 13, color: "var(--text)" }}>{q.title}</div>
                      <div style={{ fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 0.5, marginTop: 2 }}>{q.course} · {q.questions} Qs · {q.difficulty}</div>
                    </div>
                    <Tag>{q.questions} Qs</Tag>
                    <button style={{ background: "transparent", border: "none", color: "var(--accent)", fontWeight: 600, fontSize: 11, cursor: "pointer", letterSpacing: 0.5 }}>Pick</button>
                  </div>
                ))}
              </div>
            )}
          </SetupBlock>

          {/* Options */}
          <SetupBlock label="Options" kicker="08">
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <ToggleRow label="Shuffle question order"           sub="Different sequence each attempt" value={shuffle} onChange={setShuffle} />
              <ToggleRow label="Show explanations after each item" sub="Recommended for practice mode"  value={explain} onChange={setExplain} />
              <ToggleRow label="Block back-navigation"             sub="Once answered, stays locked"     value={mode === "timed"} onChange={() => {}} dim />
            </div>
          </SetupBlock>
        </div>

        {/* Right summary rail */}
        <aside style={{ padding: "28px 28px", background: "var(--surface)" }}>
          <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 10 }}>
            Session summary
          </div>

          <div style={{
            border: "1px solid var(--border)", borderRadius: 6,
            background: "var(--bg)",
            padding: "18px 20px",
          }}>
            <div style={{ fontFamily: "var(--serif)", fontSize: 22, fontWeight: 500, letterSpacing: -0.3 }}>
              Custom — {cats.length === 1 ? CATEGORIES.find((c) => c.id === cats[0])?.label : `${cats.length} subjects`}
            </div>
            <div style={{ fontSize: 12, color: "var(--muted)", marginTop: 4 }}>
              {mode === "practice" ? "Practice · untimed"  : mode === "timed" ? `Timed · ${duration} min` : "Adaptive · IRT"}
            </div>

            <Divider style={{ margin: "16px 0" }} />

            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <KV k="Questions"   v={count} mono />
              <KV k="Pool"        v={pool.toLocaleString()} mono />
              <KV k="Difficulty"  v={diff === "mixed" ? "25 / 50 / 25" : { intro: "Introductory", inter: "Intermediate", adv: "Advanced" }[diff]} mono />
              <KV k="Types"       v={types.length ? types.join(", ") : "—"} mono />
              <KV k="Duration"    v={mode === "practice" ? "untimed" : `${duration} min`} mono />
            </div>

            <Divider style={{ margin: "16px 0" }} />

            <div style={{ fontSize: 12, color: "var(--text-2)", lineHeight: 1.55 }}>
              Estimated coverage of your <span style={{ color: "var(--accent)" }}>graph traversal</span> goals:
              <span style={{ fontFamily: "var(--mono)", color: "var(--text)", marginLeft: 6 }}>74%</span>
            </div>
          </div>

          <div style={{ marginTop: 16, display: "flex", flexDirection: "column", gap: 8 }}>
            <Button variant="primary" size="lg" disabled={!canStart} onClick={() => onStart({
              cats, tags, types, diff, count, duration, mode, shuffle, explain,
            })} style={{ width: "100%", justifyContent: "center" }} icon={<Icon name="arrow" size={14} color="#0b1410" />}>
              {canStart ? `Start ${mode} session` : pool < count ? "Not enough items" : "Pick at least one type"}
            </Button>
            <Button variant="ghost" size="lg" style={{ width: "100%", justifyContent: "center" }}>Save as preset</Button>
          </div>

          <div style={{ marginTop: 20, padding: 12, background: "var(--surface-2)", border: "1px solid var(--border)", borderRadius: 6 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              <Icon name="sparkle" size={12} color="var(--accent)" />
              <span style={{ fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 1, textTransform: "uppercase" }}>Agent-equivalent</span>
            </div>
            <div style={{ fontFamily: "var(--mono)", fontSize: 11.5, color: "var(--text-2)", lineHeight: 1.6, letterSpacing: 0.2, wordBreak: "break-word" }}>
              POST /v1/sessions<br />
              {`{ "cats": [${cats.map((c) => `"${c}"`).join(", ")}],`}<br />
              &nbsp;&nbsp;{`"tags": [${tags.slice(0,3).map((t) => `"${t}"`).join(", ")}${tags.length>3 ? ", …" : ""}],`}<br />
              &nbsp;&nbsp;{`"types": [${types.map((t) => `"${t}"`).join(", ")}],`}<br />
              &nbsp;&nbsp;{`"diff": "${diff}", "count": ${count} }`}
            </div>
          </div>

          <div style={{ marginTop: 18 }}>
            <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
              Recent presets
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {[
                ["Graph traversal cram", "12 Qs · 25m · timed"],
                ["Linear algebra warmup", "8 Qs · untimed · practice"],
                ["Pre-midterm review",    "20 Qs · 40m · adaptive"],
              ].map(([n, s]) => (
                <button key={n} style={{
                  background: "transparent", border: "1px solid var(--border)",
                  padding: "8px 10px", borderRadius: 4,
                  textAlign: "left", cursor: "pointer", color: "var(--text-2)",
                }}>
                  <div style={{ fontSize: 12.5, color: "var(--text)" }}>{n}</div>
                  <div style={{ fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--muted)", letterSpacing: 0.3, marginTop: 2 }}>{s}</div>
                </button>
              ))}
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}

function SetupBlock({ label, kicker, hint, children }) {
  return (
    <div style={{ marginBottom: 26 }}>
      <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", marginBottom: 10 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 10 }}>
          {kicker ? <span style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--muted)", letterSpacing: 1 }}>{kicker}</span> : null}
          <span style={{ fontFamily: "var(--serif)", fontSize: 16, fontWeight: 500, color: "var(--text)" }}>{label}</span>
        </div>
        {hint ? <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--muted)", letterSpacing: 0.3 }}>{hint}</span> : null}
      </div>
      {children}
    </div>
  );
}

function NumberStep({ label, value, onChange, min, max, step = 1, unit, disabled }) {
  const dec = () => onChange(Math.max(min, value - step));
  const inc = () => onChange(Math.min(max, value + step));
  return (
    <div style={{ opacity: disabled ? 0.45 : 1 }}>
      <div style={{ fontFamily: "var(--mono)", fontSize: 10, letterSpacing: 1.3, textTransform: "uppercase", color: "var(--muted)", marginBottom: 8 }}>
        {label}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <button onClick={dec} disabled={disabled} style={stepBtn}>−</button>
        <div style={{
          flex: 1, padding: "10px 14px",
          background: "var(--surface)", border: "1px solid var(--border)",
          borderRadius: 6, textAlign: "center",
          fontFamily: "var(--serif)", fontSize: 20, fontWeight: 500,
          color: "var(--text)",
        }}>{value}<span style={{ color: "var(--muted)", fontSize: 13, marginLeft: 4 }}>{unit || ""}</span></div>
        <button onClick={inc} disabled={disabled} style={stepBtn}>+</button>
      </div>
    </div>
  );
}

const stepBtn = {
  width: 42, height: 42,
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 6,
  color: "var(--text)",
  fontFamily: "var(--mono)", fontSize: 18, fontWeight: 600,
  cursor: "pointer",
};

function ToggleRow({ label, sub, value, onChange, dim }) {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "10px 0", borderBottom: "1px dashed var(--border)", opacity: dim ? 0.5 : 1 }}>
      <div>
        <div style={{ fontSize: 13, color: "var(--text)" }}>{label}</div>
        <div style={{ fontSize: 11.5, color: "var(--muted)", marginTop: 2 }}>{sub}</div>
      </div>
      <button onClick={() => onChange(!value)} style={{
        width: 36, height: 20, padding: 0,
        background: value ? "var(--accent)" : "var(--surface-2)",
        border: `1px solid ${value ? "var(--accent)" : "var(--border)"}`,
        borderRadius: 999, cursor: "pointer",
        position: "relative",
      }}>
        <span style={{
          position: "absolute", top: 1, left: value ? 17 : 1,
          width: 16, height: 16, borderRadius: "50%",
          background: value ? "#0b1410" : "var(--text-2)",
          transition: "left 140ms",
        }} />
      </button>
    </div>
  );
}

Object.assign(window, { QuizScreen });