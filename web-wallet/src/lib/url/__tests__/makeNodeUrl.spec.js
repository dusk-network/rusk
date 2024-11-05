import { beforeEach, describe, expect, it, vi } from "vitest";
import { makeNodeUrl } from "..";

const protocol = "https://";

beforeEach(async () => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

describe("makeNodeUrl", () => {
  const localhostString = window.location.hostname;

  it("should return a local URL when VITE_NODE_URL is not set", () => {
    vi.stubEnv("VITE_NODE_URL", "");
    expect(makeNodeUrl().hostname).toBe(localhostString);
  });

  it("should return a local URL with no base path when VITE_NODE_URL is not set and VITE_RUSK_PATH is not set", () => {
    vi.stubEnv("VITE_NODE_URL", "");
    vi.stubEnv("VITE_RUSK_PATH", "");
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/");
  });

  it("should return a local URL with a base path when `VITE_NODE_URL` is not set and `VITE_RUSK_PATH` is set to a valid string", () => {
    vi.stubEnv("VITE_NODE_URL", "");
    vi.stubEnv("VITE_RUSK_PATH", "/testing");
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/testing");
  });

  it("should return the devnet URL when the hostname starts with 'apps.staging.devnet'", () => {
    const hostname = "apps.staging.devnet.dusk.network";
    vi.stubGlobal("location", { hostname, protocol });
    expect(makeNodeUrl().hostname).toBe("devnet.nodes.dusk.network");
  });

  it("should return the devnet URL when the hostname starts with 'apps.devnet'", () => {
    const hostname = "apps.devnet.dusk.network";
    vi.stubGlobal("location", { hostname, protocol });
    expect(makeNodeUrl().hostname).toBe("devnet.nodes.dusk.network");
  });

  it("should return the testnet URL when the hostname starts with 'apps.staging.testnet'", () => {
    const hostname = "apps.staging.testnet.dusk.network";
    vi.stubGlobal("location", { hostname, protocol });
    expect(makeNodeUrl().hostname).toBe("testnet.nodes.dusk.network");
  });

  it("should return the testnet URL when the hostname starts with 'apps.testnet'", () => {
    const hostname = "apps.testnet.dusk.network";
    vi.stubGlobal("location", { hostname, protocol });
    expect(makeNodeUrl().hostname).toBe("testnet.nodes.dusk.network");
  });

  it("should return the mainnet URL when the hostname starts with 'apps.staging'", () => {
    const hostname = "apps.staging.dusk.network";
    vi.stubGlobal("location", { hostname, protocol });
    expect(makeNodeUrl().hostname).toBe("nodes.dusk.network");
  });

  it("should return the mainnet URL when the hostname starts with 'apps'", () => {
    const hostname = "apps.dusk.network";
    vi.stubGlobal("location", { hostname, protocol });
    expect(makeNodeUrl().hostname).toBe("nodes.dusk.network");
  });
});
