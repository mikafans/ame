"use client";

import { useState } from "react";
import { api } from "@/api/client";
import { responseErrorMessage } from "@/api/errors";
import { Button } from "@/components/ui/button";

export function PortabilityPanel({
  journeys,
}: {
  journeys: { id: string; promise: string }[];
}) {
  const [journeyId, setJourneyId] = useState(journeys[0]?.id ?? "");
  const [artifact, setArtifact] = useState("");
  const [message, setMessage] = useState("");

  async function exportJourney() {
    const result = await api.GET("/api/v1/learning/journeys/{id}/export", {
      params: { path: { id: journeyId } },
    });
    if (!result.response.ok || !result.data) {
      setMessage(
        responseErrorMessage(
          result.response,
          result.error,
          "Could not export this journey.",
        ),
      );
      return;
    }
    const serialized = JSON.stringify(result.data, null, 2);
    setArtifact(serialized);
    setMessage("Portable artifact is ready to inspect.");
    const blob = new Blob([serialized], { type: "application/json" });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `ame-journey-${journeyId}.json`;
    link.click();
    URL.revokeObjectURL(link.href);
  }

  async function importJourney() {
    let manifest: unknown;
    try {
      manifest = JSON.parse(artifact);
    } catch {
      setMessage("The artifact is not valid JSON.");
      return;
    }
    const result = await api.POST("/api/v1/learning/imports", {
      body: manifest as never,
    });
    if (!result.response.ok || !result.data) {
      setMessage(
        responseErrorMessage(
          result.response,
          result.error,
          "Import rejected: check owner, checksum, schema, and conflicts.",
        ),
      );
      return;
    }
    setMessage(
      result.data.alreadyImported
        ? "Already imported; no learner state was duplicated."
        : "Journey and learner history imported.",
    );
  }

  return (
    <section
      className="rounded-3xl border border-border bg-card p-7"
      data-testid="portability"
    >
      <p className="font-mono text-xs uppercase tracking-[0.14em] text-primary">
        Portable learning
      </p>
      <h2 className="mt-2 text-2xl font-bold">Export or restore a journey</h2>
      <select
        aria-label="Journey to export"
        className="mt-4 h-11 w-full rounded-xl border border-input bg-background px-3"
        onChange={(event) => setJourneyId(event.target.value)}
        value={journeyId}
      >
        {journeys.map((journey) => (
          <option key={journey.id} value={journey.id}>
            {journey.promise}
          </option>
        ))}
      </select>
      <div className="mt-3 flex gap-2">
        <Button disabled={!journeyId} onClick={() => void exportJourney()}>
          Export JSON
        </Button>
        <Button
          disabled={!artifact.trim()}
          onClick={() => void importJourney()}
          variant="outline"
        >
          Import JSON
        </Button>
      </div>
      <textarea
        aria-label="Journey portability artifact"
        className="mt-4 min-h-52 w-full rounded-xl border border-input bg-background p-3 font-mono text-xs"
        onChange={(event) => setArtifact(event.target.value)}
        placeholder="Exported manifest appears here, or paste one to import."
        value={artifact}
      />
      {message && (
        <p className="mt-2 text-sm text-muted-foreground">{message}</p>
      )}
    </section>
  );
}
