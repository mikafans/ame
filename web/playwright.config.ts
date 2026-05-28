import { defineConfig, devices } from "@playwright/test";

const BASE_URL = process.env.E2E_BASE_URL ?? "http://localhost:23000";
const WEB_PORT = new URL(BASE_URL).port || "23000";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "list",
  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: `bun run dev -- -p ${WEB_PORT} >> ../.tmp/ame-web-e2e.log 2>&1`,
    url: BASE_URL,
    reuseExistingServer: !process.env.CI,
    timeout: 90_000,
  },
});
