"use client";

import React from "react";
import { Bell, Search } from "lucide-react";
import { BrandMark } from "@/components/BrandMark";

interface TopbarProps {
  title: string;
  subtitle?: string;
  breadcrumb?: string;
  actions?: React.ReactNode;
}

export function Topbar({ title, subtitle, breadcrumb, actions }: TopbarProps) {
  return (
    <header className="sticky top-0 z-10 flex items-center justify-between border-b border-border bg-background px-6 py-5">
      <div className="min-w-0">
        <div className="mb-4">
          <BrandMark />
        </div>
        {breadcrumb && (
          <p className="mb-2 block text-xs uppercase tracking-[0.13em] text-muted-foreground">
            {breadcrumb}
          </p>
        )}
        <h1 className="text-xl font-medium tracking-tight">{title}</h1>
        {subtitle && (
          <p className="mt-1 text-sm text-muted-foreground">{subtitle}</p>
        )}
      </div>

      <div className="flex items-center gap-3">
        {actions}
        <div className="flex items-center gap-3 border-l border-border pl-4 text-muted-foreground">
          <Bell className="size-[18px]" />
          <Search className="size-[18px]" />
        </div>
      </div>
    </header>
  );
}
