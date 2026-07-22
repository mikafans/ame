import { describe, it, expect } from "bun:test";
import React from "react";
import { renderToString } from "react-dom/server";
import { ActivityContentRenderer } from "./ActivityContentRenderer";

describe("ActivityContentRenderer", () => {
  it("renders rich text and code capabilities", () => {
    const html = renderToString(
      <ActivityContentRenderer
        taskId="task-1"
        contentVersion={2}
        content={{
          type: "rich_text",
          heading: "Checkpointing",
          body: "Durable state protects progress.",
        }}
      />,
    );
    expect(html).toContain("Checkpointing");
    expect(html).toContain("Durable state protects progress.");

    const code = renderToString(
      <ActivityContentRenderer
        content={{
          type: "code_example",
          title: "Run a job",
          language: "java",
          code: "env.execute();",
          explanation: "The runtime starts the pipeline.",
        }}
      />,
    );
    expect(code).toContain("Run a job");
    expect(code).toContain("env.execute();");
  });

  it("renders scenario choices and a safe unknown fallback", () => {
    const scenario = renderToString(
      <ActivityContentRenderer
        content={{
          type: "scenario",
          context: "A checkpoint fails.",
          prompt: "What should you inspect?",
          options: [
            { id: "logs", label: "Inspect logs" },
            { id: "retry", label: "Retry immediately" },
          ],
        }}
      />,
    );
    expect(scenario).toContain("Inspect logs");
    expect(scenario).toContain("Retry immediately");

    const unknown = renderToString(
      <ActivityContentRenderer
        content={{ type: "interactive_lab", title: "Run a cluster" }}
      />,
    );
    expect(unknown).toContain("Activity capability unavailable");
    expect(unknown).toContain("interactive_lab");
  });
});
