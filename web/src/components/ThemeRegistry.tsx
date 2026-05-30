"use client";

import "@fontsource/roboto/300.css";
import "@fontsource/roboto/400.css";
import "@fontsource/roboto/500.css";
import "@fontsource/roboto/700.css";
import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
} from "react";
import { createTheme, ThemeProvider } from "@mui/material/styles";
import CssBaseline from "@mui/material/CssBaseline";

type ColorMode = "light" | "dark";

const ColorModeContext = createContext<{
  mode: ColorMode;
  toggle: () => void;
}>({ mode: "light", toggle: () => {} });

export function useColorMode() {
  return useContext(ColorModeContext);
}

function buildTheme(mode: ColorMode) {
  return createTheme({
    palette: {
      mode,
      primary: { main: "#1976d2" },
      ...(mode === "dark" && {
        background: { default: "#0d1117", paper: "#161b22" },
      }),
    },
    typography: {
      fontFamily: "Roboto, sans-serif",
      fontSize: 14,
      button: { textTransform: "none", fontWeight: 500, letterSpacing: 0 },
    },
    shape: { borderRadius: 8 },
    components: {
      // Comfortable, consistent buttons platform-wide: no shouty all-caps,
      // flat surfaces, and a little more breathing room than MUI's defaults.
      MuiButton: {
        defaultProps: { disableElevation: true },
        styleOverrides: {
          root: { borderRadius: 8 },
          sizeSmall: { padding: "5px 14px", minHeight: 34 },
          sizeMedium: { padding: "8px 20px", minHeight: 42 },
          sizeLarge: { padding: "11px 26px", minHeight: 50, fontSize: 15 },
        },
      },
      MuiIconButton: {
        styleOverrides: {
          sizeSmall: { padding: 7 },
          sizeMedium: { padding: 9 },
        },
      },
    },
  });
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

  const toggle = useCallback(() => {
    setMode((m) => {
      const next = m === "light" ? "dark" : "light";
      try {
        localStorage.setItem("ame.colorMode", next);
      } catch {
        /* ignore */
      }
      return next;
    });
  }, []);

  return (
    <ColorModeContext.Provider value={{ mode, toggle }}>
      <ThemeProvider theme={buildTheme(mode)}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </ColorModeContext.Provider>
  );
}
