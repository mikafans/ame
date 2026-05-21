import createClient from "openapi-fetch";
import type { paths } from "./generated/schema.d.ts";

function getBaseUrl(): string {
  if (typeof window === "undefined") {
    // server-side
    return process.env.API_URL ?? "http://localhost:8080";
  }
  return process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
}

function getBearerToken(): string | undefined {
  if (typeof window === "undefined") return undefined;
  // Cookie read happens server-side in route handlers; client reads from meta tag set by layout
  return (document.cookie.match(/(?:^|;\s*)ame_token=([^;]+)/) ?? [])[1];
}

export function makeClient(bearerToken?: string) {
  return createClient<paths>({
    baseUrl: getBaseUrl(),
    headers: bearerToken
      ? { Authorization: `Bearer ${bearerToken}` }
      : getBearerToken()
        ? { Authorization: `Bearer ${getBearerToken()}` }
        : {},
  });
}

// Default singleton client (browser only — picks up token from cookie)
export const api = makeClient();
