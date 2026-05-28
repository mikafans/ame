/** @type {import('next').NextConfig} */

const allowedDevOrigins =
  process.env.NEXT_ALLOWED_ORIGINS?.split(",").filter(Boolean) ?? [];

const apiOrigin = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

// CSP is the main mitigation for the non-HttpOnly `ame_token` cookie. Until
// auth moves server-side we keep the policy tight: no inline scripts beyond
// what Next.js needs, connect only to the API origin we know about.
//
// `'unsafe-inline'` for styles is required by MUI/Emotion in dev. Drop it once
// nonce-based styles are wired (see https://mui.com/material-ui/guides/content-security-policy/).
const csp = [
  "default-src 'self'",
  "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: blob:",
  "font-src 'self' data:",
  `connect-src 'self' ${apiOrigin}`,
  "frame-ancestors 'none'",
  "base-uri 'self'",
  "form-action 'self'",
].join("; ");

const securityHeaders = [
  { key: "Content-Security-Policy", value: csp },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "X-Frame-Options", value: "DENY" },
  {
    key: "Permissions-Policy",
    value: "camera=(), microphone=(), geolocation=()",
  },
];

const nextConfig = {
  reactStrictMode: true,
  output: "standalone",
  async headers() {
    return [{ source: "/(.*)", headers: securityHeaders }];
  },
  ...(allowedDevOrigins.length > 0 && { allowedDevOrigins }),
};

export default nextConfig;
