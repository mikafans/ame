import "./globals.css";
import type { Metadata } from "next";
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
      <body>
        <ThemeRegistry>
          <AuthProvider>{children}</AuthProvider>
        </ThemeRegistry>
      </body>
    </html>
  );
}
