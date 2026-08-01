"use client";

import { ThemeProvider, useTheme } from "next-themes";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
} from "react";

type ColorMode = "light" | "dark";

export const AME_THEMES = [
  {
    id: "study-atelier",
    label: "Study Atelier",
    description: "Cool porcelain surfaces with an ink reading field.",
  },
  {
    id: "night-study",
    label: "Night Study",
    description: "Low-light ink surfaces for focused evening work.",
  },
  {
    id: "paper-moss",
    label: "Paper & Moss",
    description: "Warm paper surfaces with restrained botanical accents.",
  },
  {
    id: "high-contrast",
    label: "High Contrast",
    description: "Maximum separation for reading and keyboard navigation.",
  },
] as const;

export type AmeTheme = (typeof AME_THEMES)[number]["id"];

const DEFAULT_THEME: AmeTheme = "study-atelier";

function isAmeTheme(theme: string | undefined): theme is AmeTheme {
  return AME_THEMES.some((candidate) => candidate.id === theme);
}

const ColorModeContext = createContext<{
  mode: ColorMode;
  toggle: () => void;
  theme: AmeTheme;
  setTheme: (theme: AmeTheme) => void;
}>({
  mode: "light",
  toggle: () => {},
  theme: DEFAULT_THEME,
  setTheme: () => {},
});

/**
 * Compatibility wrapper for existing AME consumers.
 * next-themes owns persistence, SSR-safe class switching, and system behavior;
 * consumers only need AME's small mode/toggle contract.
 */
export function useColorMode() {
  return useContext(ColorModeContext);
}

function ColorModeBridge({ children }: { children: React.ReactNode }) {
  const { theme, setTheme } = useTheme();
  const selectedTheme = isAmeTheme(theme) ? theme : DEFAULT_THEME;
  const mode: ColorMode = selectedTheme === "night-study" ? "dark" : "light";

  useEffect(() => {
    if (
      theme !== undefined ||
      localStorage.getItem("ame.colorMode") !== "dark"
    ) {
      return;
    }
    setTheme("night-study");
  }, [setTheme, theme]);

  const toggle = useCallback(
    () => setTheme(mode === "dark" ? DEFAULT_THEME : "night-study"),
    [mode, setTheme],
  );
  const value = useMemo(
    () => ({
      mode,
      toggle,
      theme: selectedTheme,
      setTheme: (nextTheme: AmeTheme) => setTheme(nextTheme),
    }),
    [mode, selectedTheme, setTheme, toggle],
  );

  return (
    <ColorModeContext.Provider value={value}>
      {children}
    </ColorModeContext.Provider>
  );
}

export default function ThemeRegistry({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <ThemeProvider
      attribute="data-ame-theme"
      defaultTheme={DEFAULT_THEME}
      enableSystem={false}
      storageKey="ame.theme"
      themes={AME_THEMES.map((theme) => theme.id)}
      disableTransitionOnChange
    >
      <ColorModeBridge>{children}</ColorModeBridge>
    </ThemeProvider>
  );
}
