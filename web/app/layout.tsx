import "./globals.css";
import type { Metadata } from "next";
import ThemeRegistry from "@/components/ThemeRegistry";
import { AuthProvider } from "@/hooks/useAuth";

export const metadata: Metadata = {
  title: "ame — study, sweetened",
  description:
    "ame — study, sweetened. One clear next step for what you want to learn.",
  icons: {
    icon: [
      {
        url: "/ame-icon-light.svg",
        type: "image/svg+xml",
        media: "(prefers-color-scheme: light)",
      },
      {
        url: "/ame-icon-dark.svg",
        type: "image/svg+xml",
        media: "(prefers-color-scheme: dark)",
      },
      {
        url: "/ame-icon-16.svg",
        type: "image/svg+xml",
        sizes: "16x16",
      },
    ],
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html: `(() => { try { const theme = localStorage.getItem("ame.theme"); if (["study-atelier", "night-study", "paper-moss", "high-contrast"].includes(theme)) document.documentElement.dataset.ameTheme = theme; else document.documentElement.dataset.ameTheme = "paper-moss"; } catch { document.documentElement.dataset.ameTheme = "paper-moss"; } })();`,
          }}
        />
        <link
          rel="alternate"
          type="text/plain"
          href="/public/llms.txt"
          title="LLM/Agent Documentation"
        />
      </head>
      <body>
        <ThemeRegistry>
          <AuthProvider>{children}</AuthProvider>
        </ThemeRegistry>
      </body>
    </html>
  );
}
