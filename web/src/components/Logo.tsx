"use client";

import React from "react";
import Link from "next/link";
import { BrandMark } from "@/components/BrandMark";

interface LogoProps {
  size?: number;
}

export function Logo({ size = 24 }: LogoProps) {
  return (
    <Link href="/" className="inline-flex items-center gap-2 no-underline">
      <BrandMark size={size} />
    </Link>
  );
}
