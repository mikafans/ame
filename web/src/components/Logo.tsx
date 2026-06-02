"use client";

import React from "react";
import Link from "next/link";
import Typography from "@mui/material/Typography";

interface LogoProps {
  size?: number;
}

export function Logo({ size = 24 }: LogoProps) {
  return (
    <Link
      href="/"
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 9,
        textDecoration: "none",
      }}
    >
      <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
        <rect x="2" y="2" width="28" height="28" rx="6" fill="#1976d2" />
        <path
          d="M9 23 16 9 23 23M12 18h8"
          stroke="#ffffff"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <Typography
        component="span"
        sx={{
          fontSize: 19,
          fontWeight: 600,
          letterSpacing: -0.3,
          color: "text.primary",
        }}
      >
        ame
      </Typography>
    </Link>
  );
}
