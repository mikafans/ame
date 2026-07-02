"use client";

// NOTE: The `(learner)` directory name is a legacy label. It hosts both learner and authoring features now.
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
    if (pathname.startsWith("/admin/assessments")) return "admin-assessments";
    if (pathname.startsWith("/admin/tokens")) return "admin-tokens";
    if (pathname.startsWith("/admin/audit")) return "admin-audit";
    if (pathname.startsWith("/admin/health")) return "admin-health";
    if (pathname.startsWith("/admin")) return "admin-dashboard";
    if (pathname.startsWith("/explore")) return "explore";
    if (pathname.startsWith("/exams")) return "exams";
    if (pathname.startsWith("/practice")) return "assessment";
    if (pathname.startsWith("/flashcards")) return "flashcards";
    if (pathname.startsWith("/questions")) return "questions";
    if (pathname.startsWith("/deep-dives")) return "deep-dives";
    if (pathname.startsWith("/results")) return "results";
    if (pathname.startsWith("/progress")) return "dashboard";
    if (pathname.startsWith("/author")) return "author";
    if (pathname.startsWith("/grading")) return "grading";
    if (pathname.startsWith("/agent")) return "agent";
    return "explore";
  };

  const handleRouteChange = (route: string) => {
    const routeMap: Record<string, string> = {
      "admin-dashboard": "/admin",
      "admin-users": "/admin/users",
      "admin-assessments": "/admin/assessments",
      "admin-tokens": "/admin/tokens",
      "admin-audit": "/admin/audit",
      "admin-health": "/admin/health",
      explore: "/explore",
      exams: "/exams",
      assessment: "/practice",
      flashcards: "/flashcards",
      questions: "/questions",
      "deep-dives": "/deep-dives",
      results: "/results",
      dashboard: "/progress",
      author: "/author",
      grading: "/grading",
      agent: "/agent",
    };
    router.push(routeMap[route] || "/explore");
  };

  return (
    <AppShell route={getRouteId()} setRoute={handleRouteChange}>
      {children}
    </AppShell>
  );
}
