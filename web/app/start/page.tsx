"use client";

import { Suspense, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { ArrowRight } from "lucide-react";
import { publicApi } from "@/api/client";
import { responseErrorMessage } from "@/api/errors";
import type { components } from "@/api/generated/schema.d.ts";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/hooks/useAuth";

type NativeJourney = components["schemas"]["NativeJourneyCatalogResponse"];

function StartLearningForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { refresh, user, loading: authLoading } = useAuth();
  const routePrompt = searchParams.get("prompt") ?? "";
  const routeCatalogId = searchParams.get("catalogId") ?? "";
  const [prompt, setPrompt] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [onboardingComplete, setOnboardingComplete] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [catalogEntry, setCatalogEntry] = useState<NativeJourney | null>(null);

  useEffect(() => {
    if (routePrompt) {
      setPrompt(routePrompt);
    }
  }, [routePrompt]);

  // The catalogId query param alone carries no learner-facing content — fetch
  // the matching reviewed path so the form shows what was actually picked
  // instead of a blank "what would you like to learn?" box.
  useEffect(() => {
    if (!routeCatalogId) {
      setCatalogEntry(null);
      return;
    }
    let cancelled = false;
    void publicApi
      .GET("/public/v1/catalog/journeys")
      .then((result) => {
        if (cancelled || !result.response.ok || !result.data) return;
        const entry = result.data.find(
          (journey) => journey.id === routeCatalogId,
        );
        if (!entry) return;
        setCatalogEntry(entry);
        setPrompt((current) => (current ? current : entry.description));
      })
      .catch(() => {
        // Non-fatal: the form still works with just catalogId at submit time.
      });
    return () => {
      cancelled = true;
    };
  }, [routeCatalogId]);

  // A signed-in visitor with no specific intent (no catalogId/prompt) has
  // nothing to do on this new-account form — send them to the agent handoff
  // page instead. But a signed-in learner picking a reviewed catalog path (or
  // arriving with a prompt) is starting an additional journey on their
  // existing account, so let them through to submit it.
  const hasIntent = Boolean(routeCatalogId || routePrompt);
  useEffect(() => {
    if (user && !hasIntent && !onboardingComplete && !loading) {
      router.replace("/agent");
    }
  }, [hasIntent, loading, onboardingComplete, router, user]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setError(null);
    try {
      let onboardingToken: string | undefined;
      if (!user) {
        const registration = await publicApi.POST("/public/v1/auth/register", {
          body: { email, name: displayName, password },
        });
        if (!registration.response.ok || !registration.data) {
          throw new Error(
            responseErrorMessage(
              registration.response,
              registration.error,
              "Could not create your learner account",
            ),
          );
        }
        onboardingToken = registration.data.token;
      }
      const startResult = await publicApi.POST("/public/v1/onboarding/start", {
        headers: onboardingToken
          ? { Authorization: `Bearer ${onboardingToken}` }
          : undefined,
        body: {
          displayName: user ? user.displayName : displayName,
          email: user ? (user.email ?? "") : email,
          idempotencyKey: crypto.randomUUID(),
          prompt,
          ...(routeCatalogId ? { catalogId: routeCatalogId } : {}),
        },
      });
      if (!startResult.response.ok || !startResult.data) {
        throw new Error(
          responseErrorMessage(
            startResult.response,
            startResult.error,
            "Could not start your learning journey",
          ),
        );
      }
      setOnboardingComplete(true);
      await refresh();
      router.push(`/learning/journeys/${startResult.data.journeyId}`);
    } catch (submitError) {
      setError(
        submitError instanceof Error
          ? submitError.message
          : "Could not reach AME",
      );
    } finally {
      setLoading(false);
    }
  }

  if (authLoading || (user && !hasIntent)) {
    return (
      <main className="w-full max-w-xl rounded-2xl border border-border bg-card p-6 shadow-sm sm:p-10">
        <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
          Agent handoff
        </p>
        <h1 className="mt-3 text-2xl font-bold tracking-tight">
          Opening your agent handoff…
        </h1>
        <p className="mt-3 text-muted-foreground">
          Your account is ready. AME will take you to the place where you can
          create a limited, revocable course-authoring handoff for your agent.
        </p>
      </main>
    );
  }

  return (
    <main className="w-full max-w-xl rounded-2xl border border-border bg-card p-6 shadow-sm sm:p-10">
      <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
        Start your journey
      </p>
      <h1 className="mt-3 text-3xl font-bold tracking-tight">
        {catalogEntry
          ? catalogEntry.title
          : "Turn your intent into a first useful session."}
      </h1>
      <p className="mt-3 leading-6 text-muted-foreground">
        {catalogEntry
          ? catalogEntry.description
          : "AME creates a local learner account and a resumable journey. No email delivery or external verification is required for this self-host flow."}
      </p>
      {catalogEntry && (
        <p className="mt-3 text-xs text-muted-foreground">
          Reviewed starting path · about {catalogEntry.estimatedMinutes} minutes
          · {catalogEntry.level}
        </p>
      )}

      <form onSubmit={handleSubmit} className="mt-8 space-y-5">
        <div>
          <label
            htmlFor="start-prompt"
            className="mb-2 block text-sm font-medium"
          >
            {catalogEntry
              ? "What would you like to learn? (from your selected path — edit if you'd like)"
              : "What would you like to learn?"}
          </label>
          <textarea
            id="start-prompt"
            required={!routeCatalogId}
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            rows={3}
            className="w-full resize-none rounded-xl border border-input bg-background px-4 py-3 outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
        {user ? (
          <p className="text-sm text-muted-foreground">
            Continuing as{" "}
            <span className="font-medium text-foreground">
              {user.displayName}
            </span>
            . This adds another journey to your existing account.
          </p>
        ) : (
          <div className="grid gap-5 sm:grid-cols-2">
            <div>
              <label
                htmlFor="start-name"
                className="mb-2 block text-sm font-medium"
              >
                Your name
              </label>
              <input
                id="start-name"
                required
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
                className="h-11 w-full rounded-xl border border-input bg-background px-4 outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
            <div>
              <label
                htmlFor="start-email"
                className="mb-2 block text-sm font-medium"
              >
                Email identifier
              </label>
              <input
                id="start-email"
                required
                type="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                className="h-11 w-full rounded-xl border border-input bg-background px-4 outline-none focus:ring-2 focus:ring-ring"
              />
            </div>
          </div>
        )}
        {!user && (
          <div>
            <label
              htmlFor="start-password"
              className="mb-2 block text-sm font-medium"
            >
              Password
            </label>
            <input
              id="start-password"
              required
              minLength={8}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              className="h-11 w-full rounded-xl border border-input bg-background px-4 outline-none focus:ring-2 focus:ring-ring"
            />
            <p className="mt-2 text-xs text-muted-foreground">
              Stored locally for signing in again. AME does not send email.
            </p>
          </div>
        )}
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}
        <Button
          type="submit"
          disabled={loading}
          className="w-full rounded-full"
        >
          {loading ? "Creating your journey…" : "Create my journey"}
          <ArrowRight className="size-4" />
        </Button>
      </form>
    </main>
  );
}

export default function StartPage() {
  return (
    <div className="min-h-screen bg-background px-4 py-6 text-foreground sm:px-6 sm:py-10">
      <div className="mx-auto max-w-6xl">
        <Logo size={32} />
        <div className="flex justify-center py-12 sm:py-20">
          <Suspense
            fallback={<div className="text-muted-foreground">Loading…</div>}
          >
            <StartLearningForm />
          </Suspense>
        </div>
      </div>
    </div>
  );
}
