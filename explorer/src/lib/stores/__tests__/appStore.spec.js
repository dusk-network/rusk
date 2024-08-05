import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import appStore from "../appStore";

describe("appStore", () => {
  const originalTouchStart = window.ontouchstart;
  const originalMaxTouchPoints = navigator.maxTouchPoints;

  delete window.ontouchstart;

  Object.defineProperty(navigator, "maxTouchPoints", {
    value: 0,
    writable: true,
  });

  beforeEach(() => {
    vi.resetModules();
  });

  afterAll(() => {
    window.ontouchstart = originalTouchStart;

    Object.defineProperty(navigator, "maxTouchPoints", {
      value: originalMaxTouchPoints,
      writable: false,
    });
  });

  it("should be a readable store holding the information needed throughout the whole application", async () => {
    const { appStore } = await import("..");
    const { env } = import.meta;
    const expectedNetworks = [
      { label: "Testnet", value: env.VITE_DUSK_TESTNET_NODE },
    ];

    expect(appStore).toHaveProperty("subscribe", expect.any(Function));
    expect(appStore).not.toHaveProperty("set");
    expect(get(appStore)).toStrictEqual({
      blocksListEntries: Number(env.VITE_BLOCKS_LIST_ENTRIES),
      chainInfoEntries: Number(env.VITE_CHAIN_INFO_ENTRIES),
      darkMode: false,
      fetchInterval: Number(env.VITE_REFETCH_INTERVAL),
      isSmallScreen: false,
      hasTouchSupport: false,
      marketDataFetchInterval: Number(env.VITE_MARKET_DATA_REFETCH_INTERVAL),
      network: expectedNetworks[0].value,
      networks: expectedNetworks,
      statsFetchInterval: Number(env.VITE_STATS_REFETCH_INTERVAL),
      transactionsListEntries: Number(env.VITE_TRANSACTIONS_LIST_ENTRIES),
    });
  });

  it("should set the `hasTouchSupport` property to true if the `ontouchstart` property exists on `window`", async () => {
    window.ontouchstart = originalTouchStart;

    const { appStore } = await import("..");

    expect(get(appStore).hasTouchSupport).toBe(true);

    delete window.ontouchstart;
  });

  it("should set the `hasTouchSupport` property to true if the `navigator.maxTouchPoints` property is greater than zero", async () => {
    // @ts-ignore
    navigator.maxTouchPoints = 1;

    const { appStore } = await import("..");

    expect(get(appStore).hasTouchSupport).toBe(true);

    // @ts-ignore
    navigator.maxTouchPoints = 0;
  });

  it("should use default values for the fetch intervals if the env vars are missing", async () => {
    vi.stubEnv("VITE_REFETCH_INTERVAL", "");
    vi.stubEnv("VITE_MARKET_DATA_REFETCH_INTERVAL", "");
    vi.stubEnv("VITE_STATS_REFETCH_INTERVAL", "");

    const { appStore } = await import("..");
    const { fetchInterval, marketDataFetchInterval, statsFetchInterval } =
      get(appStore);

    expect(fetchInterval).toBe(1000);
    expect(marketDataFetchInterval).toBe(120000);
    expect(statsFetchInterval).toBe(1000);

    vi.unstubAllEnvs();
  });

  it("should expose a service method to set the selected network", async () => {
    const { appStore } = await import("..");

    appStore.setNetwork("some-network");

    expect(get(appStore).network).toBe("some-network");
  });

  it("should expose a service method to set the dark mode theme", async () => {
    const { appStore } = await import("..");

    appStore.setTheme(true);

    expect(get(appStore).darkMode).toBe(true);
  });

  it.only("should update the `isSmallScreen` property when the window width changes respective to the provided media query", async () => {
    let changeCallback;

    const mqAddListenerSpy = vi.spyOn(MediaQueryList.prototype, "addEventListener").mockImplementation((eventName, callback) => {
      if (eventName === "change") {
        changeCallback = callback;
      }
    });

    

    Object.defineProperty(window, 'innerWidth', {
      writable: true,
      configurable: true,
      value: 150,
    });

    window.dispatchEvent(new Event('resize'));

    expect(mqAddListenerSpy).toHaveBeenCalledOnce();

    mqAddListenerSpy.mockRestore();
  });
});
