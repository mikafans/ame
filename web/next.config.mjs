/** @type {import('next').NextConfig} */

const allowedDevOrigins = [
  "harus-mini",
  ...(process.env.NEXT_ALLOWED_ORIGINS?.split(",").filter(Boolean) ?? []),
];

const apiOrigin = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:28080";

// Add both apiOrigin and any host in allowedDevOrigins to connect-src.
// We map hosts to http/ws equivalents for dev.
const extraConnectSrc = allowedDevOrigins
  .map((host) => `http://${host}:* ws://${host}:*`)
  .join(" ");

// CSP provides defense-in-depth alongside our HttpOnly `ame_token` cookie.
// We keep the policy tight: no inline scripts beyond what Next.js needs,
// connect only to the API origin we know about.
//
// `'unsafe-inline'` for styles is required by MUI/Emotion in dev. Drop it once
// nonce-based styles are wired (see https://mui.com/material-ui/guides/content-security-policy/).
const csp = [
  "default-src 'self'",
  "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
  "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
  "img-src 'self' data: blob:",
  "font-src 'self' data: https://fonts.gstatic.com",
  `connect-src 'self' ${apiOrigin} ${extraConnectSrc}`,
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
  async rewrites() {
    return [
      {
        source: "/llms.txt",
        destination: `${apiOrigin}/llms.txt`,
      },
      {
        source: "/skill.json",
        destination: `${apiOrigin}/skill.json`,
      },
      {
        source: "/openapi.yaml",
        destination: `${apiOrigin}/openapi.yaml`,
      },
    ];
  },
  ...(allowedDevOrigins.length > 0 && { allowedDevOrigins }),
};

export default nextConfig;
