/**
 * @see https://github.com/davipon/svelte-component-test-recipes
 */

import * as matchers from "@testing-library/jest-dom/matchers";
import { expect, vi } from "vitest";
import { readable } from "svelte/store";
import { ResizeObserver } from "@juggle/resize-observer";
import crypto from "node:crypto";
import "jsdom-worker";
import "vitest-canvas-mock";

// adds in-memory replacement for IndexedDB
import "fake-indexeddb/auto";

import { IntersectionObserver } from "./src/lib/dusk/mocks";
import Wallet from "./__mocks__/Wallet.js";

// Mocking wallet connection modules
vi.mock("@wagmi/core");
vi.mock("@web3modal/wagmi");

// Removing the console logging created by the walletConnect library after each test file
Object.defineProperty(window, "litIssuedWarnings", {
  value: new Set([
    "Lit is in dev mode. Not recommended for production! See https://lit.dev/msg/dev-mode for more information.",
    "Multiple versions of Lit loaded. Loading multiple versions is not recommended. See https://lit.dev/msg/multiple-versions for more information.",
  ]),
  writable: false,
});

// Mocking the Wallet
vi.doMock("@dusk-network/dusk-wallet-js", async (importOriginal) => ({
  ...(await importOriginal()),
  Wallet,
}));

/*
 * Mocking deprecated `atob` and `btoa` functions in Node.
 * Vitest get stuck otherwise.
 */
vi.spyOn(global, "atob").mockImplementation((data) =>
  Buffer.from(data, "base64").toString("binary")
);
vi.spyOn(global, "btoa").mockImplementation((data) =>
  Buffer.from(data, "binary").toString("base64")
);

// Adding missing bits in JSDOM

vi.mock("./src/lib/dusk/mocks/IntersectionObserver");

global.IntersectionObserver = IntersectionObserver;
global.ResizeObserver = ResizeObserver;

/*
 * Need to set it this way for Node 20, otherwise
 * it fails saying that it can't assign to `crypto`
 * which only has a getter.
 */
Object.defineProperty(global, "crypto", {
  get() {
    return crypto;
  },
});

const elementMethods = ["scrollBy", "scrollTo", "scrollIntoView"];

elementMethods.forEach((method) => {
  if (!Element.prototype[method]) {
    Object.defineProperty(Element.prototype, method, {
      value: () => {},
      writable: true,
    });
  }
});

// Add custom jest matchers
expect.extend(matchers);

// Mock SvelteKit runtime module $app/environment
vi.mock("$app/environment", () => ({
  browser: false,
  building: false,
  dev: true,
  version: "any",
}));

// Mock app paths
vi.mock("$app/paths", async (importOriginal) => ({
  ...(await importOriginal()),
  get base() {
    return "/some-base-path";
  },
}));

// Mock SvelteKit runtime module $app/navigation
vi.mock("$app/navigation", () => ({
  afterNavigate: () => {},
  beforeNavigate: () => {},
  disableScrollHandling: () => {},
  goto: () => Promise.resolve(),
  invalidate: () => Promise.resolve(),
  invalidateAll: () => Promise.resolve(),
  preloadCode: () => Promise.resolve(),
  preloadData: () => Promise.resolve(),
}));

// Mock SvelteKit runtime module $app/stores
vi.mock("$app/stores", () => {
  const getStores = () => {
    const navigating = readable(null);
    const page = readable({
      data: {},
      error: null,
      form: undefined,
      params: {},
      route: {
        id: null,
      },
      status: 200,
      url: new URL("http://localhost"),
    });
    const updated = {
      check: async () => false,
      subscribe: readable(false).subscribe,
    };

    return { navigating, page, updated };
  };

  const page = {
    subscribe(fn) {
      return getStores().page.subscribe(fn);
    },
  };
  const navigating = {
    subscribe(fn) {
      return getStores().navigating.subscribe(fn);
    },
  };
  const updated = {
    check: async () => false,
    subscribe(fn) {
      return getStores().updated.subscribe(fn);
    },
  };

  return {
    getStores,
    navigating,
    page,
    updated,
  };
});
