import { describe, expect, it } from "vitest";
import { get } from "svelte/store";

import { appStore } from "..";

describe("appStore", () => {
  it("should be a readable store holding the information needed throughout the whole application", () => {
    const { env } = import.meta;
    const expectedNetworks = [
      { label: "Testnet", value: env.VITE_DUSK_TESTNET_NODE },
      { label: "Devnet", value: env.VITE_DUSK_DEVNET_NODE },
    ];

    expect(appStore).toHaveProperty("subscribe", expect.any(Function));
    expect(appStore).not.toHaveProperty("set");
    expect(get(appStore)).toStrictEqual({
      fetchInterval: Number(env.VITE_REFETCH_INTERVAL),
      network: expectedNetworks[0].value,
      networks: expectedNetworks,
    });
  });

  it("should expose a service method to set the selected network", () => {
    appStore.setNetwork("some-network");

    expect(get(appStore).network).toBe("some-network");
  });
});
