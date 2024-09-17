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
import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";
import { generateMnemonic } from "bip39";

import walletCache from "$lib/wallet-cache";
import { getSeedFromMnemonic } from "$lib/wallet";

import { walletStore } from "..";
import { getKey } from "lamb";

describe("Wallet store", async () => {
  vi.useFakeTimers();

  const settleTime = 1000;
  const seed = getSeedFromMnemonic(generateMnemonic());
  const profileGenerator = new ProfileGenerator(async () => seed);
  const defaultProfile = await profileGenerator.default;
  const address = await defaultProfile.address.toString();
  const balance = { maximum: 1234, value: 567 };
  const initialState = {
    addresses: [],
    balance: {
      maximum: 0,
      value: 0,
    },
    currentAddress: "",
    initialized: false,
    syncStatus: { current: 0, error: null, isInProgress: false, last: 0 },
  };
  const initializedStore = {
    ...initialState,
    addresses: [address],
    balance,
    currentAddress: address,
    initialized: true,
  };

  afterAll(() => {
    vi.useRealTimers();
  });

  describe("Initialization and sync", () => {
    it("should expose a method to initialize the store with a `ProfileGenerator` instance", async () => {
      await walletStore.init(profileGenerator);

      expect(get(walletStore)).toStrictEqual({
        ...initialState,
        addresses: [address],
        currentAddress: address,
        initialized: true,
        syncStatus: { current: 0, error: null, isInProgress: true, last: 0 },
      });

      await vi.advanceTimersByTimeAsync(settleTime);

      expect(get(walletStore)).toStrictEqual(initializedStore);
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
      const newAddress = await newGenerator.default
        .then(getKey("address"))
        .then(String);

      walletStore.clearLocalDataAndInit(newGenerator, 99n);

      await vi.runAllTimersAsync();

      expect(cacheClearSpy).toHaveBeenCalledTimes(1);
      expect(get(walletStore)).toStrictEqual({
        ...initializedStore,
        addresses: [newAddress],
        currentAddress: newAddress,
      });
    });
  });
});
