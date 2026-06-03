"use client";

import { useEffect } from "react";
import { useRouter, usePathname } from "next/navigation";
import Box from "@mui/material/Box";
import { useAuth } from "@/hooks/useAuth";
import { Sidebar } from "@/components/Sidebar";

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
    if (pathname.startsWith("/admin/assessments")) return "admin-assessments";
    if (pathname.startsWith("/admin/audit")) return "admin-audit";
    if (pathname.startsWith("/admin/health")) return "admin-health";
    if (pathname.startsWith("/admin")) return "admin-dashboard";
    return "admin-dashboard";
  };

  const handleRouteChange = (route: string) => {
    const routeMap: Record<string, string> = {
      "admin-dashboard": "/admin",
      "admin-users": "/admin/users",
      "admin-assessments": "/admin/assessments",
      "admin-audit": "/admin/audit",
      "admin-health": "/admin/health",
      explore: "/explore",
      assessment: "/practice",
      flashcards: "/flashcards",
      questions: "/questions",
      results: "/results",
      dashboard: "/progress",
      author: "/author",
      grading: "/grading",
      agent: "/agent",
    };
    router.push(routeMap[route] || "/admin");
  };

  if (loading || !user || user.role !== "admin") {
    return null;
  }

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar route={getRouteId()} setRoute={handleRouteChange} />
      <Box component="main" sx={{ flex: 1, overflowY: "auto" }}>
        {children}
      </Box>
    </Box>
  );
}
