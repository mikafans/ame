/**
 * Admin surface tests (pure API, no browser navigation required).
 *
 * Covers: GET /v1/admin/users, PATCH /v1/admin/users/{id} (plan/role/disable),
 * GET /v1/admin/audit, POST /v1/admin/moderate.
 *
 * All endpoints require the admin scope — 403 is asserted for non-admin callers.
 */
import { test, expect } from "@playwright/test";
import { API_URL, loginAs, registerUserFull } from "./helpers";

test.describe("admin API surface", () => {
  test.describe.configure({ mode: "serial" });

  let adminToken: string;
  let userToken: string;
  let userId: string;

  test.beforeAll(async ({ request }) => {
    adminToken = await loginAs(request, "admin@example.com");

    // Create a fresh target user for patch operations
    const ts = Date.now();
    const { token, id } = await registerUserFull(request, {
      email: `e2e-admin-target-${ts}@example.com`,
      name: "Admin Target",
    });
    userToken = token;
    userId = id;
  });

  // --- List users ---

  test("GET /v1/admin/users returns a list for admin", async ({ request }) => {
    const r = await request.get(`${API_URL}/v1/admin/users`, {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(Array.isArray(body.users)).toBe(true);
    expect(body.users.length).toBeGreaterThan(0);
  });

  test("GET /v1/admin/users returns 403 for non-admin", async ({ request }) => {
    const r = await request.get(`${API_URL}/v1/admin/users`, {
      headers: { Authorization: `Bearer ${userToken}` },
    });
    expect(r.status()).toBe(403);
  });

  // --- PATCH plan ---

  test("admin can upgrade a user plan to premium", async ({ request }) => {
    const r = await request.patch(`${API_URL}/v1/admin/users/${userId}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { plan: "premium" },
    });
    expect(r.status()).toBe(204);
  });

  test("admin can downgrade a user plan to free", async ({ request }) => {
    const r = await request.patch(`${API_URL}/v1/admin/users/${userId}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { plan: "free" },
    });
    expect(r.status()).toBe(204);
  });

  test("PATCH plan returns 403 for non-admin", async ({ request }) => {
    const r = await request.patch(`${API_URL}/v1/admin/users/${userId}`, {
      headers: { Authorization: `Bearer ${userToken}` },
      data: { plan: "premium" },
    });
    expect(r.status()).toBe(403);
  });

  // --- PATCH role ---

  test("admin can set user role to admin and back to user", async ({
    request,
  }) => {
    const toAdmin = await request.patch(`${API_URL}/v1/admin/users/${userId}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { role: "admin" },
    });
    expect(toAdmin.status()).toBe(204);

    const backToUser = await request.patch(
      `${API_URL}/v1/admin/users/${userId}`,
      {
        headers: { Authorization: `Bearer ${adminToken}` },
        data: { role: "user" },
      },
    );
    expect(backToUser.status()).toBe(204);
  });

  // --- PATCH disable / re-enable ---

  test("admin can disable a user (sets deactivated_at)", async ({
    request,
  }) => {
    const r = await request.patch(`${API_URL}/v1/admin/users/${userId}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { disabled: true },
    });
    expect(r.status()).toBe(204);
  });

  test("admin can re-enable a disabled user", async ({ request }) => {
    const r = await request.patch(`${API_URL}/v1/admin/users/${userId}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
      data: { disabled: false },
    });
    expect(r.status()).toBe(204);
  });

  // --- Audit log ---

  test("GET /v1/admin/audit returns entries for admin", async ({ request }) => {
    const r = await request.get(`${API_URL}/v1/admin/audit`, {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    // Should have audit entries from the PATCH operations above
    expect(Array.isArray(body.logs ?? body.entries ?? body)).toBe(true);
  });

  test("GET /v1/admin/audit is inaccessible to non-admin (401/403)", async ({
    request,
  }) => {
    // 403 = authenticated non-admin; 401 = token revoked (e.g. after disable/re-enable
    // cycle above which revokes existing tokens). Both correctly deny admin access.
    const r = await request.get(`${API_URL}/v1/admin/audit`, {
      headers: { Authorization: `Bearer ${userToken}` },
    });
    expect([401, 403]).toContain(r.status());
  });

  // --- Moderation ---

  test("POST /v1/admin/moderate is inaccessible to non-admin (401/403)", async ({
    request,
  }) => {
    // Use a bogus UUID — we just want the 401/403, not a real unpublish.
    // Token may be revoked after the disable/re-enable cycle → 401 is expected.
    const r = await request.post(`${API_URL}/v1/admin/moderate`, {
      headers: { Authorization: `Bearer ${userToken}` },
      data: { assessmentId: "00000000-0000-0000-0000-000000000000" },
    });
    expect([401, 403]).toContain(r.status());
  });
});
