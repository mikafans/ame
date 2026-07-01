"use client";

import React from "react";
import Link from "next/link";
import { BrandMark } from "@/components/BrandMark";

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
      <BrandMark size={size} />
    </Link>
  );
}
