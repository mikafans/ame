"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { setAuthToken } from "@/hooks/useAuth";
import { Button, Icon, Logo } from "@/components/ui";

type Tab = "signup" | "login";
type Role = "learner" | "instructor" | "agent";

export default function LoginPage() {
  const router = useRouter();
  const [tab, setTab] = useState<Tab>("signup");
  const [fullName, setFullName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("learner");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const apiUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      const endpoint =
        tab === "signup"
          ? `${apiUrl}/v1/auth/register`
          : `${apiUrl}/v1/auth/login`;

      const body =
        tab === "signup"
          ? {
              email,
              name: fullName,
              password,
              role,
            }
          : {
              email,
              password,
            };

      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        let message = "Authentication failed";
        try {
          const data = await response.json();
          // Look for errors in different possible structures
          message =
            data?.error?.message ||
            data?.message ||
            (typeof data?.error === "string" ? data.error : null) ||
            `Error: ${response.statusText}`;
        } catch {
          // If response body isn't JSON, fallback to status text
          message = `Authentication failed: ${response.status} ${response.statusText}`;
        }
        setError(message);
        return;
      }

      const data = await response.json();
      setAuthToken(data.token);
      router.push("/library");
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Could not reach the server",
      );
    } finally {
      setLoading(false);
    }
  }

  const roleDescriptions: Record<Role, string> = {
    learner: "Take assigned quizzes and track your progress.",
    instructor: "Author quizzes, manage cohorts, and review attempts.",
    agent: "Get an API key, an OpenAPI schema, and MCP tool descriptors.",
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "grid",
        gridTemplateColumns: "1.05fr 1fr",
        background: "var(--bg)",
      }}
    >
      {/* LEFT PANEL: Marketing */}
      <div
        style={{
          padding: "56px 64px",
          borderRight: "1px solid var(--border)",
          background:
            "linear-gradient(180deg, var(--surface) 0%, var(--bg) 70%)",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
        }}
      >
        <Logo size={28} />

        <div style={{ maxWidth: 520 }}>
          <div
            style={{
              fontFamily: "var(--mono)",
              fontSize: 11,
              letterSpacing: 1.6,
              textTransform: "uppercase",
              color: "var(--accent)",
              marginBottom: 18,
            }}
          >
            Assessment platform · est. 2025
          </div>
          <h1
            style={{
              fontFamily: "var(--serif)",
              fontSize: 56,
              lineHeight: 1.04,
              margin: 0,
              fontWeight: 500,
              letterSpacing: -1.2,
            }}
          >
            Quizzes that learners and agents can both read.
          </h1>
          <p
            style={{
              color: "var(--text-2)",
              fontSize: 16,
              lineHeight: 1.55,
              marginTop: 22,
              maxWidth: 460,
            }}
          >
            Harus is an assessment platform built for two audiences at once.
            Students get a focused test-taking experience and a real progress
            dashboard. Authors and AI agents share the same structured surface —
            every quiz, attempt, and rubric is addressable, importable, and
            queryable through a single API.
          </p>

          <div
            style={{
              marginTop: 36,
              display: "grid",
              gridTemplateColumns: "1fr 1fr",
              gap: 12,
              maxWidth: 460,
            }}
          >
            {[
              ["18,402", "active learners"],
              ["1,243", "instructors"],
              ["94", "institutions"],
              ["6.1M", "graded attempts"],
            ].map(([value, label]) => (
              <div
                key={label}
                style={{
                  padding: "12px 14px",
                  border: "1px solid var(--border)",
                  borderRadius: 6,
                }}
              >
                <div
                  style={{
                    fontFamily: "var(--serif)",
                    fontSize: 22,
                    fontWeight: 500,
                  }}
                >
                  {value}
                </div>
                <div
                  style={{
                    fontSize: 11,
                    color: "var(--muted)",
                    fontFamily: "var(--mono)",
                    letterSpacing: 1,
                    textTransform: "uppercase",
                    marginTop: 2,
                  }}
                >
                  {label}
                </div>
              </div>
            ))}
          </div>
        </div>

        <div
          style={{
            display: "flex",
            gap: 18,
            alignItems: "center",
            color: "var(--muted)",
            fontSize: 12,
            fontFamily: "var(--mono)",
            letterSpacing: 0.6,
          }}
        >
          <span>SSO · SAML</span>
          <span>·</span>
          <span>FERPA · GDPR</span>
          <span>·</span>
          <span>OpenAPI 3.1 · MCP</span>
        </div>
      </div>

      {/* RIGHT PANEL: Form */}
      <div
        style={{
          padding: "56px 64px",
          display: "flex",
          alignItems: "center",
        }}
      >
        <div style={{ width: "100%", maxWidth: 420 }}>
          {/* Tab Bar */}
          <div
            style={{
              display: "flex",
              gap: 0,
              borderBottom: "1px solid var(--border)",
              marginBottom: 28,
            }}
          >
            {(["signup", "login"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setTab(t)}
                style={{
                  background: "transparent",
                  border: "none",
                  padding: "10px 0",
                  marginRight: 24,
                  color: tab === t ? "var(--text)" : "var(--muted)",
                  borderBottom: `2px solid ${
                    tab === t ? "var(--accent)" : "transparent"
                  }`,
                  fontWeight: tab === t ? 600 : 500,
                  fontSize: 13,
                  letterSpacing: 0.3,
                  cursor: "pointer",
                  fontFamily: "var(--sans)",
                }}
              >
                {t === "signup" ? "Create account" : "Sign in"}
              </button>
            ))}
          </div>

          {/* Form */}
          <form onSubmit={handleSubmit}>
            <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
              {/* Full Name (signup only) */}
              {tab === "signup" && (
                <div>
                  <label
                    style={{
                      display: "block",
                      fontSize: 11,
                      fontFamily: "var(--mono)",
                      letterSpacing: 1.2,
                      textTransform: "uppercase",
                      color: "var(--muted)",
                      marginBottom: 6,
                    }}
                  >
                    Full name
                  </label>
                  <input
                    type="text"
                    value={fullName}
                    onChange={(e) => setFullName(e.target.value)}
                    style={{
                      width: "100%",
                      background: "var(--surface)",
                      border: "1px solid var(--border)",
                      borderRadius: 6,
                      padding: "10px 12px",
                      fontSize: 14,
                      color: "var(--text)",
                      outline: "none",
                      fontFamily: "var(--sans)",
                      boxSizing: "border-box",
                    }}
                  />
                </div>
              )}

              {/* Email */}
              <div>
                <label
                  style={{
                    display: "block",
                    fontSize: 11,
                    fontFamily: "var(--mono)",
                    letterSpacing: 1.2,
                    textTransform: "uppercase",
                    color: "var(--muted)",
                    marginBottom: 6,
                  }}
                >
                  Institutional email
                </label>
                <input
                  type="email"
                  id="email"
                  name="email"
                  autoComplete="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  style={{
                    width: "100%",
                    background: "var(--surface)",
                    border: "1px solid var(--border)",
                    borderRadius: 6,
                    padding: "10px 12px",
                    fontSize: 14,
                    color: "var(--text)",
                    outline: "none",
                    fontFamily: "var(--sans)",
                    boxSizing: "border-box",
                  }}
                />
                <div
                  style={{
                    fontSize: 11,
                    color: "var(--muted)",
                    marginTop: 6,
                    fontFamily: "var(--mono)",
                  }}
                >
                  Recognized: stanford.edu · SSO available
                </div>
              </div>

              {/* Password */}
              <div>
                <label
                  style={{
                    display: "block",
                    fontSize: 11,
                    fontFamily: "var(--mono)",
                    letterSpacing: 1.2,
                    textTransform: "uppercase",
                    color: "var(--muted)",
                    marginBottom: 6,
                  }}
                >
                  Password
                </label>
                <input
                  type="password"
                  id="password"
                  name="password"
                  autoComplete={
                    tab === "signup" ? "new-password" : "current-password"
                  }
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  style={{
                    width: "100%",
                    background: "var(--surface)",
                    border: "1px solid var(--border)",
                    borderRadius: 6,
                    padding: "10px 12px",
                    fontSize: 14,
                    color: "var(--text)",
                    outline: "none",
                    fontFamily: "var(--sans)",
                    boxSizing: "border-box",
                  }}
                />
              </div>

              {/* Role Picker (signup only) */}
              {tab === "signup" && (
                <div>
                  <label
                    style={{
                      display: "block",
                      fontSize: 11,
                      fontFamily: "var(--mono)",
                      letterSpacing: 1.2,
                      textTransform: "uppercase",
                      color: "var(--muted)",
                      marginBottom: 6,
                    }}
                  >
                    Role
                  </label>
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "1fr 1fr 1fr",
                      gap: 8,
                      marginBottom: 8,
                    }}
                  >
                    {(["learner", "instructor", "agent"] as const).map((r) => (
                      <button
                        key={r}
                        type="button"
                        onClick={() => setRole(r)}
                        style={{
                          padding: "10px 8px",
                          background:
                            role === r ? "var(--accent-dim)" : "var(--surface)",
                          border: `1px solid ${
                            role === r ? "var(--accent-line)" : "var(--border)"
                          }`,
                          color: role === r ? "var(--accent)" : "var(--text-2)",
                          borderRadius: 6,
                          fontSize: 12,
                          fontWeight: 500,
                          textTransform: "capitalize",
                          cursor: "pointer",
                          fontFamily: "var(--sans)",
                        }}
                      >
                        {r}
                      </button>
                    ))}
                  </div>
                  <div
                    style={{
                      fontSize: 11,
                      color: "var(--muted)",
                      lineHeight: 1.5,
                    }}
                  >
                    {roleDescriptions[role]}
                  </div>
                </div>
              )}

              {/* Error Message */}
              {error && (
                <div
                  style={{
                    padding: "8px 12px",
                    background: "var(--red-dim)",
                    border: "1px solid var(--red)",
                    borderRadius: 6,
                    color: "var(--red)",
                    fontSize: 12,
                  }}
                >
                  {error}
                </div>
              )}

              {/* Primary Button */}
              <div style={{ marginTop: 12 }}>
                <Button
                  variant="primary"
                  size="lg"
                  type="submit"
                  disabled={loading}
                  style={{ width: "100%", justifyContent: "center" }}
                >
                  {loading
                    ? "Loading…"
                    : tab === "signup"
                      ? "Create account"
                      : "Sign in"}
                </Button>
              </div>

              {/* OR Divider */}
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 12,
                  color: "var(--muted)",
                  fontSize: 11,
                  margin: "8px 0",
                }}
              >
                <div
                  style={{ flex: 1, height: 1, background: "var(--border)" }}
                />
                <span style={{ fontFamily: "var(--mono)", letterSpacing: 1.2 }}>
                  OR
                </span>
                <div
                  style={{ flex: 1, height: 1, background: "var(--border)" }}
                />
              </div>

              {/* Ghost Buttons */}
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: 8,
                }}
              >
                <Button variant="ghost">Continue with SSO</Button>
                <Button variant="ghost">Use access code</Button>
              </div>
            </div>
          </form>

          {/* Agent Shortcut Callout */}
          <div
            style={{
              marginTop: 36,
              padding: 14,
              border: "1px dashed var(--border-strong)",
              borderRadius: 6,
              background: "var(--surface)",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                color: "var(--accent)",
                fontSize: 12,
                fontWeight: 600,
                marginBottom: 4,
              }}
            >
              <Icon name="sparkle" size={14} color="var(--accent)" />
              Agent shortcut
            </div>
            <div
              style={{
                fontSize: 12,
                color: "var(--text-2)",
                lineHeight: 1.5,
              }}
            >
              Programmatic access?{" "}
              <span
                style={{
                  fontFamily: "var(--mono)",
                  color: "var(--text)",
                }}
              >
                POST /v1/agents/register
              </span>{" "}
              returns a key, an OpenAPI schema, and an MCP manifest in one call.
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
