"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { ArrowRight, Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Logo } from "@/components/Logo";
import { useColorMode } from "@/components/ThemeRegistry";
import { useAuth } from "@/hooks/useAuth";

type TabId = "signup" | "login";

function authDestination() {
  const returnTo = new URLSearchParams(window.location.search).get("returnTo");
  return returnTo && returnTo.startsWith("/") && !returnTo.startsWith("//")
    ? returnTo
    : "/learning";
}

export default function LoginPage() {
  const router = useRouter();
  const { user, refresh } = useAuth();
  const { mode, toggle } = useColorMode();
  const [tab, setTab] = useState<TabId>("login");
  const [fullName, setFullName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (user) router.push(authDestination());
  }, [user, router]);

  useEffect(() => {
    if (new URLSearchParams(window.location.search).get("tab") === "signup") {
      setTab("signup");
    }
  }, []);

  const apiUrl =
    process.env.NEXT_PUBLIC_API_URL ??
    (typeof window !== "undefined"
      ? `http://${window.location.hostname}:28080`
      : "http://localhost:28080");
  const publicApiUrl = `${apiUrl.replace(/\/$/, "")}/public`;

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const endpoint =
        tab === "signup" ? "/v1/auth/register" : "/v1/auth/login";
      const body =
        tab === "signup"
          ? { email, name: fullName, password, role: "user" }
          : { email, password };
      const response = await fetch(`${publicApiUrl}${endpoint}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        let message = "Authentication failed";
        try {
          const data = await response.json();
          message =
            data?.error?.message ||
            data?.message ||
            (typeof data?.error === "string" ? data.error : null) ||
            `Error: ${response.statusText}`;
        } catch {
          message = `Authentication failed: ${response.status} ${response.statusText}`;
        }
        setError(message);
        return;
      }
      await refresh();
      router.push(authDestination());
    } catch (submitError) {
      setError(
        submitError instanceof Error
          ? submitError.message
          : "Could not reach the server",
      );
    } finally {
      setLoading(false);
    }
  }

  const isDark = mode === "dark";

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <aside className="hidden w-1/2 flex-col border-r border-border bg-muted/30 p-12 md:flex dark:bg-card/30">
        <div className="flex items-center justify-between">
          <Logo size={30} />
          <button
            type="button"
            aria-label={isDark ? "Use light mode" : "Use dark mode"}
            onClick={toggle}
            className="inline-flex size-8 items-center justify-center rounded-lg text-muted-foreground outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
          >
            {isDark ? <Sun className="size-4" /> : <Moon className="size-4" />}
          </button>
        </div>

        <div className="mt-16 max-w-lg">
          <h1 className="text-3xl font-semibold leading-tight tracking-tight">
            One clear next step for what you want to learn.
          </h1>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            Tell ame your intent with an email identifier. A first journey is
            created for you, and an agent or the web app can keep it moving.
          </p>
        </div>

        <div className="mt-auto rounded-xl border border-dashed border-primary/40 bg-primary/10 p-4">
          <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.15em] text-primary">
            For agents &amp; integrations
          </p>
          <pre className="whitespace-pre-wrap break-words font-mono text-xs leading-6 text-primary">{`POST /public/v1/onboarding/start
{"email":"learner@example.com","displayName":"Learner","prompt":"I'd like to learn a new subject","idempotencyKey":"first-journey"}`}</pre>
          <p className="mt-2 text-xs leading-5 text-muted-foreground">
            Returns the learner bearer token and the first journey. No agent
            account or scope setup is required.
          </p>
        </div>
        <p className="mt-5 text-[10px] tracking-wide text-muted-foreground">
          OpenAPI 3.1 · Agents · FERPA · GDPR
        </p>
      </aside>

      <main className="flex flex-1 items-center justify-center px-6 py-10 sm:px-12">
        <div className="w-full max-w-sm">
          <div
            role="tablist"
            aria-label="Authentication mode"
            className="mb-8 flex border-b border-border"
          >
            {(["signup", "login"] as const).map((tabId) => (
              <button
                key={tabId}
                type="button"
                role="tab"
                aria-selected={tab === tabId}
                onClick={() => {
                  setTab(tabId);
                  setError(null);
                }}
                className={`flex-1 border-b-2 px-3 py-2.5 text-sm font-medium outline-none transition focus-visible:ring-2 focus-visible:ring-ring ${tab === tabId ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground"}`}
              >
                {tabId === "signup" ? "Create account" : "Sign in"}
              </button>
            ))}
          </div>

          <form
            id="auth-form"
            name="login"
            method="post"
            action="/login"
            autoComplete="on"
            className="flex flex-col gap-4"
            onSubmit={handleSubmit}
          >
            {tab === "signup" && (
              <label
                className="grid gap-1.5 text-sm font-medium"
                htmlFor="full-name"
              >
                Full name
                <input
                  id="full-name"
                  placeholder="Ada Lovelace"
                  value={fullName}
                  onChange={(event) => setFullName(event.target.value)}
                  autoComplete="name"
                  className="h-10 rounded-lg border border-input bg-background px-3 font-normal outline-none transition placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/20"
                />
              </label>
            )}
            <label className="grid gap-1.5 text-sm font-medium" htmlFor="email">
              Email
              <input
                id="email"
                name="email"
                type="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                autoComplete={tab === "signup" ? "email" : "username"}
                placeholder="name@example.com"
                spellCheck={false}
                required
                className="h-10 rounded-lg border border-input bg-background px-3 font-normal outline-none transition placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/20"
              />
            </label>
            <label
              className="grid gap-1.5 text-sm font-medium"
              htmlFor="password"
            >
              Password
              <input
                id="password"
                name="password"
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                autoComplete={
                  tab === "signup" ? "new-password" : "current-password"
                }
                placeholder={
                  tab === "signup" ? "Create a password" : "Enter your password"
                }
                required
                className="h-10 rounded-lg border border-input bg-background px-3 font-normal outline-none transition placeholder:text-muted-foreground focus:border-ring focus:ring-3 focus:ring-ring/20"
              />
            </label>

            {error && (
              <div
                role="alert"
                className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
              >
                {error}
              </div>
            )}

            <Button
              type="submit"
              disabled={loading}
              size="lg"
              className="mt-1 w-full"
            >
              {loading
                ? "Loading…"
                : tab === "signup"
                  ? "Create account"
                  : "Sign in"}
              {!loading && <ArrowRight className="size-4" />}
            </Button>
          </form>

          <div className="mt-8 rounded-xl border border-dashed border-border p-4">
            <p className="mb-1 text-xs font-semibold">Using an agent?</p>
            <p className="text-xs leading-5 text-muted-foreground">
              Give it the learner token returned by onboarding. It can use the
              same journey endpoints as this web app.
            </p>
          </div>
        </div>
      </main>
    </div>
  );
}
