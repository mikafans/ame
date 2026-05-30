"use client";

import { useEffect } from "react";
import { useRouter, usePathname } from "next/navigation";
import Box from "@mui/material/Box";
import { useAuth } from "@/hooks/useAuth";
import { Sidebar } from "@/components/Sidebar";

export default function LearnerLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const { user, loading } = useAuth();
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    if (!loading && !user) router.push("/login");
  }, [loading, user, router]);

  const role = user?.role ?? "learner";

  const getRouteId = () => {
    if (pathname.startsWith("/library")) return "library";
    if (pathname.startsWith("/exams")) return "exams";
    if (pathname.startsWith("/practice")) return "quiz";
    if (pathname.startsWith("/flashcards")) return "flashcards";
    if (pathname.startsWith("/questions")) return "questions";
    if (pathname.startsWith("/results")) return "results";
    if (pathname.startsWith("/progress")) return "dashboard";
    if (pathname.startsWith("/author")) return "author";
    if (pathname.startsWith("/grading")) return "grading";
    if (pathname.startsWith("/agent")) return "agent";
    return "library";
  };

  const handleRouteChange = (route: string) => {
    const routeMap: Record<string, string> = {
      library: "/library",
      exams: "/exams",
      quiz: "/practice",
      flashcards: "/flashcards",
      questions: "/questions",
      results: "/results",
      dashboard: "/progress",
      author: "/author",
      grading: "/grading",
      agent: "/agent",
    };
    router.push(routeMap[route] || "/library");
  };

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Sidebar
        route={getRouteId()}
        setRoute={handleRouteChange}
        showAgent={role !== "learner"}
      />
      <Box component="main" sx={{ flex: 1, overflowY: "auto" }}>
        {children}
      </Box>
    </Box>
  );
}
