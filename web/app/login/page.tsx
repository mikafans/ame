"use client";

import { useState, FormEvent } from "react";
import { useRouter } from "next/navigation";
import { setAuthToken } from "@/hooks/useAuth";
import { makeClient } from "@/api/client";

export default function LoginPage() {
  const router = useRouter();
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const client = makeClient(token.trim());
      const { data, error: apiErr } = await client.GET("/v1/me" as never);
      if (apiErr || !data) {
        setError("Invalid API key — check and try again.");
        return;
      }
      setAuthToken(token.trim());
      router.push("/");
    } catch {
      setError("Could not reach the API. Is the server running?");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "var(--bg)",
      }}
    >
      <div
        style={{
          width: 400,
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 8,
          padding: "32px 28px",
        }}
      >
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 10,
            letterSpacing: 1.5,
            textTransform: "uppercase",
            color: "var(--muted)",
            marginBottom: 8,
          }}
        >
          Harus
        </div>
        <h1
          style={{
            margin: "0 0 4px",
            fontSize: 22,
            fontWeight: 600,
            color: "var(--text)",
          }}
        >
          Sign in
        </h1>
        <p style={{ margin: "0 0 24px", color: "var(--muted)", fontSize: 13 }}>
          Paste your API key to continue.
        </p>

        <form onSubmit={handleSubmit}>
          <label
            style={{
              display: "block",
              fontSize: 12,
              fontFamily: "var(--mono)",
              color: "var(--muted)",
              marginBottom: 6,
              letterSpacing: 0.5,
            }}
          >
            API KEY
          </label>
          <input
            type="password"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="uuid_secret"
            required
            style={{
              width: "100%",
              padding: "10px 12px",
              background: "var(--surface-2)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              color: "var(--text)",
              fontFamily: "var(--mono)",
              fontSize: 13,
              boxSizing: "border-box",
              marginBottom: error ? 8 : 16,
              outline: "none",
            }}
          />
          {error && (
            <div
              style={{
                padding: "8px 12px",
                background: "var(--red-dim)",
                border: "1px solid var(--red)",
                borderRadius: 4,
                color: "var(--red)",
                fontSize: 12,
                marginBottom: 16,
              }}
            >
              {error}
            </div>
          )}
          <button
            type="submit"
            disabled={loading || !token.trim()}
            style={{
              width: "100%",
              padding: "10px 0",
              background: "var(--accent)",
              color: "#000",
              border: "none",
              borderRadius: 4,
              fontWeight: 600,
              fontSize: 14,
              cursor: loading || !token.trim() ? "not-allowed" : "pointer",
              opacity: loading || !token.trim() ? 0.6 : 1,
            }}
          >
            {loading ? "Verifying…" : "Continue"}
          </button>
        </form>
      </div>
    </div>
  );
}
