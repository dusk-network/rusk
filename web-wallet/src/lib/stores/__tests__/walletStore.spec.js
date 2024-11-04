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
import * as b58 from "$lib/vendor/w3sper.js/src/b58";
import { generateMnemonic } from "bip39";

import walletCache from "$lib/wallet-cache";
import WalletTreasury from "$lib/wallet-treasury";
import { getSeedFromMnemonic } from "$lib/wallet";

import { walletStore } from "..";

describe("Wallet store", async () => {
  vi.useFakeTimers();

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
    currentProfile: defaultProfile,
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

  afterEach(async () => {
    await vi.runAllTimersAsync();
    abortControllerSpy.mockClear();
    setCachedBalanceSpy.mockClear();
    setProfilesSpy.mockClear();
    treasuryUpdateSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    abortControllerSpy.mockRestore();
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

      await vi.runAllTimersAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);
      expect(setProfilesSpy).toHaveBeenCalledTimes(1);
      expect(setProfilesSpy).toHaveBeenCalledWith([defaultProfile]);
      expect(setProfilesSpy.mock.invocationCallOrder[0]).toBeLessThan(
        treasuryUpdateSpy.mock.invocationCallOrder[0]
      );
      expect(treasuryUpdateSpy).toHaveBeenCalledTimes(1);
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
    });
  });

  describe("Abort sync", () => {
    beforeEach(async () => {
      await walletStore.init(profileGenerator);

      await vi.runAllTimersAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);

      // somewhere else (dexie?) the abort is called already six times
      abortControllerSpy.mockClear();
      treasuryUpdateSpy.mockClear();
    });

    it("should expose a method to abort a sync that is in progress and set the current sync promise to `null` so that a new sync can be started", async () => {
      walletStore.sync();

      await vi.waitUntil(() => treasuryUpdateSpy.mock.calls.length === 1);

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

    /** @type {string} */
    let fromAddress;

    /** @type {string} */
    let fromAccount;

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

    const executeSpy = vi
      .spyOn(Network.prototype, "execute")
      .mockResolvedValue(phoenixTxResult);
    const setPendingNotesSpy = vi.spyOn(walletCache, "setPendingNoteInfo");

    beforeEach(async () => {
      await walletStore.init(profileGenerator);
      await vi.runAllTimersAsync();

      const { currentProfile } = get(walletStore);

      fromAccount = /** @type {string} */ (currentProfile?.account.toString());
      fromAddress = /** @type {string} */ (currentProfile?.address.toString());

      treasuryUpdateSpy.mockClear();
      balanceSpy.mockClear();
      setCachedBalanceSpy.mockClear();
    });

    afterEach(async () => {
      await vi.runAllTimersAsync();

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
      vi.useRealTimers();

      await walletStore.clearLocalData();

      expect(cacheClearSpy).toHaveBeenCalledTimes(1);

      vi.useFakeTimers();
    });

    it("should expose a method to clear local data and init the wallet", async () => {
      const newGenerator = new ProfileGenerator(() => new Uint8Array());
      const newProfile = await newGenerator.default;

      walletStore.clearLocalDataAndInit(newGenerator, 99n);

      await vi.runAllTimersAsync();

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
      vi.useRealTimers();

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

      vi.useFakeTimers();
    });

    it("should reject with an error if the profile is not in the known list", async () => {
      // @ts-expect-error we don't care to set a real profile
      await expect(walletStore.setCurrentProfile({})).rejects.toThrow();
    });

    it("should expose a method to execute a phoenix transfer, if the receiver is a phoenix address", async () => {
      vi.useRealTimers();

      const expectedTx = {
        amount,
        from: fromAddress,
        gas,
        obfuscated: true,
        to: b58.decode(toPhoenix),
      };
      const result = await walletStore.transfer(toPhoenix, amount, gas);

      expect(executeSpy).toHaveBeenCalledTimes(1);

      // our TransactionBuilder mock is loaded
      expect(executeSpy.mock.calls[0][0].toJSON()).toStrictEqual(expectedTx);
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
        from: fromAccount,
        gas,
        obfuscated: false,
        to: b58.decode(toMoonlight),
      };

      await walletStore.transfer(toMoonlight, amount, gas);

      // our TransactionBuilder mock is loaded
      expect(executeSpy.mock.calls[0][0].toJSON()).toStrictEqual(expectedTx);
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
  });
});
