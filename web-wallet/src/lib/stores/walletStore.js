import { get, writable } from "svelte/store";
import { getKey, uniquesBy } from "lamb";

/**
 * @typedef {import("@dusk-network/dusk-wallet-js").Wallet} Wallet
 */

/**
 * @typedef {import("./stores").WalletStoreServices["getTransactionsHistory"]} GetTransactionsHistory
 */

/** @type {Promise<void> | null} */
let syncPromise = null;

/** @type {Wallet | null} */
let walletInstance = null;

const uniquesById = uniquesBy(getKey("id"));

/** @type {import("./stores").WalletStoreContent} */
const initialState = {
	addresses: [],
	balance: {
		maximum: 0,
		value: 0
	},
	currentAddress: "",
	error: null,
	initialized: false,
	isSyncing: false
};

/* eslint-disable-next-line svelte/require-store-callbacks-use-set-param */
const walletStore = writable(initialState);
const { set, subscribe } = walletStore;

/**
 * Defensive code here as the `amount` and
 * `reward` properties can be `undefined` in the
 * returned stake info object.
 *
 * @param {WalletStakeInfo} stakeInfo
 * @returns {WalletStakeInfo}
 */
const fixStakeInfo = stakeInfo => ({
	...stakeInfo,
	amount: stakeInfo.amount ?? 0,
	reward: stakeInfo.reward ?? 0
});

const getCurrentAddress = () => get(walletStore).currentAddress;

/** @type {(action: (...args: any[]) => Promise<any>) => Promise<void>} */
const syncedAction = action => sync().then(action).finally(sync);

/** @type {() => Promise<void>} */
const clearLocalData = async () => walletInstance?.reset();

/** @type {(wallet: Wallet) => Promise<void>} */
const clearLocalDataAndInit = wallet => wallet.reset().then(() => init(wallet));

/** @type {import("./stores").WalletStoreServices["getStakeInfo"]} */
// @ts-expect-error
const getStakeInfo = async () => sync().then(() => walletInstance.stakeInfo(getCurrentAddress()))
	.then(fixStakeInfo);

/** @type {GetTransactionsHistory} */

// @ts-expect-error
const getTransactionsHistory = async () => sync().then(() => walletInstance.history(getCurrentAddress()))
	.then(uniquesById);

function reset () {
	walletInstance = null;
	set(initialState);
}

async function updateAfterSync () {
	const store = get(walletStore);

	// @ts-expect-error
	const balance = await walletInstance.getBalance(store.currentAddress);

	set({
		...store,
		balance,
		isSyncing: false
	});
}

/** @param {Wallet} wallet */
async function init (wallet) {
	walletInstance = wallet;

	const addresses = walletInstance.getPsks();
	const currentAddress = addresses[0];

	set({
		...initialState,
		addresses,
		currentAddress,
		initialized: true

	});
	sync();
}

/** @type {import("./stores").WalletStoreServices["setCurrentAddress"]} */
async function setCurrentAddress (address) {
	const store = get(walletStore);

	return store.addresses.includes(address)
		? Promise.resolve(set({ ...store, currentAddress: address })).then(sync)
		: Promise.reject(new Error("The received address is not in the list"));
}

/** @type {import("./stores").WalletStoreServices["stake"]} */
// @ts-expect-error
const stake = async (amount, gasPrice, gasLimit) => syncedAction(() => {
	if (walletInstance) {
		walletInstance.gasLimit = gasLimit;
		walletInstance.gasPrice = gasPrice;

		return walletInstance.stake(getCurrentAddress(), amount);
	}
});

/** @type {import("./stores").WalletStoreServices["sync"]} */
function sync () {
	if (!walletInstance) {
		throw new Error("No wallet instance to sync");
	}

	if (!syncPromise) {
		const store = get(walletStore);

		set({ ...store, error: null, isSyncing: true });

		syncPromise = walletInstance.sync().then(
			updateAfterSync,
			error => { set({ ...store, error, isSyncing: false }); }
		).finally(() => { syncPromise = null; });
	}

	return syncPromise;
}

/** @type {import("./stores").WalletStoreServices["transfer"]} */
// @ts-expect-error
const transfer = async (to, amount, gasPrice, gasLimit) => syncedAction(() => {
	if (walletInstance) {
		walletInstance.gasLimit = gasLimit;
		walletInstance.gasPrice = gasPrice;

		return walletInstance.transfer(
			getCurrentAddress(),
			to,
			amount
		);
	}

	return null;
});

/** @type {import("./stores").WalletStoreServices["unstake"]} */
// @ts-expect-error
const unstake = async (gasPrice, gasLimit) => syncedAction(() => {
	if (walletInstance) {
		walletInstance.gasLimit = gasLimit;
		walletInstance.gasPrice = gasPrice;

		return walletInstance.unstake(getCurrentAddress());
	}

	return null;
});

/** @type {import("./stores").WalletStoreServices["withdrawReward"]} */
// @ts-expect-error
const withdrawReward = async (gasPrice, gasLimit) => syncedAction(() => {
	if (walletInstance) {
		walletInstance.gasLimit = gasLimit;
		walletInstance.gasPrice = gasPrice;

		return walletInstance.withdrawReward(getCurrentAddress());
	}

	return null;
});

/** @type {import("./stores").WalletStore} */
export default {
	clearLocalData,
	clearLocalDataAndInit,
	getStakeInfo,
	getTransactionsHistory,
	init,
	reset,
	setCurrentAddress,
	stake,
	subscribe,
	sync,
	transfer,
	unstake,
	withdrawReward
};
