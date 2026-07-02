export const BRAND = {
  light: {
    primary: "#45C4B9",
    primaryLight: "#6FDACF",
    primaryDark: "#1F766F",
    secondary: "#FF8FB4",
    secondaryLight: "#FBB2CB",
    secondaryDark: "#D95F8D",
    background: "#F3FCFB",
    surface: "#ffffff",
    text: "#0F172A",
  },
  dark: {
    primary: "#62D8CD",
    primaryLight: "#84E2D9",
    primaryDark: "#2FA79E",
    secondary: "#FF9DC0",
    secondaryLight: "#F7A9C4",
    secondaryDark: "#E978A2",
    background: "#101E1C",
    surface: "#1B2A28",
    text: "#E2E8F0",
  },
} as const;

export const BRAND_GRADIENT = `linear-gradient(120deg, ${BRAND.light.primary}, ${BRAND.light.secondary})`;

export const BRAND_TAG_HUES = [174, 337, 164, 28, 205, 145] as const;
