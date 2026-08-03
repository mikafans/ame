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
  const isPublicAgentGuide = pathname.startsWith("/agent");
  const isPublicPage = isPublicAgentGuide || pathname.startsWith("/about");

  useEffect(() => {
    if (!isPublicPage && !loading && !user) {
      window.location.href = "/login";
    }
  }, [isPublicPage, loading, user]);

  if (isPublicAgentGuide) {
    return children;
  }

  const getRouteId = () => {
    if (pathname.startsWith("/admin/users")) return "admin-users";
    if (pathname.startsWith("/admin/audit")) return "admin-audit";
    if (pathname.startsWith("/admin/health")) return "admin-health";
    if (pathname.startsWith("/admin/settings")) return "admin-settings";
    if (pathname.startsWith("/admin/tasks")) return "admin-tasks";
    if (pathname.startsWith("/admin")) return "admin-dashboard";
    if (pathname.startsWith("/about")) return "about";
    if (pathname.startsWith("/learning")) return "learning";
    return "explore";
  };

  const handleRouteChange = (route: string) => {
    const routeMap: Record<string, string> = {
      "admin-dashboard": "/admin",
      "admin-users": "/admin/users",
      "admin-audit": "/admin/audit",
      "admin-health": "/admin/health",
      "admin-settings": "/admin/settings",
      "admin-tasks": "/admin/tasks",
      about: "/about",
      learning: "/learning",
    };
    router.push(routeMap[route] || "/learning");
  };

  return (
    <AppShell route={getRouteId()} setRoute={handleRouteChange}>
      {children}
    </AppShell>
  );
}
