/** @type {import('next').NextConfig} */

const allowedDevOrigins =
  process.env.NEXT_ALLOWED_ORIGINS?.split(",").filter(Boolean) ?? [];

const nextConfig = {
  reactStrictMode: true,
  output: "standalone",
  ...(allowedDevOrigins.length > 0 && { allowedDevOrigins }),
};

export default nextConfig;
