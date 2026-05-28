"use client";

import { useState, useEffect } from "react";

export interface AuthUser {
  id: string;
  displayName: string;
  email?: string;
  role: string;
}

const API_URL =
  typeof window !== "undefined"
    ? (process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080")
    : "http://localhost:8080";

function getCookieToken(): string | undefined {
  if (typeof document === "undefined") return undefined;
  return (document.cookie.match(/(?:^|;\s*)ame_token=([^;]+)/) ?? [])[1];
}

// Cookie is non-HttpOnly because we read the token client-side to attach it as
// a Bearer header. `Secure` is set in production so it cannot leak over plain
// HTTP. SameSite=Lax is sufficient since cross-origin requests use the
// `Authorization` header rather than ambient cookies.
const COOKIE_FLAGS =
  process.env.NODE_ENV === "production"
    ? "path=/; max-age=86400; SameSite=Lax; Secure"
    : "path=/; max-age=86400; SameSite=Lax";

export function setAuthToken(token: string) {
  document.cookie = `ame_token=${token}; ${COOKIE_FLAGS}`;
}

export function clearAuthToken() {
  const expire =
    process.env.NODE_ENV === "production"
      ? "path=/; max-age=0; SameSite=Lax; Secure"
      : "path=/; max-age=0; SameSite=Lax";
  document.cookie = `ame_token=; ${expire}`;
}

export function useAuth() {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);
  const [token, setToken] = useState<string | undefined>(undefined);

  useEffect(() => {
    const t = getCookieToken();
    if (!t) {
      setLoading(false);
      return;
    }
    setToken(t);
    fetch(`${API_URL}/v1/me`, {
      headers: { Authorization: `Bearer ${t}` },
    })
      .then((r) => {
        if (!r.ok) {
          clearAuthToken();
          setToken(undefined);
          return null;
        }
        return r.json();
      })
      .then((data) => {
        if (data) setUser(data as AuthUser);
      })
      .catch(() => {
        setToken(undefined);
      })
      .finally(() => setLoading(false));
  }, []);

  return { user, loading, token };
}
