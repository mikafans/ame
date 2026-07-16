"use client";
/* eslint-disable @next/next/no-img-element -- brand SVG variants are selected by color mode. */

import { useColorMode } from "@/components/ThemeRegistry";
import { BRAND_GRADIENT } from "@/lib/brand";

interface BrandMarkProps {
  size?: number;
}

export function BrandMark({ size = 32 }: BrandMarkProps) {
  const { mode } = useColorMode();
  const mascotSrc =
    mode === "dark" ? "/ame-icon-dark.svg" : "/ame-icon-light.svg";

  return (
    <div data-testid="brand-mark" className="flex items-center gap-3">
      <img
        src={mascotSrc}
        alt=""
        aria-hidden="true"
        data-testid="brand-mascot"
        className="block shrink-0 rounded-lg"
        style={{ width: size, height: size }}
      />
      <span
        data-testid="brand-wordmark"
        className="font-extrabold leading-none tracking-[-0.02em] text-transparent"
        style={{
          fontSize: Math.round(size * 0.75),
          background: BRAND_GRADIENT,
          backgroundClip: "text",
          WebkitBackgroundClip: "text",
        }}
      >
        ame
      </span>
    </div>
  );
}
