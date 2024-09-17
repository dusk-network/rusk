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
  AddressSyncer,
  Bookkeeper,
  Gas,
  Network,
  ProfileGenerator,
} from "$lib/vendor/w3sper.js/src/mod";
import * as b58 from "$lib/vendor/w3sper.js/src/b58";
import { generateMnemonic } from "bip39";

import { cacheUnspentNotes } from "$lib/mock-data";
import walletCache from "$lib/wallet-cache";
import { getSeedFromMnemonic } from "$lib/wallet";

import { walletStore } from "..";

describe("Wallet store", async () => {
  vi.useFakeTimers();

  const abortControllerSpy = vi.spyOn(AbortController.prototype, "abort");
  const addressSyncerNotesSpy = vi.spyOn(AddressSyncer.prototype, "notes");

  // setting up a predictable address and balance
  const address = cacheUnspentNotes[0].address;
  const bookkeeperBalance = {
    spendable: 400000000000000n,
    value: 1026179647718621n,
  };
  const balance = {
    maximum: bookkeeperBalance.spendable,
    value: bookkeeperBalance.value,
  };
  const balanceSpy = vi
    .spyOn(Bookkeeper.prototype, "balance")
    .mockResolvedValue(bookkeeperBalance);
  const defaultProfileSpy = vi
    .spyOn(ProfileGenerator.prototype, "default", "get")
    .mockResolvedValue({
      address: {
        toString() {
          return address;
        },
      },
    });
  const seed = getSeedFromMnemonic(generateMnemonic());
  const profileGenerator = new ProfileGenerator(async () => seed);
  const defaultProfile = await profileGenerator.default;

  const initialState = {
    addresses: [],
    balance: {
      maximum: 0n,
      value: 0n,
    },
    currentAddress: "",
    currentProfile: defaultProfile,
    initialized: false,
    profiles: [],
    syncStatus: {
      current: 0n,
      error: null,
      isInProgress: false,
      last: 0n,
      progress: 0,
    },
  };

  const initializedStore = {
    ...initialState,
    addresses: [address],
    balance,
    currentAddress: address,
    initialized: true,
    profiles: [defaultProfile],
  };

  afterEach(() => {
    abortControllerSpy.mockClear();
    addressSyncerNotesSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    abortControllerSpy.mockRestore();
    addressSyncerNotesSpy.mockRestore();
    balanceSpy.mockRestore();
    defaultProfileSpy.mockRestore();
  });

  describe("Initialization and sync", () => {
    it("should expose a method to initialize the store with a `ProfileGenerator` instance", async () => {
      await walletStore.init(profileGenerator);

      expect(get(walletStore)).toStrictEqual({
        ...initialState,
        addresses: [address],
        currentAddress: address,
        currentProfile: defaultProfile,
        initialized: true,
        profiles: [defaultProfile],
        syncStatus: {
          current: 0n,
          error: null,
          isInProgress: true,
          last: 0n,
          progress: 0,
        },
      });

      await vi.runAllTimersAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);
    });
  });

  describe("Abort sync", () => {
    beforeEach(async () => {
      await walletStore.init(profileGenerator);

      await vi.runAllTimersAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);

      // somewhere else (dexie?) the abort is called already six times
      abortControllerSpy.mockClear();
      addressSyncerNotesSpy.mockClear();
    });

    it("should expose a method to abort a sync that is in progress and set the current sync promise to `null` so that a new sync can be started", async () => {
      walletStore.sync();

      await vi.waitUntil(() => addressSyncerNotesSpy.mock.calls.length === 1);

      walletStore.abortSync();

      expect(abortControllerSpy).toHaveBeenCalledTimes(1);

      await vi.runAllTimersAsync();

      const { syncStatus } = get(walletStore);

      expect(syncStatus.isInProgress).toBe(false);
      expect(syncStatus.error).toBeInstanceOf(Error);

      walletStore.sync();

      await vi.runAllTimersAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);
    });

    it("should do nothing if there is no sync in progress", async () => {
      walletStore.abortSync();

      expect(abortControllerSpy).not.toHaveBeenCalled();
    });
  });

  describe("Wallet store services", () => {
    const cacheClearSpy = vi.spyOn(walletCache, "clear");

    beforeEach(async () => {
      await walletStore.init(profileGenerator);
      await vi.runAllTimersAsync();
    });

    afterEach(() => {
      cacheClearSpy.mockClear();
    });

    afterAll(() => {
      cacheClearSpy.mockRestore();
    });

    it("should expose a method to clear local data", async () => {
      vi.useRealTimers();

      await walletStore.clearLocalData();

      expect(cacheClearSpy).toHaveBeenCalledTimes(1);

      vi.useFakeTimers();
    });

    it("should expose a method to clear local data and init the wallet", async () => {
      const newGenerator = new ProfileGenerator(() => new Uint8Array());
      const newProfile = await newGenerator.default;
      const newAddress = newProfile.address.toString();

      walletStore.clearLocalDataAndInit(newGenerator, 99n);

      await vi.runAllTimersAsync();

      expect(cacheClearSpy).toHaveBeenCalledTimes(1);
      expect(get(walletStore)).toStrictEqual({
        ...initializedStore,
        addresses: [newAddress],
        currentAddress: newAddress,
        currentProfile: newProfile,
        profiles: [newProfile],
      });
    });

    it("should expose a method to execute a phoenix transfer", async () => {
      vi.useRealTimers();

      const txResult = {
        hash: "some-tx-id",
        nullifiers: [],
      };
      const executeSpy = vi
        .spyOn(Network.prototype, "execute")
        .mockResolvedValue(txResult);
      const setPendingNotesSpy = vi.spyOn(walletCache, "setPendingNoteInfo");
      // const syncSpy = vi.spyOn(walletStore, "sync");
      const from = get(walletStore).currentProfile?.address.toString();
      const to =
        "4ZH3oyfTuMHyWD1Rp4e7QKp5yK6wLrWvxHneufAiYBAjvereFvfjtDvTbBcZN5ZCsaoMo49s1LKPTwGpowik6QJG";
      const amount = 150_000_000_000n;
      const gas = new Gas({ limit: 500n, price: 1n });
      const expectedTx = {
        amount,
        from,
        gas,
        obfuscated: true,
        to: b58.decode(to),
      };
      const result = await walletStore.transfer(to, amount, gas);

      expect(executeSpy).toHaveBeenCalledTimes(1);

      // our TransactionBuilder mock is loaded
      expect(executeSpy.mock.calls[0][0].toJSON()).toStrictEqual(expectedTx);
      expect(setPendingNotesSpy).toHaveBeenCalledTimes(1);
      expect(setPendingNotesSpy).toHaveBeenCalledWith(
        txResult.nullifiers,
        txResult.hash
      );
      expect(result).toBe(txResult);
      // expect(syncSpy).toHaveBeenCalledTimes(2);

      executeSpy.mockRestore();
      setPendingNotesSpy.mockRestore();
      // syncSpy.mockRestore();
      vi.useFakeTimers();
    });
  });
});
