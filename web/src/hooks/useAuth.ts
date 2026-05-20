"use client";

import { useState, useEffect } from "react";
import { makeClient } from "@/api/client";

export interface AuthUser {
  id: string;
  displayName: string;
  email?: string;
  role: string;
}

function getTokenFromCookie(): string | undefined {
  if (typeof document === "undefined") return undefined;
  return (document.cookie.match(/(?:^|;\s*)ame_token=([^;]+)/) ?? [])[1];
}

export function setAuthToken(token: string) {
  document.cookie = `ame_token=${encodeURIComponent(token)}; path=/; max-age=${60 * 60 * 24 * 30}; SameSite=Lax`;
}

export function clearAuthToken() {
  document.cookie = "ame_token=; path=/; max-age=0";
}

export function useAuth() {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const token = getTokenFromCookie();
    if (!token) {
      setLoading(false);
      return;
    }
    const client = makeClient(token);
    client
      .GET("/v1/me" as never)
      .then(({ data }: { data?: AuthUser }) => {
        if (data) setUser(data as AuthUser);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  return { user, loading, token: getTokenFromCookie() };
}
