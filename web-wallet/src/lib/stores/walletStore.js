import { get, writable } from "svelte/store";
import { getKey, uniquesBy } from "lamb";

/**
 * @typedef {import("@dusk-network/dusk-wallet-js").Wallet} Wallet
 */

/**
 * @typedef {WalletStoreServices["getTransactionsHistory"]} GetTransactionsHistory
 */

/** @type {AbortController} */
let syncController;

/** @type {Promise<void> | null} */
let syncPromise = null;

/** @type {Wallet | null} */
let walletInstance = null;

const uniquesById = uniquesBy(getKey("id"));

/** @type {WalletStoreContent} */
const initialState = {
  addresses: [],
  balance: {
    maximum: 0,
    value: 0,
  },
  currentAddress: "",
  error: null,
  initialized: false,
  isSyncing: false,
};

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
const fixStakeInfo = (stakeInfo) => ({
  ...stakeInfo,
  amount: stakeInfo.amount ?? 0,
  reward: stakeInfo.reward ?? 0,
});

const getCurrentAddress = () => get(walletStore).currentAddress;

/** @type {(action: (...args: any[]) => Promise<any>) => Promise<void>} */
const syncedAction = (action) => sync().then(action).finally(sync);

const abortSync = () => syncPromise && syncController?.abort();

/** @type {() => Promise<void>} */
const clearLocalData = async () => walletInstance?.reset();

/** @type {(wallet: Wallet) => Promise<void>} */
const clearLocalDataAndInit = (wallet) =>
  wallet.reset().then(() => init(wallet));

/** @type {WalletStoreServices["getStakeInfo"]} */
const getStakeInfo = async () =>
  sync()
    // @ts-expect-error
    .then(() => walletInstance.stakeInfo(getCurrentAddress()))
    .then(fixStakeInfo);

/** @type {GetTransactionsHistory} */

const getTransactionsHistory = async () =>
  sync()
    // @ts-expect-error
    .then(() => walletInstance.history(getCurrentAddress()))
    .then(uniquesById);

function reset() {
  walletInstance = null;
  set(initialState);
}

async function updateAfterSync() {
  const store = get(walletStore);

  // @ts-expect-error
  const balance = await walletInstance.getBalance(store.currentAddress);

  set({
    ...store,
    balance,
    isSyncing: false,
  });
}

/** @param {Wallet} wallet */
async function init(wallet) {
  walletInstance = wallet;

  const addresses = await walletInstance.getPsks();
  const currentAddress = addresses[0];

  set({
    ...initialState,
    addresses,
    currentAddress,
    initialized: true,
  });
  sync();
}

/** @type {WalletStoreServices["setCurrentAddress"]} */
async function setCurrentAddress(address) {
  const store = get(walletStore);

  return store.addresses.includes(address)
    ? Promise.resolve(set({ ...store, currentAddress: address })).then(sync)
    : Promise.reject(new Error("The received address is not in the list"));
}

/** @type {WalletStoreServices["stake"]} */

const stake = async (amount, gasPrice, gasLimit) =>
  syncedAction(() => {
    // @ts-expect-error
    walletInstance.gasLimit = gasLimit;

    // @ts-expect-error
    walletInstance.gasPrice = gasPrice;

    // @ts-expect-error
    return walletInstance.stake(getCurrentAddress(), amount);
  });

/** @type {WalletStoreServices["sync"]} */
function sync() {
  if (!walletInstance) {
    throw new Error("No wallet instance to sync");
  }

  if (!syncPromise) {
    const store = get(walletStore);

    set({ ...store, error: null, isSyncing: true });

    syncController = new AbortController();
    syncPromise = walletInstance
      .sync({ signal: syncController.signal })
      .then(updateAfterSync, (error) => {
        set({ ...store, error, isSyncing: false });
      })
      .finally(() => {
        syncPromise = null;
      });
  }

  return syncPromise;
}

/** @type {WalletStoreServices["transfer"]} */
const transfer = async (to, amount, gasPrice, gasLimit) =>
  syncedAction(() => {
    // @ts-expect-error
    walletInstance.gasLimit = gasLimit;

    // @ts-expect-error
    walletInstance.gasPrice = gasPrice;

    // @ts-expect-error
    return walletInstance.transfer(getCurrentAddress(), to, amount);
  });

/** @type {WalletStoreServices["unstake"]} */
const unstake = async (gasPrice, gasLimit) =>
  syncedAction(() => {
    // @ts-expect-error
    walletInstance.gasLimit = gasLimit;

    // @ts-expect-error
    walletInstance.gasPrice = gasPrice;

    // @ts-expect-error
    return walletInstance.unstake(getCurrentAddress());
  });

/** @type {WalletStoreServices["withdrawReward"]} */
const withdrawReward = async (gasPrice, gasLimit) =>
  syncedAction(() => {
    // @ts-expect-error
    walletInstance.gasLimit = gasLimit;

    // @ts-expect-error
    walletInstance.gasPrice = gasPrice;

    // @ts-expect-error
    return walletInstance.withdrawReward(getCurrentAddress());
  });

/** @type {WalletStore} */
export default {
  abortSync,
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
  withdrawReward,
};
