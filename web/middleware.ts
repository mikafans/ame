import { NextRequest, NextResponse } from "next/server";

// Server-side auth gate. Runs on the web origin before the page renders, so an
// unauthenticated hit on an app route redirects to /login *without* the
// client-side flash the `(learner)` layout would otherwise show.
//
// This only checks for the *presence* of the HttpOnly `ame_token` cookie — the
// token is opaque here and cannot be verified without the API. Real enforcement
// still happens on every API request, and the `(learner)` layout keeps its
// useAuth gate to catch a present-but-expired/invalid token (it calls /v1/me
// and redirects on 401). This middleware is purely a UX fast-path.

// Public routes that never require a token. Everything else under the matcher
// is treated as an authenticated app route.
const PUBLIC_PATHS = new Set(["/", "/login"]);

export function middleware(req: NextRequest) {
  const { pathname } = req.nextUrl;
  const hasToken = req.cookies.has("ame_token");
  const isPublic = PUBLIC_PATHS.has(pathname);

  // Already signed in but sitting on /login → send them into the app.
  if (hasToken && pathname === "/login") {
    return NextResponse.redirect(new URL("/library", req.url));
  }

  // Protected route without a token → bounce to /login, remembering where they
  // were headed so login can return them after authenticating.
  if (!isPublic && !hasToken) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("returnTo", pathname + req.nextUrl.search);
    return NextResponse.redirect(loginUrl);
  }

  return NextResponse.next();
}

// Skip Next internals, static assets, and the API-proxied paths
// (/llms.txt, /v1/*) so only real app pages hit the gate.
export const config = {
  matcher: ["/((?!_next/|v1/|llms.txt|favicon.ico|.*\\.[\\w]+$).*)"],
};
