"use client";

import { Palette } from "lucide-react";
import { AME_THEMES, useColorMode } from "@/components/ThemeRegistry";

interface ThemeSelectorProps {
  className?: string;
}

export function ThemeSelector({ className = "" }: ThemeSelectorProps) {
  const { setTheme, theme } = useColorMode();

  return (
    <label className={`inline-flex items-center gap-2 ${className}`}>
      <Palette aria-hidden="true" className="size-4 text-muted-foreground" />
      <span className="sr-only">Color theme</span>
      <select
        aria-label="Color theme"
        value={theme}
        onChange={(event) => setTheme(event.target.value as typeof theme)}
        className="h-8 max-w-36 rounded-md border border-border bg-background px-2 text-xs text-foreground outline-none transition hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
      >
        {AME_THEMES.map((option) => (
          <option key={option.id} value={option.id}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}
