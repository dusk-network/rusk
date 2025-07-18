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
import { Bookkeeper, Gas, Network, ProfileGenerator } from "@dusk/w3sper";
import { generateMnemonic } from "bip39";

import { stakeInfo } from "$lib/mock-data";

import WalletTreasury from "$lib/wallet-treasury";
import { getSeedFromMnemonic } from "$lib/wallet";

import { networkStore, walletStore } from "..";

const VITE_SYNC_INTERVAL = import.meta.env.VITE_SYNC_INTERVAL;

describe("Wallet store", async () => {
  vi.useFakeTimers();

  const AUTO_SYNC_INTERVAL = !isNaN(Number(VITE_SYNC_INTERVAL))
    ? Number(VITE_SYNC_INTERVAL)
    : 5 * 60 * 1000;

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
  const cachedStakeInfo = {
    amount: {
      eligibility: 123n,
      locked: 456n,
      get total() {
        return this.value + this.locked;
      },
      value: 100n,
    },
    faults: 10,
    hardFaults: 2,
    nonce: 5n,
    reward: 56789n,
  };
  const shielded = {
    spendable: 400000000000000n,
    value: 1026179647718621n,
  };
  const unshielded = {
    nonce: 1234n,
    value: shielded.value / 2n,
  };
  const minimumStake = 1_000_000_000_000n;

  vi.spyOn(Bookkeeper.prototype, "minimumStake", "get").mockResolvedValue(
    minimumStake
  );

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

  const stakeInfoSpy = vi
    .spyOn(Bookkeeper.prototype, "stakeInfo")
    .mockImplementation(async () => stakeInfo);

  const getCachedBalanceSpy = vi
    .spyOn(WalletTreasury.prototype, "getCachedBalance")
    .mockResolvedValue(cachedBalance);
  const setCachedBalanceSpy = vi
    .spyOn(WalletTreasury.prototype, "setCachedBalance")
    .mockResolvedValue(undefined);
  const getCachedStakeInfoSpy = vi
    .spyOn(WalletTreasury.prototype, "getCachedStakeInfo")
    .mockResolvedValue(cachedStakeInfo);
  const setCachedStakeInfoSpy = vi
    .spyOn(WalletTreasury.prototype, "setCachedStakeInfo")
    .mockResolvedValue(undefined);
  const setProfilesSpy = vi.spyOn(WalletTreasury.prototype, "setProfiles");
  const treasuryUpdateSpy = vi.spyOn(WalletTreasury.prototype, "update");

  vi.spyOn(networkStore, "checkBlock").mockResolvedValue(true);
  vi.spyOn(networkStore, "getBlockHashByHeight").mockResolvedValue(
    "some-block-hash"
  );
  vi.spyOn(networkStore, "getLastFinalizedBlockHeight").mockResolvedValue(121n);

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
    minimumStake: 0n,
    profiles: [],
    stakeInfo: {
      amount: null,
      faults: 0,
      hardFaults: 0,
      reward: 0n,
    },
    syncStatus: {
      error: null,
      from: 0n,
      isInProgress: false,
      last: 0n,
      progress: 0,
    },
  };

  beforeEach(async () => {
    await vi.runOnlyPendingTimersAsync();
    vi.clearAllTimers();
    vi.clearAllMocks();
  });

  afterAll(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  describe("Initialization and sync", () => {
    it("should expose a method to initialize the store with a `ProfileGenerator` instance", async () => {
      await walletStore.init(profileGenerator);

      const state = get(walletStore);

      expect(state.balance).toStrictEqual(cachedBalance);
      expect(state.currentProfile).toBe(defaultProfile);
      expect(state.initialized).toBe(true);
      expect(state.minimumStake).toBe(minimumStake);
      expect(state.stakeInfo).toStrictEqual(cachedStakeInfo);
      expect(state.syncStatus).toStrictEqual({
        error: null,
        from: 0n,
        isInProgress: true,
        last: 0n,
        progress: 0,
      });

      // Check that we have exactly 2 profiles
      expect(state.profiles).toHaveLength(2);
      expect(state.profiles[0]).toBe(defaultProfile);
      expect(state.profiles[1]).toBeDefined();

      expect(getCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(getCachedBalanceSpy).toHaveBeenCalledWith(defaultProfile);
      expect(getCachedStakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(getCachedStakeInfoSpy).toHaveBeenCalledWith(defaultProfile);

      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      const finalState = get(walletStore);
      expect(finalState.balance).toStrictEqual({ shielded, unshielded });
      expect(finalState.currentProfile).toBe(defaultProfile);
      expect(finalState.initialized).toBe(true);
      expect(finalState.minimumStake).toBe(minimumStake);
      expect(finalState.stakeInfo).toStrictEqual(stakeInfo);
      expect(finalState.syncStatus).toStrictEqual({
        error: null,
        from: 0n,
        isInProgress: false,
        last: 0n,
        progress: 0,
      });

      // Check that we still have exactly 2 profiles
      expect(finalState.profiles).toHaveLength(2);
      expect(finalState.profiles[0]).toBe(defaultProfile);
      expect(finalState.profiles[1]).toBeDefined();

      expect(setProfilesSpy).toHaveBeenCalledTimes(1);
      expect(setProfilesSpy.mock.calls[0][0]).toHaveLength(2);
      expect(setProfilesSpy.mock.calls[0][0][0]).toBe(defaultProfile);
      expect(setProfilesSpy.mock.invocationCallOrder[0]).toBeLessThan(
        treasuryUpdateSpy.mock.invocationCallOrder[0]
      );
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);
      expect(balanceSpy).toHaveBeenCalledTimes(2);
      expect(balanceSpy).toHaveBeenNthCalledWith(1, defaultProfile.address);
      expect(balanceSpy).toHaveBeenNthCalledWith(2, defaultProfile.account);
      expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(stakeInfoSpy).toHaveBeenCalledWith(defaultProfile.account);
      expect(balanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[0]
      );
      expect(stakeInfoSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[0]
      );
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(defaultProfile, {
        shielded: await balanceSpy.mock.results[0].value,
        unshielded: await balanceSpy.mock.results[1].value,
      });
      expect(setCachedStakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(setCachedStakeInfoSpy).toHaveBeenCalledWith(
        defaultProfile,
        await stakeInfoSpy.mock.results[0].value
      );
      expect(setCachedBalanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        balanceSpy.mock.invocationCallOrder[1]
      );
      expect(setCachedStakeInfoSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        stakeInfoSpy.mock.invocationCallOrder[0]
      );

      expect(clearTimeoutSpy).toHaveBeenCalledTimes(1);

      /**
       * The first call is caused by the timeout in the network
       * connect, so we check the last call, that is the one
       * made in the wallet store.
       */
      expect(setTimeoutSpy).toHaveBeenCalledTimes(2);
      expect(clearTimeoutSpy.mock.invocationCallOrder[0]).toBeLessThan(
        setTimeoutSpy.mock.invocationCallOrder[1]
      );

      await vi.advanceTimersByTimeAsync(1);
      await vi.runOnlyPendingTimersAsync();

      // auto sync started - check that we have 2 profiles and sync is in progress
      const autoSyncState = get(walletStore);
      expect(autoSyncState.profiles).toHaveLength(2);
      expect(autoSyncState.profiles[0]).toBe(defaultProfile);
      expect(autoSyncState.profiles[1]).toBeDefined();
      expect(autoSyncState.syncStatus.isInProgress).toBe(true);

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

      const currentState = get(walletStore);
      expect(currentState.profiles).toHaveLength(2);
      expect(currentState.profiles[0]).toBe(defaultProfile);
      expect(currentState.profiles[1]).toBeDefined();
      expect(currentState.initialized).toBe(true);
      expect(currentState.syncStatus.isInProgress).toBe(false);

      clearTimeoutSpy.mockClear();

      // somewhere else (dexie?) the abort is called already six times
      abortControllerSpy.mockClear();
      treasuryUpdateSpy.mockClear();
    });

    it("should expose a method to abort a sync that is in progress and set the current sync promise to `null` so that a new sync can be started", async () => {
      walletStore.sync();

      await vi.waitUntil(() => treasuryUpdateSpy.mock.calls.length === 1);

      walletStore.abortSync();

      expect(clearTimeoutSpy).toHaveBeenCalledTimes(1);
      expect(abortControllerSpy).toHaveBeenCalledTimes(1);

      await vi.runAllTimersAsync();

      const { syncStatus } = get(walletStore);

      expect(syncStatus.isInProgress).toBe(false);
      expect(syncStatus.error).toBeInstanceOf(Error);

      walletStore.sync();

      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      const finalState = get(walletStore);
      expect(finalState.profiles).toHaveLength(2);
      expect(finalState.profiles[0]).toBe(defaultProfile);
      expect(finalState.profiles[1]).toBeDefined();
      expect(finalState.initialized).toBe(true);
      expect(finalState.syncStatus.isInProgress).toBe(false);

      walletStore.abortSync();
    });

    it("should do nothing but stopping the auto-sync if there is no sync in progress", async () => {
      walletStore.abortSync();

      expect(abortControllerSpy).not.toHaveBeenCalled();
      expect(clearTimeoutSpy).toHaveBeenCalledTimes(1);
    });
  });

  describe("Wallet store transfers", () => {
    const toPhoenix =
      "4ZH3oyfTuMHyWD1Rp4e7QKp5yK6wLrWvxHneufAiYBAjvereFvfjtDvTbBcZN5ZCsaoMo49s1LKPTwGpowik6QJG";
    const toMoonlight =
      "zTsZq814KfWUAQujzjBchbMEvqA1FiKBUakMCtAc2zCa74h9YVz4a2roYwS7LHDHeBwS1aap4f3GYhQBrxroYgsBcE4FJdkUbvpSD5LVXY6JRXNgMXgk6ckTPJUFKoHybff";
    const amount = 150_000_000_000n;
    const gas = new Gas({ limit: 500n, price: 1n });
    const memo = "";

    const phoenixTxResult = {
      buffer: new Uint8Array(),
      hash: "some-tx-id",
      nullifiers: [],
    };

    const executeSpy = vi
      .spyOn(Network.prototype, "execute")
      .mockResolvedValue(phoenixTxResult);

    const updateNonceSpy = vi.spyOn(
      WalletTreasury.prototype,
      "updateCachedNonce"
    );

    const updateCachedPendingNotesSpy = vi.spyOn(
      WalletTreasury.prototype,
      "updateCachedPendingNotes"
    );

    /**
     * @typedef { "claimRewards" | "shield" | "stake" | "transfer" | "unshield" | "unstake" } TransferMethod
     */

    /**
     * @template {TransferMethod} M
     * @param {M} method
     * @param {Parameters<WalletStoreServices[M]>} args
     */
    async function walletStoreTransferCheck(method, args) {
      /**
       * For some reason calling `useRealTimers` makes
       * `setTimeout` and `clearTimeout` disappear from
       * `window` if they are spied upon.
       */
      setTimeoutSpy.mockRestore();
      clearTimeoutSpy.mockRestore();
      vi.useRealTimers();

      const currentlyCachedBalance =
        await new WalletTreasury().getCachedBalance(defaultProfile);
      const newNonce = currentlyCachedBalance.unshielded.nonce + 1n;

      let expectedTx;

      const isPhoenixTransfer = method === "transfer" && args[0] === toPhoenix;
      const isMoonlightTransfer =
        method === "transfer" && args[0] === toMoonlight;

      if (isPhoenixTransfer) {
        expectedTx = {
          amount,
          gas,
          memo,
          obfuscated: true,
          to: toPhoenix,
        };
      } else {
        executeSpy.mockResolvedValueOnce({
          buffer: new Uint8Array(),
          hash: phoenixTxResult.hash,
          nonce: newNonce,
        });

        if (isMoonlightTransfer) {
          expectedTx = { amount, gas, memo, to: toMoonlight };
        } else {
          switch (method) {
            case "stake":
              expectedTx = { amount, gas, topup: false };
              break;

            case "unstake":
              expectedTx = { amount, gas };
              break;

            default:
              expectedTx = { amount, gas };
              break;
          }
        }
      }

      // @ts-ignore here args can't be inferred apparently
      await walletStore[method](...args);

      expect(executeSpy).toHaveBeenCalledTimes(1);
      expect(executeSpy.mock.calls[0][0].attributes).toStrictEqual(expectedTx);
      expect(executeSpy.mock.calls[0][0].bookentry.profile).toStrictEqual(
        defaultProfile
      );

      if (isPhoenixTransfer) {
        expect(updateNonceSpy).not.toHaveBeenCalled();
        expect(updateCachedPendingNotesSpy).toHaveBeenCalledTimes(1);
        expect(updateCachedPendingNotesSpy).toHaveBeenCalledWith(
          phoenixTxResult.nullifiers,
          phoenixTxResult.hash
        );
        updateCachedPendingNotesSpy.mockClear();
      } else {
        expect(updateNonceSpy).toHaveBeenCalledTimes(1);
        expect(updateNonceSpy).toHaveBeenCalledWith(defaultProfile, newNonce);
        expect(updateCachedPendingNotesSpy).not.toHaveBeenCalled();
        setCachedBalanceSpy.mockClear();
      }

      // check that we made a sync before the transfer
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);

      // but the balance is not updated yet
      expect(balanceSpy).not.toHaveBeenCalled();

      // and neither the stake info
      expect(stakeInfoSpy).not.toHaveBeenCalled();

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
      expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(stakeInfoSpy).toHaveBeenCalledWith(defaultProfile.account);
      expect(balanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[1]
      );
      expect(balanceSpy.mock.invocationCallOrder[1]).toBeGreaterThan(
        treasuryUpdateSpy.mock.invocationCallOrder[1]
      );
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(defaultProfile, {
        shielded: await balanceSpy.mock.results[0].value,
        unshielded: await balanceSpy.mock.results[1].value,
      });
      expect(setCachedStakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(setCachedStakeInfoSpy).toHaveBeenCalledWith(
        defaultProfile,
        await stakeInfoSpy.mock.results[0].value
      );
      expect(setCachedBalanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        balanceSpy.mock.invocationCallOrder[1]
      );
      expect(setCachedStakeInfoSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        stakeInfoSpy.mock.invocationCallOrder[0]
      );

      vi.useFakeTimers();
    }

    beforeEach(async () => {
      walletStore.reset();
      expect(get(walletStore)).toStrictEqual(initialState);

      await walletStore.init(profileGenerator);
      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      vi.clearAllTimers();

      const currentStore = get(walletStore);

      expect(currentStore.profiles).toHaveLength(2);
      expect(currentStore.profiles[0]).toBe(defaultProfile);
      expect(currentStore.profiles[1]).toBeDefined();
      expect(currentStore.initialized).toBe(true);
      expect(currentStore.syncStatus.isInProgress).toBe(false);

      const { currentProfile } = currentStore;

      expect(currentProfile).toBeDefined();

      treasuryUpdateSpy.mockClear();
      balanceSpy.mockClear();
      stakeInfoSpy.mockClear();
      setCachedBalanceSpy.mockClear();
      setCachedStakeInfoSpy.mockClear();
    });

    afterEach(async () => {
      executeSpy.mockClear();
      updateNonceSpy.mockClear();
      updateCachedPendingNotesSpy.mockClear();
    });

    afterAll(() => {
      executeSpy.mockRestore();
      updateNonceSpy.mockRestore();
      updateCachedPendingNotesSpy.mockRestore();
    });

    /* eslint-disable vitest/expect-expect */

    it("should expose a method to claim the rewards", async () => {
      await walletStoreTransferCheck("claimRewards", [amount, gas]);
    });

    it("should expose a method to shield a given amount from the unshielded account", async () => {
      await walletStoreTransferCheck("shield", [amount, gas]);
    });

    it("should expose a method to execute a stake", async () => {
      await walletStoreTransferCheck("stake", [amount, gas]);
    });

    it("should expose a method to execute a phoenix transfer, if the receiver is a phoenix address", async () => {
      await walletStoreTransferCheck("transfer", [
        toPhoenix,
        amount,
        memo,
        gas,
      ]);
    });

    it("should use the moonlight account and shouldn't obfuscate the transaction if the receiver is a moonlight account", async () => {
      await walletStoreTransferCheck("transfer", [
        toMoonlight,
        amount,
        memo,
        gas,
      ]);
    });

    it("should expose a method to unshield a given amount from the shielded account", async () => {
      await walletStoreTransferCheck("unshield", [amount, gas]);
    });

    it("should expose a method to unstake the staked amount", async () => {
      await walletStoreTransferCheck("unstake", [amount, gas]);
    });

    /* eslint-enable vitest/expect-expect */
  });

  describe("Wallet store services", () => {
    const cacheClearSpy = vi.spyOn(WalletTreasury.prototype, "clearCache");

    beforeEach(async () => {
      walletStore.reset();
      expect(get(walletStore)).toStrictEqual(initialState);

      await walletStore.init(profileGenerator);
      await vi.advanceTimersByTimeAsync(AUTO_SYNC_INTERVAL - 1);

      vi.clearAllTimers();

      const currentStore = get(walletStore);

      expect(currentStore.profiles).toHaveLength(2);
      expect(currentStore.profiles[0]).toBe(defaultProfile);
      expect(currentStore.profiles[1]).toBeDefined();
      expect(currentStore.initialized).toBe(true);
      expect(currentStore.syncStatus.isInProgress).toBe(false);

      const { currentProfile } = currentStore;

      expect(currentProfile).toBeDefined();

      treasuryUpdateSpy.mockClear();
      balanceSpy.mockClear();
      cacheClearSpy.mockClear();
      stakeInfoSpy.mockClear();
      setCachedBalanceSpy.mockClear();
      setCachedStakeInfoSpy.mockClear();
    });

    afterAll(() => {
      cacheClearSpy.mockRestore();
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
      const finalState = get(walletStore);
      expect(finalState.profiles).toHaveLength(2);
      expect(finalState.profiles[0]).toBe(newProfile);
      expect(finalState.profiles[1]).toBeDefined(); // Don't check exact instance, just that it exists
      expect(finalState.currentProfile).toBe(newProfile);
      expect(finalState.initialized).toBe(true);
      expect(finalState.balance).toStrictEqual({ shielded, unshielded });
      expect(finalState.stakeInfo).toStrictEqual(stakeInfo);
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);
      expect(balanceSpy).toHaveBeenCalledTimes(2);
      expect(balanceSpy).toHaveBeenNthCalledWith(1, newProfile.address);
      expect(balanceSpy).toHaveBeenNthCalledWith(2, newProfile.account);
      expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(stakeInfoSpy).toHaveBeenCalledWith(newProfile.account);
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
      expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(stakeInfoSpy).toHaveBeenCalledWith(fakeExtraProfile.account);
      expect(setCachedBalanceSpy).toHaveBeenCalledTimes(1);
      expect(setCachedBalanceSpy).toHaveBeenCalledWith(fakeExtraProfile, {
        shielded: await balanceSpy.mock.results[0].value,
        unshielded: await balanceSpy.mock.results[1].value,
      });
      expect(setCachedStakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(setCachedStakeInfoSpy).toHaveBeenCalledWith(
        fakeExtraProfile,
        await stakeInfoSpy.mock.results[0].value
      );
      expect(setCachedBalanceSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        balanceSpy.mock.invocationCallOrder[1]
      );
      expect(setCachedStakeInfoSpy.mock.invocationCallOrder[0]).toBeGreaterThan(
        stakeInfoSpy.mock.invocationCallOrder[0]
      );
    });

    it("should reject with an error if the profile is not in the known list", async () => {
      // @ts-expect-error we don't care to set a real profile
      await expect(walletStore.setCurrentProfile({})).rejects.toThrow();
    });
  });
});
