import { describe, expect, it } from "vitest";
import { makeNodeUrl } from "..";
// import { JSDOM } from "jsdom";

describe("makeNodeUrl", () => {
  const localhostString = window.location.hostname;

  it("should return a local URL when VITE_NODE_URL is not set", () => {
    delete import.meta.env.VITE_NODE_URL;
    expect(makeNodeUrl().hostname).toBe(localhostString);
  });

  it("should return a local URL when VITE_NODE_URL is an empty string", () => {
    import.meta.env.VITE_NODE_URL = "";
    expect(makeNodeUrl().hostname).toBe(localhostString);
  });

  it("should return a local URL with no base path when VITE_NODE_URL is not set and VITE_RUSK_PATH is not set", () => {
    delete import.meta.env.VITE_NODE_URL;
    delete import.meta.env.VITE_RUSK_PATH;
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/");
  });

  it("should return a local URL with no base path when VITE_NODE_URL is set to an empty string and VITE_RUSK_PATH is not set", () => {
    import.meta.env.VITE_NODE_URL = "";
    delete import.meta.env.VITE_RUSK_PATH;
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/");
  });

  it("should return a local URL with no base path when VITE_NODE_URL is not set and VITE_RUSK_PATH is set to an empty string", () => {
    delete import.meta.env.VITE_NODE_URL;
    import.meta.env.VITE_RUSK_PATH = "";
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/");
  });

  it("should return a local URL with no base path when VITE_NODE_URL is set to an empty string and VITE_RUSK_PATH is set to an empty string", () => {
    import.meta.env.VITE_NODE_URL = "";
    import.meta.env.VITE_RUSK_PATH = "";
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/");
  });

  it("should return a local URL with a base path when `VITE_NODE_URL` is not set and `VITE_RUSK_PATH` is set to a valid string", () => {
    delete import.meta.env.VITE_NODE_URL;
    import.meta.env.VITE_RUSK_PATH = "/testing";
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/testing");
  });

  it("should return a local URL with a base path when `VITE_NODE_URL` is set to an empty string and `VITE_RUSK_PATH` is set to a valid string", () => {
    import.meta.env.VITE_NODE_URL = "";
    import.meta.env.VITE_RUSK_PATH = "/testing";
    expect(makeNodeUrl().hostname).toBe(localhostString);
    expect(makeNodeUrl().pathname).toBe("/testing");
  });

  it("should return the devnet URL when the hostname starts with 'devnet.staging' on the staging URL", () => {
    global.window = Object.create(window);

    const url = new URL("https://devnet.staging.dusk.network");

    Object.defineProperty(window, "location", {
      value: {
        hostname: url.hostname,
        href: url.href,
        protocol: url.protocol,
      },
    });

    expect(makeNodeUrl().hostname).toBe("devnet.nodes.dusk.network");
  });

  it("should return the devnet URL when the hostname starts with 'devnet'", () => {
    global.window = Object.create(window);

    const url = new URL("https://devnet.dusk.network");

    Object.defineProperty(window, "location", {
      value: {
        hostname: url.hostname,
        href: url.href,
        protocol: url.protocol,
      },
    });

    expect(makeNodeUrl().hostname).toBe("devnet.nodes.dusk.network");
  });

  it("should return the testnet URL when the hostname starts with 'testnet.staging' on the staging URL", () => {
    global.window = Object.create(window);

    const url = new URL("https://testnet.staging.dusk.network");

    Object.defineProperty(window, "location", {
      value: {
        hostname: url.hostname,
        href: url.href,
        protocol: url.protocol,
      },
    });

    expect(makeNodeUrl().hostname).toBe("testnet.nodes.dusk.network");
  });

  it("should return the testnet URL when the hostname starts with 'testnet'", () => {
    global.window = Object.create(window);

    const url = new URL("https://testnet.dusk.network");

    Object.defineProperty(window, "location", {
      value: {
        hostname: url.hostname,
        href: url.href,
        protocol: url.protocol,
      },
    });

    expect(makeNodeUrl().hostname).toBe("testnet.nodes.dusk.network");
  });

  it("should return the mainnet URL when the hostname starts with 'apps.staging' on the staging URL", () => {
    global.window = Object.create(window);

    const url = new URL("https://apps.staging.dusk.network");

    Object.defineProperty(window, "location", {
      value: {
        hostname: url.hostname,
        href: url.href,
        protocol: url.protocol,
      },
    });

    expect(makeNodeUrl().hostname).toBe("nodes.dusk.network");
  });

  it("should return the mainnet URL when the hostname starts with 'apps'", () => {
    global.window = Object.create(window);

    const url = new URL("https://apps.dusk.network");

    Object.defineProperty(window, "location", {
      value: {
        hostname: url.hostname,
        href: url.href,
        protocol: url.protocol,
      },
    });

    expect(makeNodeUrl().hostname).toBe("nodes.dusk.network");
  });

  it("should return the mainnet URL when the hostname starts with 'staging.apps'", () => {
    global.window = Object.create(window);

    const url = new URL("https://staging.apps.dusk.network");

    Object.defineProperty(window, "location", {
      value: {
        hostname: url.hostname,
        href: url.href,
        protocol: url.protocol,
      },
    });

    expect(makeNodeUrl().hostname).toBe("nodes.dusk.network");
  });
});
