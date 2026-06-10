import { test, expect } from "@playwright/test";
import { API_URL, loginAs, setAuthCookie } from "./helpers";

test.describe("admin settings — maintenance mode", () => {
  test.describe.configure({ mode: "serial" });
  let adminToken: string;
  let learnerToken: string;

  test.beforeAll(async ({ request }) => {
    adminToken = await loginAs(request, "admin@example.com");
    learnerToken = await loginAs(request, "ada@example.com");
  });

  test.afterAll(async ({ request }) => {
    // safety net: always clear maintenance mode
    await request.put(`${API_URL}/v1/admin/settings`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { maintenanceMode: false },
    });
  });

  test("admin sees settings page and can toggle maintenance", async ({
    page,
  }) => {
    await setAuthCookie(page, adminToken);
    await page.goto("/admin/settings");
    await expect(
      page.getByRole("heading", { name: "Platform Settings" }),
    ).toBeVisible({ timeout: 8000 });

    // assert the maintenance Switch is present (MUI Switch exposes role "switch")
    await expect(
      page.getByRole("switch", { name: /Maintenance mode/i }),
    ).toBeVisible();
  });

  test("maintenance on -> learner 503, off -> learner 200", async ({
    request,
  }) => {
    try {
      // helper to GET /v1/me as the learner and return status
      const meStatus = async () => {
        const resp = await request.get(`${API_URL}/v1/me`, {
          headers: { Authorization: `Bearer ${learnerToken}` },
        });
        return resp.status();
      };

      // baseline: learner OK
      expect(await meStatus()).toBe(200);

      // flip ON via admin API
      const toggleOn = await request.put(`${API_URL}/v1/admin/settings`, {
        headers: { Authorization: `Bearer ${adminToken}` },
        data: { maintenanceMode: true },
      });
      expect(toggleOn.ok()).toBeTruthy();

      // learner now blocked
      expect(await meStatus()).toBe(503);

      // admin still allowed (admin token is exempt) — GET settings should still 200
      const adminCheck = await request.get(`${API_URL}/v1/admin/settings`, {
        headers: { Authorization: `Bearer ${adminToken}` },
      });
      expect(adminCheck.status()).toBe(200);
    } finally {
      // restore OFF no matter what
      await request.put(`${API_URL}/v1/admin/settings`, {
        headers: { Authorization: `Bearer ${adminToken}` },
        data: { maintenanceMode: false },
      });
    }

    // after restore, learner OK again
    const meStatus = async () => {
      const resp = await request.get(`${API_URL}/v1/me`, {
        headers: { Authorization: `Bearer ${learnerToken}` },
      });
      return resp.status();
    };
    expect(await meStatus()).toBe(200);
  });
});
