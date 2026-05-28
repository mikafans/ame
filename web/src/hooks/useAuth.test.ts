import { describe, it, expect, mock, beforeEach } from "bun:test";

const mockFetch = mock(() =>
  Promise.resolve({ ok: true, json: () => Promise.resolve(null) }),
);

beforeEach(() => {
  mockFetch.mockReset();
  global.fetch = mockFetch as unknown as typeof fetch;
});

describe("logout", () => {
  it("POSTs to /v1/auth/logout with credentials", async () => {
    const { logout } = await import("./useAuth");
    await logout();
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/v1/auth/logout"),
      expect.objectContaining({ method: "POST", credentials: "include" }),
    );
  });

  it("does not throw when the request fails", async () => {
    mockFetch.mockImplementation(() => Promise.reject(new Error("network")));
    const spy = mock(() => {});
    const original = console.error;
    console.error = spy;
    const { logout } = await import("./useAuth");
    await expect(logout()).resolves.toBeUndefined();
    console.error = original;
  });
});
