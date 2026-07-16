/**
 * PageShell — unified page layout wrapper used by all learner & admin pages.
 *
 * Provides:
 *  - Consistent horizontal/vertical padding (48px sides, 40px top, 64px bottom)
 *  - Standard kicker + title + optional subtitle header pattern
 *  - Optional right-hand action slot (e.g. a button)
 *  - Subtle fade-in-up entrance animation
 */
import React from "react";

interface PageShellProps {
  /** Small uppercase label above the title */
  kicker?: string;
  /** Main page title */
  title: string;
  /** Optional subtitle below the title */
  subtitle?: string;
  /** Optional element placed in the top-right of the header row */
  action?: React.ReactNode;
  /** Max width for the content area (default: 900) */
  maxWidth?: number | string;
  children: React.ReactNode;
}

export function PageShell({
  kicker,
  title,
  subtitle,
  action,
  maxWidth = "100%",
  children,
}: PageShellProps) {
  return (
    <div
      className="animate-[fadeUp_0.22s_ease-out_both] px-6 pb-16 pt-10 sm:px-10"
      style={{ maxWidth, marginInline: "auto" }}
    >
      {/* Header */}
      <div className="mb-8 flex flex-col justify-between gap-2 sm:flex-row sm:items-start">
        <div>
          {kicker && (
            <p className="mb-1 block text-xs uppercase tracking-[0.15em] text-muted-foreground">
              {kicker}
            </p>
          )}
          <h1 className="text-xl font-semibold leading-tight">{title}</h1>
          {subtitle && (
            <p className="mt-1 max-w-[540px] text-sm text-muted-foreground">
              {subtitle}
            </p>
          )}
        </div>
        {action && <div className="mt-0.5 shrink-0 sm:ml-2">{action}</div>}
      </div>

      {children}
    </div>
  );
}
