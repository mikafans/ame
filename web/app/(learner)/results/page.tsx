"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

export default function ResultsRedirectPage() {
  const router = useRouter();

  useEffect(() => {
    try {
      const lastId = localStorage.getItem("ame.lastSessionId");
      if (lastId) {
        router.replace(`/sessions/${lastId}/results`);
      } else {
        router.replace("/library");
      }
    } catch {
      router.replace("/library");
    }
  }, [router]);

  return null;
}
