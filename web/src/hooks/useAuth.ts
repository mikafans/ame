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

export function setAuthToken(token: string) {
  document.cookie = `ame_token=${token}; path=/; max-age=86400; SameSite=Lax`;
}

export function clearAuthToken() {
  document.cookie = "ame_token=; path=/; max-age=0";
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
