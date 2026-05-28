"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Snackbar from "@mui/material/Snackbar";
import Alert from "@mui/material/Alert";

export default function ResultsRedirectPage() {
  const router = useRouter();
  const [noSession, setNoSession] = useState(false);

  useEffect(() => {
    try {
      const lastId = localStorage.getItem("ame.lastSessionId");
      if (lastId) {
        router.replace(`/sessions/${lastId}/results`);
      } else {
        setNoSession(true);
        const t = setTimeout(() => router.replace("/library"), 2500);
        return () => clearTimeout(t);
      }
    } catch {
      router.replace("/library");
    }
  }, [router]);

  return (
    <Snackbar
      open={noSession}
      anchorOrigin={{ vertical: "top", horizontal: "center" }}
    >
      <Alert severity="info" variant="filled">
        No recent session found — redirecting to Library…
      </Alert>
    </Snackbar>
  );
}
