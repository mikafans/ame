import createClient from "openapi-fetch";
import type { paths } from "./generated/schema.d.ts";

function getBaseUrl(): string {
  if (typeof window === "undefined") {
    // server-side
    return process.env.API_URL ?? "http://localhost:28080";
  }
  return (
    process.env.NEXT_PUBLIC_API_URL ??
    `http://${window.location.hostname}:28080`
  );
}

export function makeClient() {
  return createClient<paths>({
    baseUrl: getBaseUrl(),
    credentials: "include",
  });
}

// Default singleton client (browser only — uses HttpOnly cookie)
export const api = makeClient();
