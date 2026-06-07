import "./globals.css";
import type { Metadata } from "next";
import { AppRouterCacheProvider } from "@mui/material-nextjs/v16-appRouter";
import ThemeRegistry from "@/components/ThemeRegistry";
import { AuthProvider } from "@/hooks/useAuth";

export const metadata: Metadata = {
  title: "ame",
  description: "Question collector + exam platform",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <link
          rel="alternate"
          type="text/plain"
          href="/llms.txt"
          title="LLM/Agent Documentation"
        />
      </head>
      <body>
        <AppRouterCacheProvider options={{ key: "mui" }}>
          <ThemeRegistry>
            <AuthProvider>{children}</AuthProvider>
          </ThemeRegistry>
        </AppRouterCacheProvider>
      </body>
    </html>
  );
}
