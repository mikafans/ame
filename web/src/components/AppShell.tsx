"use client";

import React, { useEffect, useState } from "react";
import Link from "next/link";
import { LogOut, Menu } from "lucide-react";
import { api } from "@/api/client";
import type { components } from "@/api/generated/schema.d.ts";
import { Sidebar } from "@/components/Sidebar";
import { Logo } from "@/components/Logo";
import { ThemeSelector } from "@/components/ThemeSelector";
import { useAuth } from "@/hooks/useAuth";

interface AppShellProps {
  route: string;
  setRoute: (route: string) => void;
  children: React.ReactNode;
}

export function AppShell({ route, setRoute, children }: AppShellProps) {
  const [mobileOpen, setMobileOpen] = useState(false);
  const { user, logout } = useAuth();
  const [rateLimit, setRateLimit] = useState<
    components["schemas"]["RateLimitStatusResponse"] | null
  >(null);

  useEffect(() => {
    if (!user) {
      setRateLimit(null);
      return;
    }
    void api.GET("/api/v1/me/rate-limit").then((result) => {
      if (result.response.ok && result.data) setRateLimit(result.data);
    });
  }, [user]);

  const handleRouteChange = (r: string) => {
    setMobileOpen(false);
    setRoute(r);
  };

  const handleMobileClose = () => {
    setMobileOpen(false);
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Sidebar
        route={route}
        setRoute={handleRouteChange}
        desktop={false}
        mobileOpen={mobileOpen}
        onClose={handleMobileClose}
      />
      <header className="sticky top-0 z-20 border-b border-border bg-background">
        <div className="mx-auto flex min-h-16 max-w-7xl items-center gap-4 px-4 sm:px-6">
          <button
            type="button"
            aria-label="Open navigation"
            onClick={() => setMobileOpen(true)}
            className="inline-flex size-8 items-center justify-center rounded-lg outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring md:hidden"
          >
            <Menu className="size-5" />
          </button>
          <Logo size={22} />
          <nav
            aria-label="Main navigation"
            className="hidden h-16 items-stretch gap-1 md:flex"
          >
            {[
              ["learning", "Learning desk"],
              ...(user?.role === "admin" ? [["admin-dashboard", "Admin"]] : []),
              ["about", "About"],
            ].map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => handleRouteChange(id)}
                className={`border-b-2 px-3 text-sm font-medium transition ${route === id ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:border-border hover:text-foreground"}`}
              >
                {label}
              </button>
            ))}
          </nav>
          <div className="ml-auto hidden items-center gap-3 md:flex">
            <ThemeSelector />
            {rateLimit && (
              <span
                className="rounded-full border border-border px-2 py-1 text-xs text-muted-foreground"
                title={`Rate budget resets in ${rateLimit.resetAfterSeconds} seconds`}
              >
                {rateLimit.tier === "premium" ? "VIP" : "Free"} ·{" "}
                {rateLimit.remaining.toLocaleString()}/
                {rateLimit.limit.toLocaleString()}
              </span>
            )}
            <span className="max-w-32 truncate text-sm font-medium">
              {user?.displayName}
            </span>
            {user && (
              <span className="rounded-full border border-primary/30 px-2 py-1 text-[0.65rem] font-semibold uppercase tracking-wide text-primary">
                {user.plan === "premium" ? "VIP" : "Free"}
              </span>
            )}
            <button
              type="button"
              title="Sign out"
              aria-label="Sign out"
              onClick={() => void logout()}
              className="inline-flex size-8 items-center justify-center rounded-lg text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
            >
              <LogOut className="size-4" />
            </button>
          </div>
        </div>
      </header>
      <main className="mx-auto min-w-0 max-w-7xl px-4 py-6 sm:px-6 sm:py-8">
        {children}
      </main>
      <footer className="border-t border-border px-4 py-6 text-sm text-muted-foreground sm:px-6">
        <div className="mx-auto flex max-w-7xl flex-wrap justify-between gap-4">
          <span>© 2026 AME</span>
          <div className="flex gap-5">
            <Link href="/about" className="transition hover:text-primary">
              About
            </Link>
            <Link href="/agent" className="transition hover:text-primary">
              Agent API
            </Link>
            <a
              href="https://github.com/mikafans/ame"
              className="transition hover:text-primary"
            >
              GitHub
            </a>
            <Link
              href="/self-hosting"
              className="transition hover:text-primary"
            >
              Self-hosting
            </Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
