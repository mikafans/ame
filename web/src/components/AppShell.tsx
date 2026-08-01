"use client";

import React, { useState } from "react";
import { LogOut, Menu } from "lucide-react";
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
          <nav aria-label="Main navigation" className="hidden h-16 items-stretch gap-1 md:flex">
            {[
              ["learning", "Learning desk"],
              ["agent", "Agent API"],
              ...(user?.role === "admin" ? [["admin-dashboard", "Admin"]] : []),
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
            <span className="max-w-32 truncate text-sm font-medium">
              {user?.displayName}
            </span>
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
      <main className="mx-auto min-w-0 max-w-7xl px-4 py-6 sm:px-6 sm:py-8">{children}</main>
    </div>
  );
}
