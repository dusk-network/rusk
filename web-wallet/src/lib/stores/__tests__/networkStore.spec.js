import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { get } from "svelte/store";

import {
  AccountSyncer,
  AddressSyncer,
  Network,
} from "$lib/vendor/w3sper.js/src/mod";

describe("Network store", async () => {
  const blockHeight = 999_888_777n;
  const connectSpy = vi.spyOn(Network.prototype, "connect");
  const disconnectSpy = vi.spyOn(Network.prototype, "disconnect");
  const blockHeightSpy = vi
    .spyOn(Network.prototype, "blockHeight", "get")
    .mockResolvedValue(blockHeight);
  const networkQuerySpy = vi.spyOn(Network.prototype, "query");

  afterEach(() => {
    connectSpy.mockClear();
    disconnectSpy.mockClear();
    networkQuerySpy.mockClear();
  });

  afterAll(() => {
    connectSpy.mockRestore();
    disconnectSpy.mockRestore();
    blockHeightSpy.mockRestore();
    networkQuerySpy.mockRestore();
  });

  it("should build the network with the correct URL and expose a name for it", async () => {
    let network;
    let store;

    store = (await import("..")).networkStore;
    network = await store.connect();

    expect(network.url).toStrictEqual(new URL("https://localhost"));

    vi.resetModules();

    /** @type {Record<string, string>} */
    const matches = {
      "https://devnet.nodes.dusk.network/": "Devnet",
      "https://nodes.dusk.network/": "Mainnet",
      "https://testnet.nodes.dusk.network/": "Testnet",
    };

    for (const match of Object.keys(matches)) {
      vi.stubEnv("VITE_NODE_URL", match);

      store = (await import("..")).networkStore;
      network = await store.connect();

      expect(network.url).toStrictEqual(new URL(match));
      vi.resetModules();
    }

    vi.unstubAllEnvs();
  });

  it("should expose a method to connect to the network and update the store's connection status", async () => {
    const store = (await import("..")).networkStore;

    expect(connectSpy).not.toHaveBeenCalled();

    const network = await store.connect();

    expect(connectSpy).toHaveBeenCalledTimes(1);
    expect(get(store).connected).toBe(true);
    expect(network).toBeInstanceOf(Network);
  });

  describe("Connection and disconnection", () => {
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
  });

  describe("Service methods", () => {
    /** @type {NetworkStore} */
    let store;

    beforeEach(async () => {
      store = (await import("..")).networkStore;

      // we check that every service method takes
      // care of connecting to the network when necessary
      await store.disconnect();

      expect(get(store).connected).toBe(false);
    });

    it("should expose a service method to check if a block with the given height and hash exists on the network", async () => {
      networkQuerySpy
        .mockResolvedValueOnce({ checkBlock: true })
        .mockResolvedValueOnce({ checkBlock: false });

      await expect(store.checkBlock(12n, "some-hash")).resolves.toBe(true);
      await expect(store.checkBlock(12n, "some-hash")).resolves.toBe(false);

      // check that the cached network is used
      expect(connectSpy).toHaveBeenCalledTimes(1);
    });

    it("should expose a service method to retrieve a `AccountSyncer` for the network", async () => {
      connectSpy.mockClear();

      const syncer = await store.getAccountSyncer();

      expect(connectSpy).toHaveBeenCalledTimes(1);
      expect(syncer).toBeInstanceOf(AccountSyncer);

      // check that the cached network is used
      await store.getAccountSyncer();
      expect(connectSpy).toHaveBeenCalledTimes(1);
      expect(syncer).toBeInstanceOf(AccountSyncer);
    });

    it("should expose a service method to retrieve a `AddressSyncer` for the network", async () => {
      connectSpy.mockClear();

      const syncer = await store.getAddressSyncer();

      expect(connectSpy).toHaveBeenCalledTimes(1);
      expect(syncer).toBeInstanceOf(AddressSyncer);

      // check that the cached network is used
      await store.getAddressSyncer();
      expect(connectSpy).toHaveBeenCalledTimes(1);
      expect(syncer).toBeInstanceOf(AddressSyncer);
    });

    it("should expose a method to retrieve a block hash by its height and return an empty string if the block is not found", async () => {
      const expectedHash = "some-block-hash";

      networkQuerySpy.mockResolvedValueOnce({
        block: { header: { hash: expectedHash } },
      });

      await expect(store.getBlockHashByHeight(123n)).resolves.toStrictEqual(
        expectedHash
      );

      networkQuerySpy.mockResolvedValueOnce({ block: null });

      await expect(store.getBlockHashByHeight(123n)).resolves.toBe("");

      // check that the cached network is used
      expect(connectSpy).toHaveBeenCalledTimes(1);
    });

    it("should expose a service method to retrieve the current block height", async () => {
      await expect(store.getCurrentBlockHeight()).resolves.toBe(blockHeight);
    });

    it("should expose a method to retrieve the last finalized block height and return `0n` if the block is not found", async () => {
      const height = 123;

      networkQuerySpy.mockResolvedValueOnce({
        lastBlockPair: {
          // eslint-disable-next-line camelcase
          json: { last_finalized_block: [height, "some-block-hash"] },
        },
      });

      await expect(store.getLastFinalizedBlockHeight()).resolves.toStrictEqual(
        BigInt(height)
      );

      networkQuerySpy.mockResolvedValueOnce({ lastBlockPair: null });

      await expect(store.getLastFinalizedBlockHeight()).resolves.toBe(0n);

      // check that the cached network is used
      expect(connectSpy).toHaveBeenCalledTimes(1);
    });
  });
});
