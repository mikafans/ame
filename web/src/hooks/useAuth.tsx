"use client";

import React, {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
} from "react";
import { useRouter } from "next/navigation";

export interface AuthUser {
  id: string;
  displayName: string;
  email?: string;
  role: string;
  plan: "free" | "premium";
}

interface AuthContextType {
  user: AuthUser | null;
  loading: boolean;
  refresh: () => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

const API_URL =
  typeof window !== "undefined"
    ? (process.env.NEXT_PUBLIC_API_URL ??
      `http://${window.location.hostname}:28080`)
    : "http://localhost:28080";

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);
  const router = useRouter();

  const fetchUser = useCallback(async () => {
    try {
      const r = await fetch(`${API_URL}/api/v1/me`, {
        credentials: "include",
      });
      if (!r.ok) {
        setUser(null);
        return;
      }
      const data = await r.json();
      setUser(data as AuthUser);
    } catch (err) {
      console.error("Auth fetch failed:", err);
      setUser(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (
      document.cookie
        .split(";")
        .some((cookie) => cookie.trim() === "ame_session=1")
    ) {
      fetchUser();
    } else {
      setLoading(false);
    }
  }, [fetchUser]);

  const logout = useCallback(async () => {
    try {
      await fetch(`${API_URL}/api/v1/auth/logout`, {
        method: "POST",
        credentials: "include",
      });
    } catch (err) {
      console.error("Logout request failed:", err);
    }
    setUser(null);
    document.cookie = "ame_session=; SameSite=Lax; Path=/; Max-Age=0";
    if (typeof window !== "undefined") {
      router.push("/login");
    }
  }, [router]);

  return (
    <AuthContext.Provider value={{ user, loading, refresh: fetchUser, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}

/**
 * @deprecated Use the logout function from useAuth() hook where possible.
 * This remains available for non-component callers.
 */
export async function logout() {
  try {
    await fetch(`${API_URL}/api/v1/auth/logout`, {
      method: "POST",
      credentials: "include",
    });
  } catch (err) {
    console.error("Standalone logout request failed:", err);
  }
  if (typeof window !== "undefined") {
    window.location.href = "/login";
  }
}
