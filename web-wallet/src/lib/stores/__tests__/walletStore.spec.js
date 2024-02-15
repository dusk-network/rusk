import {
	afterAll,
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { get } from "svelte/store";
import { keys } from "lamb";
import { Wallet } from "@dusk-network/dusk-wallet-js";

import { addresses, transactions } from "$lib/mock-data";

import { walletStore } from "..";

vi.useFakeTimers();

describe("walletStore", async () => {
	const balance = { maximum: 100, value: 1 };
	const wallet = new Wallet([]);

	const getBalanceSpy = vi.spyOn(Wallet.prototype, "getBalance").mockResolvedValue(balance);
	const getPsksSpy = vi.spyOn(Wallet.prototype, "getPsks").mockResolvedValue(addresses);
	const historySpy = vi.spyOn(Wallet.prototype, "history").mockResolvedValue(transactions);
	const resetSpy = vi.spyOn(Wallet.prototype, "reset").mockResolvedValue(void 0);
	const stakeInfoSpy = vi.spyOn(Wallet.prototype, "stakeInfo").mockResolvedValue({});
	const stakeSpy = vi.spyOn(Wallet.prototype, "stake").mockResolvedValue(void 0);
	const syncSpy = vi.spyOn(Wallet.prototype, "sync").mockResolvedValue(void 0);
	const transferSpy = vi.spyOn(Wallet.prototype, "transfer").mockResolvedValue(void 0);
	const unstakeSpy = vi.spyOn(Wallet.prototype, "unstake").mockResolvedValue(void 0);
	const withdrawRewardSpy = vi.spyOn(Wallet.prototype, "withdrawReward").mockResolvedValue(void 0);

	const initialState = {
		balance: {
			maximum: 0,
			value: 0
		},
		currentAddress: "",
		error: null,
		initialized: false,
		isSyncing: false,
		addresses: []
	};
	const initializedStore = {
		...initialState,
		balance,
		currentAddress: addresses[0],
		initialized: true,
		addresses: addresses
	};
	const gas = {
		price: 1,
		limit: 30000000
	};

	afterEach(() => {
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
	});

	afterAll(() => {
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
			await vi.advanceTimersToNextTimerAsync();

			expect(get(walletStore)).toStrictEqual(initializedStore);

			walletStore.reset();

			expect(get(walletStore)).toStrictEqual(initialState);
		});

		it("should expose a method to initialize the store with a Wallet instance", async () => {
			await walletStore.init(wallet);

			expect(get(walletStore)).toStrictEqual({
				...initialState,
				currentAddress: addresses[0],
				error: null,
				initialized: true,
				isSyncing: true,
				addresses: addresses
			});

			expect(getPsksSpy).toHaveBeenCalledTimes(1);
			expect(getBalanceSpy).not.toHaveBeenCalled();

			await vi.advanceTimersToNextTimerAsync();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(syncSpy).toHaveBeenCalledWith({ signal: expect.any(AbortSignal) });
			expect(getBalanceSpy).toHaveBeenCalledTimes(1);
			expect(getBalanceSpy).toHaveBeenCalledWith(addresses[0]);
			expect(get(walletStore)).toStrictEqual(initializedStore);
		});

		it("should set the sync error in the store if the sync fails", async () => {
			walletStore.reset();

			const storeWhileLoading = {
				...initialState,
				currentAddress: addresses[0],
				error: null,
				initialized: true,
				isSyncing: true,
				addresses: addresses
			};
			const error = new Error("sync failed");

			syncSpy.mockRejectedValueOnce(error);

			await walletStore.init(wallet);

			expect(get(walletStore)).toStrictEqual(storeWhileLoading);
			expect(getPsksSpy).toHaveBeenCalledTimes(1);
			expect(getBalanceSpy).not.toHaveBeenCalled();

			await vi.advanceTimersToNextTimerAsync();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(syncSpy).toHaveBeenCalledWith({ signal: expect.any(AbortSignal) });
			expect(getBalanceSpy).not.toHaveBeenCalled();
			expect(get(walletStore)).toStrictEqual({
				...storeWhileLoading,
				error,
				isSyncing: false
			});

			walletStore.reset();
		});

		it("should throw an error when the synchronization is called without initializing the store first", async () => {
			walletStore.reset();

			expect(() => walletStore.sync()).toThrow();
		});

		it("should return the pending sync promise if a sync is called while another one is in progress", async () => {
			walletStore.reset();

			await walletStore.init(wallet);
			await vi.advanceTimersToNextTimerAsync();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(syncSpy).toHaveBeenCalledWith({ signal: expect.any(AbortSignal) });

			syncSpy.mockClear();

			const syncPromise1 = walletStore.sync();
			const syncPromise2 = walletStore.sync();
			const syncPromise3 = walletStore.sync();

			expect(syncPromise1).toBe(syncPromise2);
			expect(syncPromise1).toBe(syncPromise3);

			await syncPromise1;

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(syncSpy).toHaveBeenCalledWith({ signal: expect.any(AbortSignal) });

			const syncPromise4 = walletStore.sync();

			expect(syncPromise1).not.toBe(syncPromise4);
			expect(syncSpy).toHaveBeenCalledTimes(2);

			await syncPromise4;

			walletStore.reset();
		});
	});

	describe("Abort sync", () => {
		const abortControllerSpy = vi.spyOn(AbortController.prototype, "abort");

		afterEach(() => {
			abortControllerSpy.mockClear();
		});

		afterAll(() => {
			abortControllerSpy.mockRestore();
		});

		it("should expose a method to abort a sync that is in progress", async () => {
			await walletStore.init(wallet);

			walletStore.abortSync();

			await vi.advanceTimersToNextTimerAsync();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(syncSpy).toHaveBeenCalledWith({ signal: expect.any(AbortSignal) });
			expect(abortControllerSpy).toHaveBeenCalledTimes(1);
		});

		it("should do nothing if there is no sync in progress", async () => {
			walletStore.reset();

			await walletStore.init(wallet);
			await vi.advanceTimersToNextTimerAsync();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(syncSpy).toHaveBeenCalledWith({ signal: expect.any(AbortSignal) });

			syncSpy.mockClear();

			walletStore.abortSync();

			expect(abortControllerSpy).not.toHaveBeenCalled();
		});
	});

	describe("Wallet store services", () => {
		const currentAddress = addresses[0];

		afterEach(() => {
			walletStore.reset();
		});

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
			syncSpy.mockClear();
			walletStore.reset();

			await walletStore.clearLocalDataAndInit(wallet);

			expect(get(walletStore)).toStrictEqual({
				...initialState,
				currentAddress: addresses[0],
				error: null,
				initialized: true,
				isSyncing: true,
				addresses: addresses
			});

			await vi.advanceTimersToNextTimerAsync();

			expect(getPsksSpy).toHaveBeenCalledTimes(1);
			expect(getBalanceSpy).toHaveBeenCalledTimes(1);
			expect(getBalanceSpy).toHaveBeenCalledWith(addresses[0]);
			expect(syncSpy).toHaveBeenCalledTimes(1);

			await vi.advanceTimersToNextTimerAsync();

			expect(get(walletStore)).toStrictEqual(initializedStore);
		});

		it("should expose a method to retrieve the stake info", async () => {
			await walletStore.getStakeInfo();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
			expect(stakeInfoSpy).toHaveBeenCalledWith(currentAddress);
			expect(syncSpy.mock.invocationCallOrder[0])
				.toBeLessThan(stakeInfoSpy.mock.invocationCallOrder[0]);
		});

		it("should fix the returned stake info by adding the amount and the reward if they are missing", async () => {
			stakeInfoSpy.mockResolvedValueOnce({
				"has_key": false,
				"has_staked": false
			});

			const expected = {
				"amount": 0,
				"has_key": false,
				"has_staked": false,
				"reward": 0
			};
			const result = await walletStore.getStakeInfo();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(stakeInfoSpy).toHaveBeenCalledTimes(1);
			expect(stakeInfoSpy).toHaveBeenCalledWith(currentAddress);
			expect(syncSpy.mock.invocationCallOrder[0])
				.toBeLessThan(stakeInfoSpy.mock.invocationCallOrder[0]);
			expect(result).toStrictEqual(expected);
		});

		it("should expose a method to retrieve the transaction history", async () => {
			await walletStore.getTransactionsHistory();

			expect(syncSpy).toHaveBeenCalledTimes(1);
			expect(historySpy).toHaveBeenCalledTimes(1);
			expect(historySpy).toHaveBeenCalledWith(currentAddress);
			expect(syncSpy.mock.invocationCallOrder[0])
				.toBeLessThan(historySpy.mock.invocationCallOrder[0]);
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
			expect(get(walletStore).currentAddress).toBe(addresses[1]);
			expect(setCurrentAddressSpy.mock.invocationCallOrder[0])
				.toBeLessThan(syncSpy.mock.invocationCallOrder[0]);

			setCurrentAddressSpy.mockRestore();
		});

		it("should return a rejected promise if the new address is not in the list", () => {
			expect(walletStore.setCurrentAddress("foo bar")).rejects.toThrow();

			expect(syncSpy).not.toHaveBeenCalled();
			expect(get(walletStore).currentAddress).toBe(currentAddress);
		});

		it("should expose a method to allow to stake an amount of Dusk", async () => {
			await walletStore.stake(10, gas.price, gas.limit);

			expect(wallet.gasLimit).toBe(gas.limit);
			expect(wallet.gasPrice).toBe(gas.price);

			expect(syncSpy).toHaveBeenCalledTimes(2);
			expect(stakeSpy).toHaveBeenCalledTimes(1);
			expect(stakeSpy).toHaveBeenCalledWith(currentAddress, 10);
			expect(syncSpy.mock.invocationCallOrder[0])
				.toBeLessThan(stakeSpy.mock.invocationCallOrder[0]);
			expect(syncSpy.mock.invocationCallOrder[1])
				.toBeGreaterThan(stakeSpy.mock.invocationCallOrder[0]);
		});

		it("should expose a method to allow to transfer an amount of Dusk", async () => {
			await walletStore.transfer(addresses[1], 10, gas.price, gas.limit);

			expect(wallet.gasLimit).toBe(gas.limit);
			expect(wallet.gasPrice).toBe(gas.price);

			expect(syncSpy).toHaveBeenCalledTimes(2);
			expect(transferSpy).toHaveBeenCalledTimes(1);
			expect(transferSpy).toHaveBeenCalledWith(currentAddress, addresses[1], 10);
			expect(syncSpy.mock.invocationCallOrder[0])
				.toBeLessThan(transferSpy.mock.invocationCallOrder[0]);
			expect(syncSpy.mock.invocationCallOrder[1])
				.toBeGreaterThan(transferSpy.mock.invocationCallOrder[0]);
		});

		it("should expose a method to allow to unstake the current address", async () => {
			await walletStore.unstake(gas.price, gas.limit);

			expect(wallet.gasLimit).toBe(gas.limit);
			expect(wallet.gasPrice).toBe(gas.price);

			expect(syncSpy).toHaveBeenCalledTimes(2);
			expect(unstakeSpy).toHaveBeenCalledTimes(1);
			expect(unstakeSpy).toHaveBeenCalledWith(currentAddress);
			expect(syncSpy.mock.invocationCallOrder[0])
				.toBeLessThan(unstakeSpy.mock.invocationCallOrder[0]);
			expect(syncSpy.mock.invocationCallOrder[1])
				.toBeGreaterThan(unstakeSpy.mock.invocationCallOrder[0]);
		});

		it("should expose a method to allow to withdraw a reward", async () => {
			await walletStore.withdrawReward(gas.price, gas.limit);

			expect(wallet.gasLimit).toBe(gas.limit);
			expect(wallet.gasPrice).toBe(gas.price);

			expect(syncSpy).toHaveBeenCalledTimes(2);
			expect(withdrawRewardSpy).toHaveBeenCalledTimes(1);
			expect(withdrawRewardSpy).toHaveBeenCalledWith(currentAddress);
			expect(syncSpy.mock.invocationCallOrder[0])
				.toBeLessThan(withdrawRewardSpy.mock.invocationCallOrder[0]);
			expect(syncSpy.mock.invocationCallOrder[1])
				.toBeGreaterThan(withdrawRewardSpy.mock.invocationCallOrder[0]);
		});
	});

	describe("State changing failures", () => {
		/** @typedef {"stake" | "transfer" | "unstake" | "withdrawReward"} Operation */
		/** @type {Record<Operation, import("vitest").SpyInstance<any>>} */
		const operationsMap = {
			stake: stakeSpy,
			transfer: transferSpy,
			unstake: unstakeSpy,
			withdrawReward: withdrawRewardSpy
		};
		const fakeFailure = new Error("operation failure");
		const fakeSuccess = {};
		const fakeSyncError = new Error("bad sync");

		keys(operationsMap).forEach(operation => {
			const spy = operationsMap[operation];

			it("should return a resolved promise with the operation result if an operation succeeds", async () => {
				await walletStore.init(wallet);
				await vi.advanceTimersToNextTimerAsync();

				syncSpy
					.mockResolvedValueOnce(void 0)
					.mockRejectedValueOnce(fakeSyncError);
				spy.mockResolvedValueOnce(fakeSuccess);

				expect(get(walletStore).error).toBe(null);

				// @ts-ignore it's a mock and we don't care to pass the correct arguments
				expect(await walletStore[operation]()).toBe(fakeSuccess);
				expect(get(walletStore).error).toBe(fakeSyncError);

				walletStore.reset();
			});

			it("should return a rejected promise with the operation error if an operation fails and try a sync afterwards nonetheless", async () => {
				await walletStore.init(wallet);
				await vi.advanceTimersToNextTimerAsync();

				syncSpy
					.mockResolvedValueOnce(void 0)
					.mockRejectedValueOnce(fakeSyncError);
				spy.mockRejectedValueOnce(fakeFailure);

				expect(get(walletStore).error).toBe(null);

				// @ts-ignore it's a mock and we don't care to pass the correct arguments
				expect(walletStore[operation]()).rejects.toThrowError(fakeFailure);

				await vi.advanceTimersToNextTimerAsync();

				expect(get(walletStore).error).toBe(fakeSyncError);

				walletStore.reset();
			});
		});
	});
});
