// App version, injected at build time from package.json via next.config.mjs.
// Falls back to "dev" when the env var is absent (e.g. ad-hoc tooling).
export const APP_VERSION = process.env.NEXT_PUBLIC_APP_VERSION ?? "dev";
