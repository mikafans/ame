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
  const isDark = mode === "dark";
  return createTheme({
    palette: {
      mode,
      primary: {
        main: isDark ? "#62D8CD" : "#45C4B9",
        light: isDark ? "#84E2D9" : "#6FDACF",
        dark: isDark ? "#2FA79E" : "#1F766F",
        contrastText: "#0C1817",
      },
      secondary: {
        main: isDark ? "#FF9DC0" : "#FF8FB4",
        light: isDark ? "#F7A9C4" : "#FBB2CB",
        dark: isDark ? "#E978A2" : "#D95F8D",
        contrastText: "#0C1817",
      },
      error: { main: isDark ? "#f87171" : "#dc2626" },
      warning: { main: isDark ? "#fbbf24" : "#d97706" },
      success: { main: isDark ? "#62D8CD" : "#1F766F" },
      background: isDark
        ? { default: "#101E1C", paper: "#1B2A28" }
        : { default: "#F3FCFB", paper: "#ffffff" },
      divider: isDark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.07)",
      text: isDark
        ? { primary: "#e2e8f0", secondary: "#94a3b8", disabled: "#475569" }
        : { primary: "#0f172a", secondary: "#475569", disabled: "#94a3b8" },
    },
    typography: {
      fontFamily:
        '"Inter", "Roboto", "PingFang SC", "Microsoft YaHei", "Noto Sans SC", "Noto Sans CJK SC", sans-serif',
      fontSize: 14,
      fontWeightRegular: 400,
      fontWeightMedium: 500,
      fontWeightBold: 700,
      h4: { fontWeight: 700, letterSpacing: "-0.01em" },
      h5: { fontWeight: 600, letterSpacing: "-0.005em" },
      h6: { fontWeight: 600 },
      subtitle1: { fontWeight: 500 },
      subtitle2: { fontWeight: 600, letterSpacing: "0.01em" },
      body2: { lineHeight: 1.6 },
      overline: {
        fontWeight: 600,
        letterSpacing: "0.08em",
        fontSize: "0.7rem",
      },
      button: {
        textTransform: "none",
        fontWeight: 500,
        letterSpacing: "0.01em",
      },
      caption: { lineHeight: 1.5 },
    },
    shape: { borderRadius: 10 },
    shadows: [
      "none",
      isDark
        ? "0 1px 3px rgba(0,0,0,0.5)"
        : "0 1px 3px rgba(0,0,0,0.06), 0 1px 2px rgba(0,0,0,0.04)",
      isDark
        ? "0 2px 6px rgba(0,0,0,0.5)"
        : "0 2px 8px rgba(0,0,0,0.06), 0 1px 3px rgba(0,0,0,0.04)",
      isDark
        ? "0 4px 16px rgba(0,0,0,0.5)"
        : "0 4px 16px rgba(0,0,0,0.06), 0 2px 6px rgba(0,0,0,0.04)",
      isDark
        ? "0 8px 24px rgba(0,0,0,0.5)"
        : "0 8px 24px rgba(0,0,0,0.07), 0 4px 8px rgba(0,0,0,0.04)",
      ...Array(20).fill(
        isDark ? "0 12px 40px rgba(0,0,0,0.6)" : "0 12px 40px rgba(0,0,0,0.08)",
      ),
    ] as any,
    components: {
      MuiCssBaseline: {
        styleOverrides: `
          *, *::before, *::after { box-sizing: border-box; }
          body { -webkit-font-smoothing: antialiased; }
          ::-webkit-scrollbar { width: 6px; height: 6px; }
          ::-webkit-scrollbar-track { background: transparent; }
          ::-webkit-scrollbar-thumb { background: ${isDark ? "rgba(255,255,255,0.15)" : "rgba(0,0,0,0.15)"}; border-radius: 6px; }
          ::-webkit-scrollbar-thumb:hover { background: ${isDark ? "rgba(255,255,255,0.25)" : "rgba(0,0,0,0.25)"}; }
        `,
      },
      MuiButton: {
        defaultProps: { disableElevation: true },
        styleOverrides: {
          root: { borderRadius: 8, fontWeight: 500 },
          sizeSmall: { padding: "5px 14px", minHeight: 32, fontSize: 13 },
          sizeMedium: { padding: "7px 18px", minHeight: 38 },
          sizeLarge: { padding: "10px 24px", minHeight: 46, fontSize: 15 },
          containedPrimary: {
            background: isDark
              ? "linear-gradient(135deg, #2FA79E 0%, #62D8CD 100%)"
              : "linear-gradient(135deg, #2FA79E 0%, #45C4B9 100%)",
            "&:hover": {
              background: isDark
                ? "linear-gradient(135deg, #45C4B9 0%, #84E2D9 100%)"
                : "linear-gradient(135deg, #248E87 0%, #2FA79E 100%)",
            },
          },
        },
      },
      MuiIconButton: {
        styleOverrides: {
          root: {
            borderRadius: 8,
            transition: "background 0.15s ease",
          },
          sizeSmall: { padding: 6 },
          sizeMedium: { padding: 8 },
        },
      },
      MuiCard: {
        defaultProps: { elevation: 0 },
        styleOverrides: {
          root: {
            borderRadius: 12,
            border: `1px solid ${isDark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.07)"}`,
            backgroundImage: "none",
            transition: "box-shadow 0.2s ease, transform 0.2s ease",
          },
        },
      },
      MuiPaper: {
        defaultProps: { elevation: 0 },
        styleOverrides: {
          root: { backgroundImage: "none" },
          outlined: {
            borderColor: isDark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.07)",
          },
        },
      },
      MuiChip: {
        styleOverrides: {
          root: { borderRadius: 8, fontWeight: 500 },
          sizeMedium: { height: 34, fontSize: 13 },
          sizeSmall: { height: 24, fontSize: 11 },
          outlined: {
            borderColor: isDark ? "rgba(255,255,255,0.15)" : "rgba(0,0,0,0.12)",
          },
        },
      },
      MuiTextField: {
        defaultProps: { variant: "outlined" as const },
        styleOverrides: {
          root: {
            "& .MuiOutlinedInput-root": {
              borderRadius: 8,
              transition: "box-shadow 0.15s ease",
              "&.Mui-focused": {
                boxShadow: isDark
                  ? "0 0 0 3px rgba(98,216,205,0.2)"
                  : "0 0 0 3px rgba(47,167,158,0.12)",
              },
            },
          },
        },
      },
      MuiOutlinedInput: {
        styleOverrides: {
          root: {
            borderRadius: 8,
            "& fieldset": {
              borderColor: isDark
                ? "rgba(255,255,255,0.15)"
                : "rgba(0,0,0,0.15)",
              transition: "border-color 0.15s ease",
            },
          },
        },
      },
      MuiTableContainer: {
        styleOverrides: {
          root: {
            overflow: "auto",
            borderRadius: 12,
            border: `1px solid ${isDark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.07)"}`,
            backgroundColor: isDark ? "#1B2A28" : "#ffffff",
          },
        },
      },
      MuiTableHead: {
        styleOverrides: {
          root: {
            "& .MuiTableCell-root": {
              fontWeight: 600,
              fontSize: 12,
              letterSpacing: "0.04em",
              textTransform: "uppercase",
              color: isDark ? "#94a3b8" : "#64748b",
              borderBottom: `1px solid ${isDark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.07)"}`,
              backgroundColor: isDark
                ? "rgba(255,255,255,0.02)"
                : "rgba(0,0,0,0.015)",
            },
          },
        },
      },
      MuiTableCell: {
        styleOverrides: {
          root: {
            borderBottom: `1px solid ${isDark ? "rgba(255,255,255,0.05)" : "rgba(0,0,0,0.05)"}`,
            padding: "12px 16px",
          },
        },
      },
      MuiTableRow: {
        styleOverrides: {
          root: {
            transition: "background 0.12s ease",
            "&:last-child td": { borderBottom: 0 },
          },
        },
      },
      MuiListItemButton: {
        styleOverrides: {
          root: {
            borderRadius: 8,
            transition: "background 0.12s ease",
            "&.Mui-selected": {
              backgroundColor: isDark
                ? "rgba(98,216,205,0.15)"
                : "rgba(47,167,158,0.08)",
              color: isDark ? "#62D8CD" : "#1F766F",
              "& .MuiListItemIcon-root": {
                color: isDark ? "#62D8CD" : "#1F766F",
              },
              "&:hover": {
                backgroundColor: isDark
                  ? "rgba(98,216,205,0.22)"
                  : "rgba(47,167,158,0.13)",
              },
            },
          },
        },
      },
      MuiToggleButton: {
        styleOverrides: {
          root: {
            borderRadius: 8,
            textTransform: "none",
            fontWeight: 500,
            fontSize: 13,
            "&.Mui-selected": {
              color: isDark ? "#62D8CD" : "#1F766F",
              backgroundColor: isDark
                ? "rgba(98,216,205,0.15)"
                : "rgba(47,167,158,0.08)",
            },
          },
        },
      },
      MuiAlert: {
        styleOverrides: {
          root: { borderRadius: 10 },
          outlined: {
            borderWidth: 1,
          },
        },
      },
      MuiDialog: {
        styleOverrides: {
          paper: {
            borderRadius: 16,
            border: `1px solid ${isDark ? "rgba(255,255,255,0.08)" : "rgba(0,0,0,0.06)"}`,
          },
        },
      },
      MuiDrawer: {
        styleOverrides: {
          paper: {
            borderRight: `1px solid ${isDark ? "rgba(255,255,255,0.07)" : "rgba(0,0,0,0.07)"}`,
            backgroundColor: isDark ? "#13161f" : "#ffffff",
          },
        },
      },
      MuiDivider: {
        styleOverrides: {
          root: {
            borderColor: isDark ? "rgba(255,255,255,0.07)" : "rgba(0,0,0,0.06)",
          },
        },
      },
      MuiTooltip: {
        styleOverrides: {
          tooltip: {
            borderRadius: 6,
            fontSize: 12,
            backgroundColor: isDark ? "#334155" : "#1e293b",
          },
        },
      },
      MuiTablePagination: {
        styleOverrides: {
          root: { fontSize: 13 },
          selectLabel: { fontSize: 13 },
          displayedRows: { fontSize: 13 },
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
