"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/hooks/useAuth";

export default function LearnerLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { user, loading } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (!loading && !user) {
      router.replace("/login");
    }
  }, [user, loading, router]);

  if (loading) {
    return (
      <div
        style={{
          minHeight: "100vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "var(--muted)",
          fontFamily: "var(--mono)",
          fontSize: 13,
        }}
      >
        Loading…
      </div>
    );
  }

  if (!user) return null;

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "220px 1fr",
        minHeight: "100vh",
      }}
    >
      {/* Sidebar */}
      <nav
        style={{
          background: "var(--surface)",
          borderRight: "1px solid var(--border)",
          padding: "24px 16px",
          display: "flex",
          flexDirection: "column",
          gap: 4,
        }}
      >
        <div
          style={{
            fontFamily: "var(--mono)",
            fontSize: 11,
            letterSpacing: 1.5,
            textTransform: "uppercase",
            color: "var(--accent)",
            marginBottom: 20,
            padding: "0 8px",
          }}
        >
          Harus
        </div>
        {[
          { href: "/", label: "Library" },
          { href: "/practice", label: "Practice" },
          { href: "/progress", label: "Progress" },
        ].map(({ href, label }) => (
          <a
            key={href}
            href={href}
            style={{
              display: "block",
              padding: "8px 12px",
              borderRadius: 4,
              color: "var(--text-2)",
              textDecoration: "none",
              fontSize: 13,
              fontWeight: 500,
            }}
          >
            {label}
          </a>
        ))}
        <div
          style={{
            marginTop: "auto",
            borderTop: "1px solid var(--border)",
            paddingTop: 16,
          }}
        >
          <div
            style={{ padding: "0 12px", color: "var(--muted)", fontSize: 12 }}
          >
            {user.displayName}
          </div>
          <a
            href="/login"
            onClick={() => {
              document.cookie = "ame_token=; path=/; max-age=0";
            }}
            style={{
              display: "block",
              padding: "6px 12px",
              color: "var(--muted)",
              fontSize: 12,
              textDecoration: "none",
            }}
          >
            Sign out
          </a>
        </div>
      </nav>

      {/* Main */}
      <main style={{ overflowY: "auto" }}>{children}</main>
    </div>
  );
}
