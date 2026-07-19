"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
} from "react";

type ColorMode = "light" | "dark";
const ColorModeContext = createContext<{ mode: ColorMode; toggle: () => void }>(
  { mode: "light", toggle: () => {} },
);
export function useColorMode() {
  return useContext(ColorModeContext);
}

export default function ThemeRegistry({
  children,
}: {
  children: React.ReactNode;
}) {
  const [mode, setMode] = useState<ColorMode>("light");
  useEffect(() => {
    try {
      const saved = localStorage.getItem("ame.colorMode") as ColorMode | null;
      if (saved === "dark" || saved === "light") setMode(saved);
    } catch {
      /* ignore */
    }
  }, []);
  useEffect(() => {
    document.documentElement.classList.toggle("dark", mode === "dark");
  }, [mode]);
  const toggle = useCallback(
    () =>
      setMode((current) => {
        const next = current === "light" ? "dark" : "light";
        try {
          localStorage.setItem("ame.colorMode", next);
        } catch {
          /* ignore */
        }
        return next;
      }),
    [],
  );
  return (
    <ColorModeContext.Provider value={{ mode, toggle }}>
      {children}
    </ColorModeContext.Provider>
  );
}
