"use client";

import React, { JSX } from "react";

interface IconProps {
  name: string;
  size?: number;
  color?: string;
}

const icons: Record<
  string,
  (props: React.SVGProps<SVGSVGElement>) => JSX.Element
> = {
  bell: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M13 10v1c0 .5.5 1 1 1H2c.5 0 1-.5 1-1v-1M5 13h6M5.5 1.5c.5 0 1 .5 1 1v3c0 1.5 1 2 1 2s1-.5 1-2v-3c0-.5.5-1 1-1" />
    </svg>
  ),
  search: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <circle cx="6" cy="6" r="4.5" />
      <path d="M10 10l4.5 4.5" />
    </svg>
  ),
  settings: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <circle cx="8" cy="8" r="1.5" />
      <path d="M13.5 8a5.5 5.5 0 0 0-.4-1.8l1-1.3-1.5-1.8-1.3.5c-.5-.4-1.1-.6-1.8-.8V.5h-2v1.3c-.7.2-1.3.4-1.8.8l-1.3-.5-1.5 1.8 1 1.3C2.9 6.2 2.5 7 2.5 8s.4 1.8.6 1.8l-1 1.3 1.5 1.8 1.3-.5c.5.4 1.1.6 1.8.8v1.3h2v-1.3c.7-.2 1.3-.4 1.8-.8l1.3.5 1.5-1.8-1-1.3c.2-.6.4-1.2.4-1.8z" />
    </svg>
  ),
  arrow: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M2 8h12M10 5l3 3-3 3" />
    </svg>
  ),
  sparkle: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M8 2v3M8 11v3M3 8h3M10 8h3M4 4l2 2M10 10l2 2M12 4l-2 2M6 10l-2 2" />
    </svg>
  ),
  check: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M3 8l3 3 7-7" />
    </svg>
  ),
  stack: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M8 2l6 3-6 3-6-3 6-3z" />
      <path d="M2 8l6 3 6-3M2 12l6 3 6-3" />
    </svg>
  ),
  library: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M2 2v12h12V2H2zm2 1v10h2V3H4zm4 0v10h2V3H8zm4 0v10h2V3h-2z" />
    </svg>
  ),
  take: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M3 2l10 6-10 6V2z" />
    </svg>
  ),
  results: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M2 2h12v12H2V2zm2 2v8h2V4H4zm4 0v8h2V4H8zm4 0v8h2V4h-2z" />
    </svg>
  ),
  dashboard: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <rect x="2" y="2" width="5" height="5" />
      <rect x="9" y="2" width="5" height="5" />
      <rect x="2" y="9" width="5" height="5" />
      <rect x="9" y="9" width="5" height="5" />
    </svg>
  ),
  author: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M3 13h3l7-7-3-3-7 7v3z" />
      <path d="M10 4l2 2" />
    </svg>
  ),
  agent: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <rect x="1" y="4" width="14" height="9" rx="1" />
      <path d="M5 4V2M11 4V2M1 8h14" />
      <circle cx="5" cy="11" r="0.8" />
      <circle cx="11" cy="11" r="0.8" />
    </svg>
  ),
  flag: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M3 2v12M14 3l-8 2-8-2v6l8 2 8-2" />
    </svg>
  ),
  code: (p) => (
    <svg {...p} viewBox="0 0 16 16">
      <path d="M5 3l-3 5 3 5M11 3l3 5-3 5" />
    </svg>
  ),
};

export function Icon({ name, size = 16, color = "currentColor" }: IconProps) {
  const Icon = icons[name];
  if (!Icon) return null;

  return (
    <Icon
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke={color}
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  );
}
