import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

import { Network } from "$lib/vendor/w3sper.js/src/mod";

describe("Network store", async () => {
  const blockHeight = 999_888_777n;
  const connectSpy = vi.spyOn(Network.prototype, "connect");
  const disconnectSpy = vi.spyOn(Network.prototype, "disconnect");
  const blockHeightSpy = vi
    .spyOn(Network.prototype, "blockHeight", "get")
    .mockResolvedValue(blockHeight);

  afterEach(() => {
    connectSpy.mockClear();
    disconnectSpy.mockClear();
  });

  afterAll(() => {
    connectSpy.mockRestore();
    disconnectSpy.mockRestore();
    blockHeightSpy.mockRestore();
  });

  it("should expose a method to connect to the network and update the store's connection status", async () => {
    const store = (await import("..")).networkStore;

    expect(connectSpy).not.toHaveBeenCalled();

    const network = await store.connect();

    expect(connectSpy).toHaveBeenCalledTimes(1);
    expect(get(store).connected).toBe(true);
    expect(network).toBeInstanceOf(Network);
  });

  it("should expose a method to disconnect from the network and update the store's connection status", async () => {
    const store = (await import("..")).networkStore;

    await store.connect();

    expect(get(store).connected).toBe(true);

    await store.disconnect();

    expect(disconnectSpy).toHaveBeenCalledTimes(1);
    expect(get(store).connected).toBe(false);
  });

  it("should not try to connect again to the network if it's already connected", async () => {
    const store = (await import("..")).networkStore;

    const network = await store.connect();

    expect(connectSpy).toHaveBeenCalledTimes(1);
    expect(get(store).connected).toBe(true);

    connectSpy.mockClear();

    const network2 = await store.connect();

    expect(network2).toBe(network);
    expect(connectSpy).not.toHaveBeenCalled();
    expect(get(store).connected).toBe(true);
  });

  it("should expose a service method to retrieve the current block height", async () => {
    const store = (await import("..")).networkStore;

    await expect(store.getCurrentBlockHeight()).resolves.toBe(blockHeight);
  });
});
