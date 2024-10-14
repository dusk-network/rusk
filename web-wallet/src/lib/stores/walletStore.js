import { get, writable } from "svelte/store";
import { setKey } from "lamb";

import walletCache from "$lib/wallet-cache";
import { resolveAfter } from "$lib/dusk/promise";

import { transactions } from "$lib/mock-data";

import settingsStore from "./settingsStore";

/** @type {Promise<void> | null} */
let syncPromise = null;

/** @type {WalletStoreContent} */
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

const walletStore = writable(initialState);
const { set, subscribe } = walletStore;

/** @type {(...args: any) => Promise<void>} */
const asyncNoopFailure = () => Promise.reject(new Error("Not implemented"));

/** @type {WalletStoreServices["abortSync"]} */
const abortSync = () => {};

/** @type {WalletStoreServices["clearLocalData"]} */
const clearLocalData = () => walletCache.clear();

/** @type {WalletStoreServices["clearLocalDataAndInit"]} */
const clearLocalDataAndInit = (profileGenerator, syncFromBlock) =>
  clearLocalData().then(() => {
    return init(profileGenerator, syncFromBlock);
  });

/** @type {WalletStoreServices["getStakeInfo"]} */
const getStakeInfo = async () => ({ amount: 0, reward: 0 });

/** @type {WalletStoreServices["getTransactionsHistory"]} */
const getTransactionsHistory = async () => transactions;

/** @type {WalletStoreServices["init"]} */
async function init(profileGenerator, syncFromBlock) {
  const currentAddress = (await profileGenerator.default).address.toString();

  set({
    ...initialState,
    addresses: [currentAddress],
    currentAddress,
    initialized: true,
  });

  sync(syncFromBlock).then(() => {
    settingsStore.update(setKey("userId", currentAddress));
  });
}

/** @type {WalletStoreServices["reset"]} */
function reset() {
  set(initialState);
}

/** @type {WalletStoreServices["setCurrentAddress"]} */
async function setCurrentAddress(address) {
  const store = get(walletStore);

  return store.addresses.includes(address)
    ? Promise.resolve(set({ ...store, currentAddress: address })).then(() =>
        sync()
      )
    : Promise.reject(new Error("The received address is not in the list"));
}

/** @type {WalletStoreServices["stake"]} */
const stake = async (amount, gasSettings) =>
  asyncNoopFailure(amount, gasSettings);

/** @type {WalletStoreServices["sync"]} */
async function sync(/* from */) {
  const store = get(walletStore);

  if (!store.initialized) {
    throw new Error(
      "The wallet store needs to be initialized with a profile generator"
    );
  }

  if (!syncPromise) {
    set({
      ...store,
      syncStatus: {
        current: 0,
        error: null,
        isInProgress: true,
        last: 0,
      },
    });

    syncPromise = resolveAfter(1000, undefined)
      .then(() => {
        set({
          ...store,
          balance: { maximum: 1234, value: 567 },
          syncStatus: initialState.syncStatus,
        });
      })
      .catch((error) => {
        set({
          ...store,
          syncStatus: {
            current: 0,
            error,
            isInProgress: false,
            last: 0,
          },
        });
      })
      .finally(() => {
        syncPromise = null;
      });
  }

  return syncPromise;
}

/** @type {WalletStoreServices["transfer"]} */
const transfer = async (to, amount, gasSettings) =>
  asyncNoopFailure(to, amount, gasSettings);

/** @type {WalletStoreServices["unstake"]} */
const unstake = async (gasSettings) => asyncNoopFailure(gasSettings);

/** @type {WalletStoreServices["withdrawReward"]} */
const withdrawReward = async (gasSettings) => asyncNoopFailure(gasSettings);

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
