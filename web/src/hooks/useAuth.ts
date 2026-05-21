"use client";

export interface AuthUser {
  id: string;
  displayName: string;
  email?: string;
  role: string;
}

const DEMO_USER: AuthUser = {
  id: "00000000-0000-0000-0000-000000000001",
  displayName: "Haru",
  email: "haru@harus.dev",
  role: "instructor",
};

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function setAuthToken(_token: string) {}
export function clearAuthToken() {}

export function useAuth() {
  return { user: DEMO_USER, loading: false, token: "demo" };
}
