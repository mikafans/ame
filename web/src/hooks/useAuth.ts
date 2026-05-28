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

export async function logout() {
  try {
    await fetch(`${API_URL}/v1/auth/logout`, {
      method: "POST",
      credentials: "include",
    });
  } catch (err) {
    console.error("Logout request failed:", err);
  }
  if (typeof window !== "undefined") {
    window.location.href = "/login";
  }
}

export function useAuth() {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch(`${API_URL}/v1/me`, {
      credentials: "include",
    })
      .then((r) => {
        if (!r.ok) {
          return null;
        }
        return r.json();
      })
      .then((data) => {
        if (data) setUser(data as AuthUser);
      })
      .catch(() => {
        // Not logged in or request failed
      })
      .finally(() => setLoading(false));
  }, []);

  return { user, loading };
}
