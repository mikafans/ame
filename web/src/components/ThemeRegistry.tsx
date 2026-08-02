"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
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

const DEFAULT_THEME: AmeTheme = "paper-moss";

function isAmeTheme(theme: string | null): theme is AmeTheme {
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

export function useColorMode() {
  return useContext(ColorModeContext);
}

export default function ThemeRegistry({
  children,
}: {
  children: React.ReactNode;
}) {
  const [theme, setStoredTheme] = useState<AmeTheme>(DEFAULT_THEME);

  const setTheme = useCallback((nextTheme: AmeTheme) => {
    setStoredTheme(nextTheme);
    localStorage.setItem("ame.theme", nextTheme);
    document.documentElement.dataset.ameTheme = nextTheme;
  }, []);

  useEffect(() => {
    const savedTheme = localStorage.getItem("ame.theme");
    const initialTheme = isAmeTheme(savedTheme) ? savedTheme : DEFAULT_THEME;
    setStoredTheme(initialTheme);
    document.documentElement.dataset.ameTheme = initialTheme;
  }, []);

  const mode: ColorMode = theme === "night-study" ? "dark" : "light";
  const toggle = useCallback(
    () => setTheme(mode === "dark" ? DEFAULT_THEME : "night-study"),
    [mode, setTheme],
  );
  const value = useMemo(
    () => ({ mode, toggle, theme, setTheme }),
    [mode, setTheme, theme, toggle],
  );

  return (
    <ColorModeContext.Provider value={value}>
      {children}
    </ColorModeContext.Provider>
  );
}
