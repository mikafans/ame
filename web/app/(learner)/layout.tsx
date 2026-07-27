"use client";

import { useEffect } from "react";
import { useRouter, usePathname } from "next/navigation";
import { useAuth } from "@/hooks/useAuth";
import { AppShell } from "@/components/AppShell";

export default function LearnerLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { user, loading } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    if (!loading && !user) {
      window.location.href = "/login";
    }
  }, [loading, user]);

  const getRouteId = () => {
    if (pathname.startsWith("/admin/users")) return "admin-users";
    if (pathname.startsWith("/admin/tokens")) return "admin-tokens";
    if (pathname.startsWith("/admin/audit")) return "admin-audit";
    if (pathname.startsWith("/admin/health")) return "admin-health";
    if (pathname.startsWith("/admin")) return "admin-dashboard";
    if (pathname.startsWith("/learning")) return "learning";
    if (pathname.startsWith("/agent")) return "agent";
    return "explore";
  };

  const handleRouteChange = (route: string) => {
    const routeMap: Record<string, string> = {
      "admin-dashboard": "/admin",
      "admin-users": "/admin/users",
      "admin-tokens": "/admin/tokens",
      "admin-audit": "/admin/audit",
      "admin-health": "/admin/health",
      learning: "/learning",
      agent: "/agent",
    };
    router.push(routeMap[route] || "/learning");
  };

  return (
    <AppShell route={getRouteId()} setRoute={handleRouteChange}>
      {children}
    </AppShell>
  );
}
