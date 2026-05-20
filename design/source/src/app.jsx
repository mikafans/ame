function App() {
  const [route, setRoute] = React.useState("library");
  const [showSignup, setShowSignup] = React.useState(true);
  const [t, setTweak] = useTweaks(window.TWEAK_DEFAULTS);

  // Apply theme to root
  React.useEffect(() => {
    document.documentElement.setAttribute("data-theme", t.theme || "slate");
  }, [t.theme]);

  // Route helpers
  const go = (r) => setRoute(r);

  if (showSignup) {
    return (
      <>
        <SignupScreen onEnter={() => setShowSignup(false)} />
        <TweaksUI t={t} setTweak={setTweak} />
      </>
    );
  }

  return (
    <div style={{ display: "flex", minHeight: "100vh", background: "var(--bg)" }} data-screen-label={"Harus · " + route}>
      <Sidebar route={route} setRoute={setRoute} showAgent={t.showAgentPanel !== false} />
      <main style={{ flex: 1, minWidth: 0 }}>
        {route === "library"   && (
          <>
            <Topbar title="Library" subtitle="Browse, filter, and start any assigned quiz." breadcrumb={"Spring 2026 · 4 assigned"} />
            <LibraryScreen onOpen={go} />
          </>
        )}
        {route === "exams" && (
          <>
            <Topbar title="Exams" subtitle="Composed assessments — bundled quizzes with weighted sections." breadcrumb={`Spring 2026 · ${EXAMS.length} exams`} />
            <ExamsScreen onStartQuiz={() => go("quiz")} />
          </>
        )}
        {route === "quiz" && (
          <QuizScreen onSubmit={() => go("results")} />
        )}
        {route === "results" && (
          <>
            <Topbar title="Results" subtitle="Review your last attempt and the cohort comparison." breadcrumb="Algorithms · attempt 1 of 2" />
            <ResultsScreen onContinue={() => go("library")} onReview={() => go("dashboard")} />
          </>
        )}
        {route === "dashboard" && (
          <>
            <Topbar title="Progress" subtitle="Trends, cohort comparison, and item analysis." breadcrumb="All courses" />
            <DashboardScreen statsDepth={t.statsDepth || "full"} />
          </>
        )}
        {route === "author" && (
          <>
            <Topbar title="Author studio" subtitle="Compose, generate, and distribute assessments." breadcrumb="Drafts · 3" />
            <AuthorScreen />
          </>
        )}
        {route === "agent" && (
          <>
            <Topbar title="Agent integration" subtitle="API keys, MCP tool descriptors, and import demo." breadcrumb="Connected · GraderBot · TA-Assist" />
            <AgentScreen />
          </>
        )}
      </main>

      <TweaksUI t={t} setTweak={setTweak} />
    </div>
  );
}

function TweaksUI({ t, setTweak }) {
  return (
    <TweaksPanel>
      <TweakSection label="Theme">
        <TweakRadio
          label="Color"
          value={t.theme}
          onChange={(v) => setTweak("theme", v)}
          options={["slate", "paper", "cobalt"]}
        />
      </TweakSection>

      <TweakSection label="Data">
        <TweakRadio
          label="Stats depth"
          value={t.statsDepth}
          onChange={(v) => setTweak("statsDepth", v)}
          options={["minimal", "standard", "full"]}
        />
        <TweakToggle
          label="Agent panel"
          value={t.showAgentPanel}
          onChange={(v) => setTweak("showAgentPanel", v)}
        />
      </TweakSection>
    </TweaksPanel>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(
  <ShareProvider>
    <App />
  </ShareProvider>
);
