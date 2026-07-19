"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { ArrowRight } from "lucide-react";
import { api } from "@/api/client";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";

function StartLearningForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [prompt, setPrompt] = useState(searchParams.get("prompt") ?? "");
  const [displayName, setDisplayName] = useState("");
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const { data, response } = await api.POST("/v1/onboarding/start", {
        body: {
          displayName,
          email,
          idempotencyKey: crypto.randomUUID(),
          prompt,
        },
      });
      if (!response.ok || !data) {
        throw new Error("Could not start your learning journey");
      }
      router.push(`/learning/journeys/${data.journeyId}`);
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

  return (
    <main className="w-full max-w-xl rounded-2xl border border-border bg-card p-6 shadow-sm sm:p-10">
      <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
        Start your journey
      </p>
      <h1 className="mt-3 text-3xl font-bold tracking-tight">
        Turn your intent into a first useful session.
      </h1>
      <p className="mt-3 leading-6 text-muted-foreground">
        AME creates a local learner account and a resumable journey. No email
        delivery or external verification is required for this self-host flow.
      </p>

      <form onSubmit={handleSubmit} className="mt-8 space-y-5">
        <div>
          <label
            htmlFor="start-prompt"
            className="mb-2 block text-sm font-medium"
          >
            What would you like to learn?
          </label>
          <textarea
            id="start-prompt"
            required
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            rows={3}
            className="w-full resize-none rounded-xl border border-input bg-background px-4 py-3 outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
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
