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
  Bookkeeper,
  Gas,
  Network,
  ProfileGenerator,
} from "$lib/vendor/w3sper.js/src/mod";
import { generateMnemonic } from "bip39";

import walletCache from "$lib/wallet-cache";
import WalletTreasury from "$lib/wallet-treasury";
import { getSeedFromMnemonic } from "$lib/wallet";

import { walletStore } from "..";

describe("Wallet store", async () => {
  vi.useFakeTimers();

  const AUTO_SYNC_INTERVAL = 5 * 60 * 1000;
  const cachedBalance = {
    shielded: {
      spendable: 10n,
      value: 5n,
    },
    unshielded: {
      nonce: 3n,
      value: 4n,
    },
  };
  const shielded = {
    spendable: 400000000000000n,
    value: 1026179647718621n,
  };
  const unshielded = {
    nonce: 1234n,
    value: shielded.value / 2n,
  };

  const setTimeoutSpy = vi.spyOn(window, "setTimeout");
  const clearTimeoutSpy = vi.spyOn(window, "clearTimeout");

  const abortControllerSpy = vi.spyOn(AbortController.prototype, "abort");
  const balanceSpy = vi
    .spyOn(Bookkeeper.prototype, "balance")
    .mockImplementation(async (identifier) => {
      return ProfileGenerator.typeOf(identifier.toString()) === "address"
        ? shielded
        : unshielded;
    });

  const getCachedBalanceSpy = vi
    .spyOn(walletCache, "getBalanceInfo")
    .mockResolvedValue(cachedBalance);
  const setCachedBalanceSpy = vi
    .spyOn(walletCache, "setBalanceInfo")
    .mockResolvedValue(undefined);
  const setLastBlockHeightSpy = vi.spyOn(walletCache, "setLastBlockHeight");
  const setProfilesSpy = vi.spyOn(WalletTreasury.prototype, "setProfiles");
  const treasuryUpdateSpy = vi.spyOn(WalletTreasury.prototype, "update");

  const seed = getSeedFromMnemonic(generateMnemonic());
  const profileGenerator = new ProfileGenerator(async () => seed);
  const defaultProfile = await profileGenerator.default;

  const initialState = {
    balance: {
      shielded: {
        spendable: 0n,
        value: 0n,
      },
      unshielded: {
        nonce: 0n,
        value: 0n,
      },
    },
    currentProfile: null,
    initialized: false,
    profiles: [],
    syncStatus: {
      error: null,
      from: 0n,
      isInProgress: false,
      last: 0n,
      progress: 0,
    },
  };

  const initializedStore = {
    ...initialState,
    balance: { shielded, unshielded },
    currentProfile: defaultProfile,
    initialized: true,
    profiles: [defaultProfile],
  };

  beforeEach(async () => {
    await vi.runOnlyPendingTimersAsync();
    vi.clearAllTimers();
    setTimeoutSpy.mockClear();
    clearTimeoutSpy.mockClear();
    abortControllerSpy.mockClear();
    setLastBlockHeightSpy.mockClear();
    setCachedBalanceSpy.mockClear();
    setProfilesSpy.mockClear();
    treasuryUpdateSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    setTimeoutSpy.mockRestore();
    clearTimeoutSpy.mockRestore();
    abortControllerSpy.mockRestore();
    setLastBlockHeightSpy.mockRestore();
    balanceSpy.mockRestore();
    getCachedBalanceSpy.mockRestore();
    setCachedBalanceSpy.mockRestore();
    setProfilesSpy.mockRestore();
    treasuryUpdateSpy.mockRestore();
  });

  describe("Initialization and sync", () => {
    it("should expose a method to initialize the store with a `ProfileGenerator` instance", async () => {
      await walletStore.init(profileGenerator);

      expect(get(walletStore)).toStrictEqual({
        ...initialState,
        balance: cachedBalance,
        currentProfile: defaultProfile,
        initialized: true,
        profiles: [defaultProfile],
        syncStatus: {
          error: null,
          from: 0n,
          isInProgress: true,
          last: 0n,
          progress: 0,
        },
      });

      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      expect(get(walletStore)).toStrictEqual(initializedStore);
      expect(setProfilesSpy).toHaveBeenCalledTimes(1);
      expect(setProfilesSpy).toHaveBeenCalledWith([defaultProfile]);
      expect(setProfilesSpy.mock.invocationCallOrder[0]).toBeLessThan(
        treasuryUpdateSpy.mock.invocationCallOrder[0]
      );
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);
      expect(setLastBlockHeightSpy).toHaveBeenCalledTimes(1);
      expect(setLastBlockHeightSpy).toHaveBeenCalledWith(expect.any(BigInt));
      expect(setLastBlockHeightSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[0]
      );
      expect(balanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[0]
      );
      expect(balanceSpy).toHaveBeenCalledTimes(2);
      expect(balanceSpy).toHaveBeenNthCalledWith(1, defaultProfile.address);
      expect(balanceSpy).toHaveBeenNthCalledWith(2, defaultProfile.account);
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(
        defaultProfile.address.toString(),
        {
          shielded: await balanceSpy.mock.results[0].value,
          unshielded: await balanceSpy.mock.results[1].value,
        }
      );
      expect(setCachedBalanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        balanceSpy.mock.invocationCallOrder[1]
      );
      expect(clearTimeoutSpy).toHaveBeenCalledTimes(1);
      expect(setTimeoutSpy).toHaveBeenCalledTimes(1);
      expect(clearTimeoutSpy.mock.invocationCallOrder[0]).toBeLessThan(
        setTimeoutSpy.mock.invocationCallOrder[0]
      );

      await vi.advanceTimersByTimeAsync(1);
      await vi.runOnlyPendingTimersAsync();

      // auto sync started
      expect(get(walletStore)).toStrictEqual({
        ...initializedStore,
        syncStatus: {
          ...initializedStore.syncStatus,
          isInProgress: true,
        },
      });

      walletStore.reset();
      expect(get(walletStore)).toStrictEqual(initialState);
    });
  });

  describe("Abort sync", () => {
    beforeEach(async () => {
      walletStore.reset();
      expect(get(walletStore)).toStrictEqual(initialState);

      await walletStore.init(profileGenerator);
      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      vi.clearAllTimers();

      expect(get(walletStore)).toStrictEqual(initializedStore);

      clearTimeoutSpy.mockClear();

      // somewhere else (dexie?) the abort is called already six times
      abortControllerSpy.mockClear();
      treasuryUpdateSpy.mockClear();
    });

    it("should expose a method to abort a sync that is in progress and set the current sync promise to `null` so that a new sync can be started", async () => {
      walletStore.sync();

      await vi.waitUntil(() => treasuryUpdateSpy.mock.calls.length === 1);

      walletStore.abortSync();

      expect(abortControllerSpy).toHaveBeenCalledTimes(1);
      expect(clearTimeoutSpy).toHaveBeenCalledTimes(1);

      await vi.runAllTimersAsync();

      const { syncStatus } = get(walletStore);

      expect(syncStatus.isInProgress).toBe(false);
      expect(syncStatus.error).toBeInstanceOf(Error);

      walletStore.sync();

      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      expect(get(walletStore)).toStrictEqual(initializedStore);

      walletStore.abortSync();
    });

    it("should do nothing but stopping the auto-sync if there is no sync in progress", async () => {
      walletStore.abortSync();

      expect(abortControllerSpy).not.toHaveBeenCalled();
      expect(clearTimeoutSpy).toHaveBeenCalledTimes(1);
    });
  });

  describe("Wallet store services", () => {
    const toPhoenix =
      "4ZH3oyfTuMHyWD1Rp4e7QKp5yK6wLrWvxHneufAiYBAjvereFvfjtDvTbBcZN5ZCsaoMo49s1LKPTwGpowik6QJG";
    const toMoonlight =
      "zTsZq814KfWUAQujzjBchbMEvqA1FiKBUakMCtAc2zCa74h9YVz4a2roYwS7LHDHeBwS1aap4f3GYhQBrxroYgsBcE4FJdkUbvpSD5LVXY6JRXNgMXgk6ckTPJUFKoHybff";
    const amount = 150_000_000_000n;
    const gas = new Gas({ limit: 500n, price: 1n });

    const phoenixTxResult = {
      hash: "some-tx-id",
      nullifiers: [],
    };

    const cacheClearSpy = vi.spyOn(walletCache, "clear");
    const executeSpy = vi
      .spyOn(Network.prototype, "execute")
      .mockResolvedValue(phoenixTxResult);
    const setPendingNotesSpy = vi.spyOn(walletCache, "setPendingNoteInfo");

    beforeEach(async () => {
      walletStore.reset();
      expect(get(walletStore)).toStrictEqual(initialState);

      await walletStore.init(profileGenerator);
      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      vi.clearAllTimers();

      const currentStore = get(walletStore);

      expect(currentStore).toStrictEqual(initializedStore);

      const { currentProfile } = currentStore;

      expect(currentProfile).toBeDefined();

      treasuryUpdateSpy.mockClear();
      balanceSpy.mockClear();
      setCachedBalanceSpy.mockClear();
    });

    afterEach(async () => {
      cacheClearSpy.mockClear();
      executeSpy.mockClear();
      setPendingNotesSpy.mockClear();
    });

    afterAll(() => {
      cacheClearSpy.mockRestore();
      executeSpy.mockRestore();
      setPendingNotesSpy.mockRestore();
    });

    it("should expose a method to clear local data", async () => {
      /**
       * For some reason calling `useRealTimers` makes
       * `setTimeout` and `clearTimeout` disappear from
       * `window` if they are spied upon.
       */
      setTimeoutSpy.mockRestore();
      clearTimeoutSpy.mockRestore();
      vi.useRealTimers();

      await walletStore.clearLocalData();

      expect(cacheClearSpy).toHaveBeenCalledTimes(1);

      vi.useFakeTimers();
    });

    it("should expose a method to clear local data and init the wallet", async () => {
      const newGenerator = new ProfileGenerator(() => new Uint8Array());
      const newProfile = await newGenerator.default;

      walletStore.clearLocalDataAndInit(newGenerator, 99n);

      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);
      vi.clearAllTimers();

      expect(cacheClearSpy).toHaveBeenCalledTimes(1);
      expect(get(walletStore)).toStrictEqual({
        ...initializedStore,
        currentProfile: newProfile,
        profiles: [newProfile],
      });
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);
      expect(balanceSpy).toHaveBeenCalledTimes(2);
      expect(balanceSpy).toHaveBeenNthCalledWith(1, newProfile.address);
      expect(balanceSpy).toHaveBeenNthCalledWith(2, newProfile.account);
    });

    it("should expose a method to set the current profile and update the balance afterwards", async () => {
      const fakeExtraProfile = {
        account: {
          toString() {
            return "some-fake-account";
          },
        },
        address: {
          toString() {
            return "some-fake-address";
          },
        },
      };

      // nasty mutation for the sake of easy testing
      // @ts-expect-error we don't care for it to be a real profile
      get(walletStore).profiles.push(fakeExtraProfile);

      await expect(
        // @ts-expect-error we don't care to set a real profile
        walletStore.setCurrentProfile(fakeExtraProfile)
      ).resolves.toBeUndefined();

      expect(get(walletStore).currentProfile).toBe(fakeExtraProfile);
      expect(balanceSpy).toHaveBeenCalledTimes(2);
      expect(balanceSpy).toHaveBeenNthCalledWith(1, fakeExtraProfile.address);
      expect(balanceSpy).toHaveBeenNthCalledWith(2, fakeExtraProfile.account);
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(
        fakeExtraProfile.address.toString(),
        {
          shielded: await balanceSpy.mock.results[0].value,
          unshielded: await balanceSpy.mock.results[1].value,
        }
      );
      expect(setCachedBalanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        balanceSpy.mock.invocationCallOrder[1]
      );
    });

    it("should reject with an error if the profile is not in the known list", async () => {
      // @ts-expect-error we don't care to set a real profile
      await expect(walletStore.setCurrentProfile({})).rejects.toThrow();
    });

    it("should expose a method to shield a given amount from the unshielded account", async () => {
      setTimeoutSpy.mockRestore();
      clearTimeoutSpy.mockRestore();
      vi.useRealTimers();

      const currentlyCachedBalance = await walletCache.getBalanceInfo(
        defaultProfile.address.toString()
      );
      const newNonce = currentlyCachedBalance.unshielded.nonce + 1n;

      executeSpy.mockResolvedValueOnce({
        hash: phoenixTxResult.hash,
        nonce: newNonce,
      });

      const expectedTx = {
        amount,
        gas,
      };

      await walletStore.shield(amount, gas);

      expect(executeSpy.mock.calls[0][0].attributes).toStrictEqual(expectedTx);
      expect(executeSpy.mock.calls[0][0].bookentry.profile).toStrictEqual(
        defaultProfile
      );
      expect(setPendingNotesSpy).not.toHaveBeenCalled();
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(
        defaultProfile.address.toString(),
        {
          ...currentlyCachedBalance,
          unshielded: {
            ...currentlyCachedBalance.unshielded,
            nonce: newNonce,
          },
        }
      );

      vi.useFakeTimers();
    });

    it("should expose a method to execute a phoenix transfer, if the receiver is a phoenix address", async () => {
      /**
       * For some reason calling `useRealTimers` makes
       * `setTimeout` and `clearTimeout` disappear from
       * `window` if they are spied upon.
       */
      setTimeoutSpy.mockRestore();
      clearTimeoutSpy.mockRestore();
      vi.useRealTimers();

      const expectedTx = {
        amount,
        gas,
        obfuscated: true,
        to: toPhoenix,
      };
      const result = await walletStore.transfer(toPhoenix, amount, gas);

      expect(executeSpy).toHaveBeenCalledTimes(1);
      expect(executeSpy.mock.calls[0][0].attributes).toStrictEqual(expectedTx);
      expect(executeSpy.mock.calls[0][0].bookentry.profile).toStrictEqual(
        defaultProfile
      );
      expect(setCachedBalanceSpy).not.toHaveBeenCalled();
      expect(setPendingNotesSpy).toHaveBeenCalledTimes(1);
      expect(setPendingNotesSpy).toHaveBeenCalledWith(
        phoenixTxResult.nullifiers,
        phoenixTxResult.hash
      );
      expect(result).toBe(phoenixTxResult);

      // check that we made a sync before the transfer
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);

      // but the balance is not updated yet
      expect(balanceSpy).not.toHaveBeenCalled();

      expect(treasuryUpdateSpy.mock.invocationCallOrder[0]).toBeLessThan(
        executeSpy.mock.invocationCallOrder[0]
      );

      // hacky check that we used the correct API through our Transactions mock
      const expectedScope = {
        id: phoenixTxResult.hash,
        name: "transactions",
        once: true,
      };

      // this will trigger the resolve in the `removed` promise
      dispatchEvent(
        new CustomEvent("transaction::removed", { detail: expectedScope })
      );

      // check that a sync starts after the transaction is removed from the mempool
      await vi.waitUntil(() => treasuryUpdateSpy.mock.calls.length === 2);

      // check that the balance is updated afterwards
      expect(balanceSpy).toHaveBeenCalledTimes(2);
      expect(balanceSpy).toHaveBeenNthCalledWith(1, defaultProfile.address);
      expect(balanceSpy).toHaveBeenNthCalledWith(2, defaultProfile.account);
      expect(balanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[1]
      );
      expect(balanceSpy.mock.invocationCallOrder[1]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[1]
      );
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(
        defaultProfile.address.toString(),
        {
          shielded: await balanceSpy.mock.results[0].value,
          unshielded: await balanceSpy.mock.results[1].value,
        }
      );
      expect(setCachedBalanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        balanceSpy.mock.invocationCallOrder[1]
      );

      vi.useFakeTimers();
    });

    it("should use the moonlight account and shouldn't obfuscate the transaction if the receiver is a moonlight account", async () => {
      /**
       * For some reason calling `useRealTimers` makes
       * `setTimeout` and `clearTimeout` disappear from
       * `window` if they are spied upon.
       */
      setTimeoutSpy.mockRestore();
      clearTimeoutSpy.mockRestore();
      vi.useRealTimers();

      const currentlyCachedBalance = await walletCache.getBalanceInfo(
        defaultProfile.address.toString()
      );
      const newNonce = currentlyCachedBalance.unshielded.nonce + 1n;

      executeSpy.mockResolvedValueOnce({
        hash: phoenixTxResult.hash,
        nonce: newNonce,
      });

      const expectedTx = {
        amount,
        gas,
        to: toMoonlight,
      };

      await walletStore.transfer(toMoonlight, amount, gas);

      expect(executeSpy.mock.calls[0][0].attributes).toStrictEqual(expectedTx);
      expect(executeSpy.mock.calls[0][0].bookentry.profile).toStrictEqual(
        defaultProfile
      );
      expect(setPendingNotesSpy).not.toHaveBeenCalled();
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(
        defaultProfile.address.toString(),
        {
          ...currentlyCachedBalance,
          unshielded: {
            ...currentlyCachedBalance.unshielded,
            nonce: newNonce,
          },
        }
      );

      vi.useFakeTimers();
    });

    it("should expose a method to unshield a given amount from the shielded account", async () => {
      setTimeoutSpy.mockRestore();
      clearTimeoutSpy.mockRestore();
      vi.useRealTimers();

      const expectedTx = {
        amount,
        gas,
      };
      const result = await walletStore.unshield(amount, gas);

      expect(executeSpy).toHaveBeenCalledTimes(1);
      expect(executeSpy.mock.calls[0][0].attributes).toStrictEqual(expectedTx);
      expect(executeSpy.mock.calls[0][0].bookentry.profile).toStrictEqual(
        defaultProfile
      );
      expect(setCachedBalanceSpy).not.toHaveBeenCalled();
      expect(setPendingNotesSpy).toHaveBeenCalledTimes(1);
      expect(setPendingNotesSpy).toHaveBeenCalledWith(
        phoenixTxResult.nullifiers,
        phoenixTxResult.hash
      );
      expect(result).toBe(phoenixTxResult);

      // check that we made a sync before the transfer
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);

      // but the balance is not updated yet
      expect(balanceSpy).not.toHaveBeenCalled();

      expect(treasuryUpdateSpy.mock.invocationCallOrder[0]).toBeLessThan(
        executeSpy.mock.invocationCallOrder[0]
      );

      vi.useFakeTimers();
    });
  });
});
