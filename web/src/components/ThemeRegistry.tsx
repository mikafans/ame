"use client";

import { ThemeProvider, useTheme } from "next-themes";
import { createContext, useCallback, useContext } from "react";

type ColorMode = "light" | "dark";

const ColorModeContext = createContext<{
  mode: ColorMode;
  toggle: () => void;
}>({
  mode: "light",
  toggle: () => {},
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
  const mode: ColorMode = theme === "dark" ? "dark" : "light";
  const toggle = useCallback(
    () => setTheme(mode === "dark" ? "light" : "dark"),
    [mode, setTheme],
  );

  return (
    <ColorModeContext.Provider value={{ mode, toggle }}>
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
      attribute="class"
      defaultTheme="light"
      enableSystem={false}
      storageKey="ame.colorMode"
      disableTransitionOnChange
    >
      <ColorModeBridge>{children}</ColorModeBridge>
    </ThemeProvider>
  );
}
