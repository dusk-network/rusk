import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { keys } from "lamb";
import { Gas, Wallet } from "@dusk-network/dusk-wallet-js";

import { addresses, transactions } from "$lib/mock-data";
import { rejectAfter, resolveAfter } from "$lib/dusk/test-helpers";

import { walletStore } from "..";
import { waitFor } from "@testing-library/svelte";

const settleTime = 1000;

vi.useFakeTimers();

describe("walletStore", async () => {
  const balance = { maximum: 100, value: 1 };
  const wallet = new Wallet([]);

  const defaultSyncOptions = {
    from: undefined,
    onblock: expect.any(Function),
    signal: expect.any(AbortSignal),
  };

  const abortControllerSpy = vi.spyOn(AbortController.prototype, "abort");
  const blockHeightSpy = vi
    .spyOn(Wallet, "networkBlockHeight", "get")
    .mockResolvedValue(1536);
  const getBalanceSpy = vi
    .spyOn(Wallet.prototype, "getBalance")
    .mockResolvedValue(balance);
  const getPsksSpy = vi
    .spyOn(Wallet.prototype, "getPsks")
    .mockResolvedValue(addresses);
  const historySpy = vi
    .spyOn(Wallet.prototype, "history")
    .mockResolvedValue(transactions);
  const resetSpy = vi
    .spyOn(Wallet.prototype, "reset")
    .mockResolvedValue(void 0);
  const stakeInfoSpy = vi
    .spyOn(Wallet.prototype, "stakeInfo")
    .mockResolvedValue({});
  const stakeSpy = vi
    .spyOn(Wallet.prototype, "stake")
    .mockResolvedValue(void 0);
  const syncSpy = vi.spyOn(Wallet.prototype, "sync").mockResolvedValue(void 0);
  const transferSpy = vi
    .spyOn(Wallet.prototype, "transfer")
    .mockResolvedValue(void 0);
  const unstakeSpy = vi
    .spyOn(Wallet.prototype, "unstake")
    .mockResolvedValue(void 0);
  const withdrawRewardSpy = vi
    .spyOn(Wallet.prototype, "withdrawReward")
    .mockResolvedValue(void 0);

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
    addresses: addresses,
    balance,
    currentAddress: addresses[0],
    initialized: true,
  };
  const gasSettings = {
    limit: 30000000,
    price: 1,
  };

  beforeEach(() => {
    abortControllerSpy.mockClear();
    blockHeightSpy.mockClear();
    getBalanceSpy.mockClear();
    getPsksSpy.mockClear();
    historySpy.mockClear();
    resetSpy.mockClear();
    stakeInfoSpy.mockClear();
    stakeSpy.mockClear();
    syncSpy.mockClear();
    transferSpy.mockClear();
    unstakeSpy.mockClear();
    withdrawRewardSpy.mockClear();
    vi.runAllTimers();
    walletStore.reset();
    vi.clearAllTimers();
  });

  afterAll(() => {
    abortControllerSpy.mockRestore();
    blockHeightSpy.mockRestore();
    getBalanceSpy.mockRestore();
    getPsksSpy.mockRestore();
    historySpy.mockRestore();
    resetSpy.mockRestore();
    stakeInfoSpy.mockRestore();
    stakeSpy.mockRestore();
    syncSpy.mockRestore();
    transferSpy.mockRestore();
    unstakeSpy.mockRestore();
    withdrawRewardSpy.mockRestore();
  });

  describe("Initialization and sync", () => {
    it("should expose a `reset` method to bring back the store to its initial state", async () => {
      await walletStore.init(wallet);

      expect(syncSpy).toHaveBeenCalledTimes(1);

      await vi.advanceTimersToNextTimerAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);

      walletStore.reset();

      expect(get(walletStore)).toStrictEqual(initialState);
    });

    it("should abort a sync in progress during the reset and set an error in the store", async () => {
      walletStore.init(wallet);

      await waitFor(() => syncSpy.mock.calls.length === 1);

      walletStore.reset();

      expect(abortControllerSpy).toHaveBeenCalledOnce();
      expect(get(walletStore)).toStrictEqual(initialState);

      await vi.advanceTimersToNextTimerAsync();

      expect(get(walletStore)).toStrictEqual({
        ...initializedStore,
        balance: initialState.balance,
        syncStatus: {
          current: 0,
          error: new Error("Synchronization aborted"),
          isInProgress: false,
          last: 0,
        },
      });
    });

    it("should expose a method to initialize the store with a Wallet instance", async () => {
      await walletStore.init(wallet);

      expect(get(walletStore)).toStrictEqual({
        ...initialState,
        addresses: addresses,
        currentAddress: addresses[0],
        initialized: true,
        syncStatus: { current: 0, error: null, isInProgress: true, last: 0 },
      });

      expect(getPsksSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);

      await vi.advanceTimersToNextTimerAsync();

      expect(getBalanceSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).toHaveBeenCalledWith(addresses[0]);

      await vi.advanceTimersToNextTimerAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);
    });

    it("should allow to start the sync from a specific block height after initializing the wallet", async () => {
      const from = 9999;

      await walletStore.init(wallet, from);

      expect(get(walletStore)).toStrictEqual({
        ...initialState,
        addresses: addresses,
        currentAddress: addresses[0],
        initialized: true,
        syncStatus: { current: 0, error: null, isInProgress: true, last: 0 },
      });

      expect(getPsksSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).not.toHaveBeenCalled();

      await vi.advanceTimersToNextTimerAsync();

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith({ ...defaultSyncOptions, from });
      expect(getBalanceSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).toHaveBeenCalledWith(addresses[0]);
      expect(get(walletStore)).toStrictEqual(initializedStore);
    });

    it("should set the sync error in the store if the sync fails", async () => {
      const storeWhileLoading = {
        ...initialState,
        addresses: addresses,
        currentAddress: addresses[0],
        initialized: true,
        syncStatus: { current: 0, error: null, isInProgress: true, last: 0 },
      };
      const error = new Error("sync failed");

      syncSpy.mockImplementationOnce(() => rejectAfter(settleTime, error));

      await walletStore.init(wallet);

      expect(get(walletStore)).toStrictEqual(storeWhileLoading);
      expect(getPsksSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).not.toHaveBeenCalled();

      await vi.advanceTimersByTimeAsync(settleTime);

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);
      expect(getBalanceSpy).not.toHaveBeenCalled();
      expect(get(walletStore)).toStrictEqual({
        ...storeWhileLoading,
        syncStatus: { current: 0, error, isInProgress: false, last: 0 },
      });
    });

    it("should throw an error when the synchronization is called without initializing the store first", async () => {
      expect(() => walletStore.sync()).toThrow();
    });

    it("should return the pending sync promise if a sync is called while another one is in progress", async () => {
      await walletStore.init(wallet);
      await vi.advanceTimersToNextTimerAsync();

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);

      syncSpy.mockClear();

      const syncPromise1 = walletStore.sync();
      const syncPromise2 = walletStore.sync();
      const syncPromise3 = walletStore.sync();

      expect(syncPromise1).toBe(syncPromise2);
      expect(syncPromise1).toBe(syncPromise3);

      await syncPromise1;

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);

      const syncPromise4 = walletStore.sync();

      expect(syncPromise1).not.toBe(syncPromise4);
      expect(syncSpy).toHaveBeenCalledTimes(2);

      await syncPromise4;
    });
  });

  describe("Abort sync", () => {
    it("should expose a method to abort a sync that is in progress", async () => {
      await walletStore.init(wallet);

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);

      walletStore.abortSync();

      expect(abortControllerSpy).toHaveBeenCalledTimes(1);
    });

    it("should set to `null` the current sync promise so that a new call to `sync` will start a new synchronization", async () => {
      await walletStore.init(wallet);

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);

      walletStore.abortSync();

      expect(abortControllerSpy).toHaveBeenCalledTimes(1);

      walletStore.sync();

      expect(syncSpy).toHaveBeenCalledTimes(2);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);
    });

    it("should do nothing if there is no sync in progress", async () => {
      await walletStore.init(wallet);

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);

      await vi.advanceTimersToNextTimerAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);

      walletStore.abortSync();

      expect(abortControllerSpy).not.toHaveBeenCalled();
    });
  });

  describe("Wallet store services", () => {
    const currentAddress = addresses[0];

    beforeEach(async () => {
      await walletStore.init(wallet);
      await vi.advanceTimersToNextTimerAsync();

      syncSpy.mockClear();
    });

    it("should expose a method to clear local data", async () => {
      await walletStore.clearLocalData();

      expect(resetSpy).toHaveBeenCalledTimes(1);
    });

    it("should expose a method to clear local data and then init the wallet", async () => {
      getPsksSpy.mockClear();
      getBalanceSpy.mockClear();
      syncSpy
        .mockClear()
        .mockImplementationOnce(() => resolveAfter(settleTime, undefined));

      await walletStore.clearLocalDataAndInit(wallet);

      expect(get(walletStore)).toStrictEqual({
        ...initialState,
        addresses: addresses,
        currentAddress: addresses[0],
        initialized: true,
        syncStatus: { current: 0, error: null, isInProgress: true, last: 0 },
      });

      expect(getPsksSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).not.toHaveBeenCalled();
      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);

      await vi.advanceTimersByTimeAsync(settleTime);

      expect(getBalanceSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).toHaveBeenCalledWith(addresses[0]);
      expect(get(walletStore)).toStrictEqual(initializedStore);
    });

    it("should allow to start the sync from a specific block height after clearing and initializing the wallet", async () => {
      getPsksSpy.mockClear();
      getBalanceSpy.mockClear();
      syncSpy.mockClear();
      walletStore.reset();

      const from = 4276;

      await walletStore.clearLocalDataAndInit(wallet, from);

      expect(get(walletStore)).toStrictEqual({
        ...initialState,
        addresses: addresses,
        currentAddress: addresses[0],
        initialized: true,
        syncStatus: {
          ...initialState.syncStatus,
          error: null,
          isInProgress: true,
        },
      });

      await vi.advanceTimersToNextTimerAsync();

      expect(getPsksSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).toHaveBeenCalledTimes(1);
      expect(getBalanceSpy).toHaveBeenCalledWith(addresses[0]);
      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith({
        ...defaultSyncOptions,
        from,
      });

      await vi.advanceTimersToNextTimerAsync();

      expect(get(walletStore)).toStrictEqual(initializedStore);
    });

    it("should expose a method to retrieve the current block height", async () => {
      // This method needs to work even without a wallet instance
      walletStore.reset();

      await walletStore.getCurrentBlockHeight();

      expect(blockHeightSpy).toHaveBeenCalledTimes(1);
    });

    it("should expose a method to retrieve the stake info", async () => {
      await walletStore.getStakeInfo();

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);
      expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(stakeInfoSpy).toHaveBeenCalledWith(currentAddress);
      expect(syncSpy.mock.invocationCallOrder[0]).toBeLessThan(
        stakeInfoSpy.mock.invocationCallOrder[0]
      );
    });

    it("should fix the returned stake info by adding the amount and the reward if they are missing", async () => {
      stakeInfoSpy.mockResolvedValueOnce({
        /* eslint-disable camelcase */
        has_key: false,
        has_staked: false,
        /* eslint-disable camelcase */
      });

      const expected = {
        /* eslint-disable camelcase */
        amount: 0,
        has_key: false,
        has_staked: false,
        reward: 0,
        /* eslint-disable camelcase */
      };
      const result = await walletStore.getStakeInfo();

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);
      expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(stakeInfoSpy).toHaveBeenCalledWith(currentAddress);
      expect(syncSpy.mock.invocationCallOrder[0]).toBeLessThan(
        stakeInfoSpy.mock.invocationCallOrder[0]
      );
      expect(result).toStrictEqual(expected);
    });

    it("should expose a method to retrieve the transaction history", async () => {
      await walletStore.getTransactionsHistory();

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);
      expect(historySpy).toHaveBeenCalledTimes(1);
      expect(historySpy).toHaveBeenCalledWith(currentAddress);
      expect(syncSpy.mock.invocationCallOrder[0]).toBeLessThan(
        historySpy.mock.invocationCallOrder[0]
      );
    });

    it("should remove eventual duplicate transactions from the list", async () => {
      historySpy.mockResolvedValueOnce(transactions.concat(transactions));

      const result = await walletStore.getTransactionsHistory();

      expect(result).toStrictEqual(transactions);
    });

    it("should expose a method to set the current address", async () => {
      const setCurrentAddressSpy = vi.spyOn(walletStore, "setCurrentAddress");

      await walletStore.setCurrentAddress(addresses[1]);

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);
      expect(get(walletStore).currentAddress).toBe(addresses[1]);
      expect(setCurrentAddressSpy.mock.invocationCallOrder[0]).toBeLessThan(
        syncSpy.mock.invocationCallOrder[0]
      );

      setCurrentAddressSpy.mockRestore();
    });

    it("should return a rejected promise if the new address is not in the list", () => {
      expect(walletStore.setCurrentAddress("foo bar")).rejects.toThrow();

      expect(syncSpy).not.toHaveBeenCalled();
      expect(get(walletStore).currentAddress).toBe(currentAddress);
    });

    it("should expose a method to allow to stake an amount of Dusk", async () => {
      await walletStore.stake(10, gasSettings);

      expect(syncSpy).toHaveBeenCalledTimes(2);
      expect(syncSpy).toHaveBeenNthCalledWith(1, defaultSyncOptions);
      expect(syncSpy).toHaveBeenNthCalledWith(2, defaultSyncOptions);
      expect(stakeSpy).toHaveBeenCalledTimes(1);
      expect(stakeSpy).toHaveBeenCalledWith(currentAddress, 10, gasSettings);
      expect(stakeSpy.mock.calls[0][2]).toBeInstanceOf(Gas);
      expect(syncSpy.mock.invocationCallOrder[0]).toBeLessThan(
        stakeSpy.mock.invocationCallOrder[0]
      );
      expect(syncSpy.mock.invocationCallOrder[1]).toBeGreaterThan(
        stakeSpy.mock.invocationCallOrder[0]
      );
    });

    it("should expose a method to manually start a synchronization", async () => {
      await walletStore.sync();

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith(defaultSyncOptions);
    });

    it("should allow to start a synchronization from a specific block height", async () => {
      const from = 7654;

      await walletStore.sync(from);

      expect(syncSpy).toHaveBeenCalledTimes(1);
      expect(syncSpy).toHaveBeenCalledWith({
        ...defaultSyncOptions,
        from,
      });
    });

    it("should expose a method to allow to transfer an amount of Dusk", async () => {
      await walletStore.transfer(addresses[1], 10, gasSettings);

      expect(syncSpy).toHaveBeenCalledTimes(2);
      expect(syncSpy).toHaveBeenNthCalledWith(1, defaultSyncOptions);
      expect(syncSpy).toHaveBeenNthCalledWith(2, defaultSyncOptions);
      expect(transferSpy).toHaveBeenCalledTimes(1);
      expect(transferSpy).toHaveBeenCalledWith(
        currentAddress,
        addresses[1],
        10,
        gasSettings
      );
      expect(transferSpy.mock.calls[0][3]).toBeInstanceOf(Gas);
      expect(syncSpy.mock.invocationCallOrder[0]).toBeLessThan(
        transferSpy.mock.invocationCallOrder[0]
      );
      expect(syncSpy.mock.invocationCallOrder[1]).toBeGreaterThan(
        transferSpy.mock.invocationCallOrder[0]
      );
    });

    it("should expose a method to allow to unstake the current address", async () => {
      await walletStore.unstake(gasSettings);

      expect(syncSpy).toHaveBeenCalledTimes(2);
      expect(syncSpy).toHaveBeenNthCalledWith(1, defaultSyncOptions);
      expect(syncSpy).toHaveBeenNthCalledWith(2, defaultSyncOptions);
      expect(unstakeSpy).toHaveBeenCalledTimes(1);
      expect(unstakeSpy).toHaveBeenCalledWith(currentAddress, gasSettings);
      expect(unstakeSpy.mock.calls[0][1]).toBeInstanceOf(Gas);
      expect(syncSpy.mock.invocationCallOrder[0]).toBeLessThan(
        unstakeSpy.mock.invocationCallOrder[0]
      );
      expect(syncSpy.mock.invocationCallOrder[1]).toBeGreaterThan(
        unstakeSpy.mock.invocationCallOrder[0]
      );
    });

    it("should expose a method to allow to withdraw a reward", async () => {
      await walletStore.withdrawReward(gasSettings);

      expect(syncSpy).toHaveBeenCalledTimes(2);
      expect(syncSpy).toHaveBeenNthCalledWith(1, defaultSyncOptions);
      expect(syncSpy).toHaveBeenNthCalledWith(2, {
        ...defaultSyncOptions,
      });
      expect(withdrawRewardSpy).toHaveBeenCalledTimes(1);
      expect(withdrawRewardSpy).toHaveBeenCalledWith(
        currentAddress,
        gasSettings
      );
      expect(withdrawRewardSpy.mock.calls[0][1]).toBeInstanceOf(Gas);
      expect(syncSpy.mock.invocationCallOrder[0]).toBeLessThan(
        withdrawRewardSpy.mock.invocationCallOrder[0]
      );
      expect(syncSpy.mock.invocationCallOrder[1]).toBeGreaterThan(
        withdrawRewardSpy.mock.invocationCallOrder[0]
      );
    });
  });

  describe("State changing failures", () => {
    /** @typedef {"stake" | "transfer" | "unstake" | "withdrawReward"} Operation */
    /** @type {Record<Operation, import("vitest").MockInstance<any>>} */
    const operationsMap = {
      stake: stakeSpy,
      transfer: transferSpy,
      unstake: unstakeSpy,
      withdrawReward: withdrawRewardSpy,
    };
    const fakeFailure = new Error("operation failure");
    const fakeSuccess = {};
    const fakeSyncError = new Error("bad sync");

    keys(operationsMap).forEach((operation) => {
      const spy = operationsMap[operation];

      it("should return a resolved promise with the operation result if an operation succeeds even if the last sync fails", async () => {
        await walletStore.init(wallet);
        await vi.advanceTimersToNextTimerAsync();

        syncSpy
          .mockResolvedValueOnce(void 0)
          .mockRejectedValueOnce(fakeSyncError);
        spy.mockResolvedValueOnce(fakeSuccess);

        expect(get(walletStore).syncStatus.error).toBe(null);

        // @ts-ignore it's a mock and we don't care to pass the correct arguments
        expect(await walletStore[operation]()).toBe(fakeSuccess);
        expect(get(walletStore).syncStatus.error).toBe(fakeSyncError);
        expect(syncSpy).toHaveBeenCalledTimes(3);
        expect(syncSpy).toHaveBeenNthCalledWith(1, defaultSyncOptions);
        expect(syncSpy).toHaveBeenNthCalledWith(2, defaultSyncOptions);
        expect(syncSpy).toHaveBeenNthCalledWith(3, defaultSyncOptions);

        walletStore.reset();
      });

      it("should return a rejected promise with the operation error if an operation fails and try a sync afterwards", async () => {
        await walletStore.init(wallet);
        await vi.advanceTimersToNextTimerAsync();

        syncSpy
          .mockResolvedValueOnce(void 0)
          .mockRejectedValueOnce(fakeSyncError);
        spy.mockRejectedValueOnce(fakeFailure);

        expect(get(walletStore).syncStatus.error).toBe(null);

        // @ts-ignore it's a mock and we don't care to pass the correct arguments
        expect(walletStore[operation]()).rejects.toThrowError(fakeFailure);

        await vi.advanceTimersToNextTimerAsync();

        expect(get(walletStore).syncStatus.error).toBe(fakeSyncError);
        expect(syncSpy).toHaveBeenCalledTimes(3);
        expect(syncSpy).toHaveBeenNthCalledWith(1, defaultSyncOptions);
        expect(syncSpy).toHaveBeenNthCalledWith(2, defaultSyncOptions);
        expect(syncSpy).toHaveBeenNthCalledWith(3, defaultSyncOptions);
      });
    });
  });
});
