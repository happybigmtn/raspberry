import { describe, expect, test } from "bun:test";

import {
  buildSetupCallbackUrl,
  buildSetupStateCookie,
  clearSetupStateCookie,
  isGitHubLoginAllowed,
  isLoopbackRequest,
  readCookieValue,
} from "./auth-policy.server";

describe("auth-policy.server", () => {
  test("GitHub login policy fails closed on an empty allowlist", () => {
    expect(isGitHubLoginAllowed([], "octocat")).toBeFalse();
    expect(isGitHubLoginAllowed(["octocat"], "octocat")).toBeTrue();
    expect(isGitHubLoginAllowed(["alice"], "octocat")).toBeFalse();
  });

  test("loopback detection only allows local setup hosts", () => {
    expect(isLoopbackRequest(new Request("http://localhost:5173/setup"))).toBeTrue();
    expect(isLoopbackRequest(new Request("http://127.0.0.1:5173/setup"))).toBeTrue();
    expect(isLoopbackRequest(new Request("http://0.0.0.0:5173/setup"))).toBeTrue();
    expect(isLoopbackRequest(new Request("https://fabro.example.com/setup"))).toBeFalse();
  });

  test("setup callback URLs carry the setup state", () => {
    expect(
      buildSetupCallbackUrl("http://localhost:5173", "state123"),
    ).toBe("http://localhost:5173/setup/callback?setup_state=state123");
  });

  test("setup state cookies round-trip and clear cleanly", () => {
    const cookie = buildSetupStateCookie("state123", true);

    expect(cookie).toContain("fabro_setup_state=state123");
    expect(cookie).toContain("HttpOnly");
    expect(cookie).toContain("Secure");
    expect(readCookieValue(cookie, "fabro_setup_state")).toBe("state123");

    const cleared = clearSetupStateCookie(false);
    expect(cleared).toContain("fabro_setup_state=");
    expect(cleared).toContain("Max-Age=0");
  });
});
