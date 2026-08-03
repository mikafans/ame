"use client";

import { useEffect } from "react";
import { useRouter, usePathname } from "next/navigation";
import { useAuth } from "@/hooks/useAuth";
import { AppShell } from "@/components/AppShell";

export default function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { user, loading } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    if (!loading) {
      if (!user) {
        window.location.href = "/login";
      } else if (user.role !== "admin") {
        router.push("/");
      }
    }
  }, [loading, user, router]);

  const getRouteId = () => {
    if (pathname.startsWith("/admin/users")) return "admin-users";
    if (pathname.startsWith("/admin/audit")) return "admin-audit";
    if (pathname.startsWith("/admin/health")) return "admin-health";
    if (pathname.startsWith("/admin/settings")) return "admin-settings";
    if (pathname.startsWith("/admin/tasks")) return "admin-tasks";
    if (pathname.startsWith("/admin")) return "admin-dashboard";
    return "admin-dashboard";
  };

  const handleRouteChange = (route: string) => {
    const routeMap: Record<string, string> = {
      learning: "/learning",
      about: "/about",
      "admin-dashboard": "/admin",
      "admin-users": "/admin/users",
      "admin-audit": "/admin/audit",
      "admin-health": "/admin/health",
      "admin-settings": "/admin/settings",
      "admin-tasks": "/admin/tasks",
    };
    router.push(routeMap[route] || "/admin");
  };

  if (loading || !user || user.role !== "admin") {
    return null;
  }

  return (
    <AppShell route={getRouteId()} setRoute={handleRouteChange}>
      {children}
    </AppShell>
  );
}
