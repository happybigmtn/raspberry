import { randomBytes } from "node:crypto";

const LOOPBACK_HOSTNAMES = new Set(["localhost", "127.0.0.1", "::1", "0.0.0.0"]);
const SETUP_STATE_COOKIE = "fabro_setup_state";

export function isGitHubLoginAllowed(
  allowedUsernames: string[],
  login: string | null | undefined,
): boolean {
  return (
    typeof login === "string"
    && allowedUsernames.length > 0
    && allowedUsernames.includes(login)
  );
}

export function isLoopbackRequest(request: Request): boolean {
  const { hostname } = new URL(request.url);
  return LOOPBACK_HOSTNAMES.has(hostname);
}

export function shouldUseSecureCookie(request: Request): boolean {
  return new URL(request.url).protocol === "https:";
}

export function generateSetupState(): string {
  return randomBytes(16).toString("hex");
}

export function buildSetupCallbackUrl(baseUrl: string, state: string): string {
  const url = new URL("/setup/callback", baseUrl);
  url.searchParams.set("setup_state", state);
  return url.toString();
}

export function readCookieValue(cookieHeader: string, name: string): string | null {
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = cookieHeader.match(new RegExp(`(?:^|;\\s*)${escapedName}=([^;]+)`));
  return match?.[1] ?? null;
}

export function buildSetupStateCookie(state: string, secure: boolean): string {
  const secureFlag = secure ? "; Secure" : "";
  return `${SETUP_STATE_COOKIE}=${state}; HttpOnly; Path=/; Max-Age=600; SameSite=Lax${secureFlag}`;
}

export function clearSetupStateCookie(secure: boolean): string {
  const secureFlag = secure ? "; Secure" : "";
  return `${SETUP_STATE_COOKIE}=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax${secureFlag}`;
}
