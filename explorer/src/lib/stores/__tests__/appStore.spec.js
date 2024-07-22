import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

describe("appStore", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("should be a readable store holding the information needed throughout the whole application", async () => {
    const { appStore } = await import("..");
    const { env } = import.meta;
    const expectedNetworks = [
      { label: "Testnet", value: env.VITE_DUSK_TESTNET_NODE },
      { label: "Devnet", value: env.VITE_DUSK_DEVNET_NODE },
    ];

    expect(appStore).toHaveProperty("subscribe", expect.any(Function));
    expect(appStore).not.toHaveProperty("set");
    expect(get(appStore)).toStrictEqual({
      blocksListEntries: Number(env.VITE_BLOCKS_LIST_ENTRIES),
      chainInfoEntries: Number(env.VITE_CHAIN_INFO_ENTRIES),
      darkMode: false,
      fetchInterval: Number(env.VITE_REFETCH_INTERVAL),
      marketDataFetchInterval: Number(env.VITE_MARKET_DATA_REFETCH_INTERVAL),
      network: expectedNetworks[0].value,
      networks: expectedNetworks,
      statsFetchInterval: Number(env.VITE_STATS_REFETCH_INTERVAL),
      transactionsListEntries: Number(env.VITE_TRANSACTIONS_LIST_ENTRIES),
    });
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
});
