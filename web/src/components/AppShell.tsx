"use client";

import React, { useState } from "react";
import { Menu } from "lucide-react";
import { Sidebar } from "@/components/Sidebar";
import { Logo } from "@/components/Logo";

interface AppShellProps {
  route: string;
  setRoute: (route: string) => void;
  children: React.ReactNode;
}

export function AppShell({ route, setRoute, children }: AppShellProps) {
  const [mobileOpen, setMobileOpen] = useState(false);

  const handleRouteChange = (r: string) => {
    setMobileOpen(false);
    setRoute(r);
  };

  const handleMobileClose = () => {
    setMobileOpen(false);
  };

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <Sidebar
        route={route}
        setRoute={handleRouteChange}
        mobileOpen={mobileOpen}
        onClose={handleMobileClose}
      />
      <main className="min-w-0 flex-1 overflow-y-auto">
        <header className="sticky top-0 z-10 flex min-h-12 items-center border-b border-border bg-background px-4 md:hidden">
          <button
            type="button"
            aria-label="Open navigation"
            onClick={() => setMobileOpen(true)}
            className="mr-3 inline-flex size-8 items-center justify-center rounded-lg outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
          >
            <Menu className="size-5" />
          </button>
          <Logo size={20} />
        </header>
        {children}
      </main>
    </div>
  );
}
