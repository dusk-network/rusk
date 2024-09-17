import { get, writable } from "svelte/store";
import { setKey } from "lamb";
import { Bookkeeper, Bookmark } from "$lib/vendor/w3sper.js/src/mod";

import walletCache from "$lib/wallet-cache";

import { transactions } from "$lib/mock-data";

import networkStore from "./networkStore";
import settingsStore from "./settingsStore";

/** @type {AbortController | null} */
let syncController = null;

/** @type {Promise<void> | null} */
let syncPromise = null;

/** @type {WalletStoreContent} */
const initialState = {
  addresses: [],
  balance: {
    maximum: 0n,
    value: 0n,
  },
  currentAddress: "",
  currentProfile: null,
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

const walletStore = writable(initialState);
const { set, subscribe, update } = walletStore;

const bookkeeper = new Bookkeeper(walletCache.treasury);

/** @type {(...args: any) => Promise<void>} */
const asyncNoopFailure = () => Promise.reject(new Error("Not implemented"));

/** @type {WalletStoreServices["abortSync"]} */
const abortSync = () => {
  syncPromise && syncController?.abort();
  syncPromise = null;
};

/** @type {WalletStoreServices["clearLocalData"]} */
const clearLocalData = () => {
  abortSync();

  return walletCache.clear();
};

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
  const currentProfile = await profileGenerator.default;
  const currentAddress = currentProfile.address.toString();

  set({
    ...initialState,
    addresses: [currentAddress],
    currentAddress,
    currentProfile,
    initialized: true,
    profiles: [currentProfile],
  });

  sync(syncFromBlock).then(() => {
    settingsStore.update(setKey("userId", currentAddress));
  });
}

/** @type {WalletStoreServices["reset"]} */
function reset() {
  abortSync();
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
async function sync(fromBlock) {
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
        current: 0n,
        error: null,
        isInProgress: true,
        last: 0n,
        progress: 0,
      },
    });

    syncController = new AbortController();

    const addressSyncer = await networkStore.getAddressSyncer({
      signal: syncController.signal,
    });

    /*
     * Unless the user wants to sync from a specific block height,
     * we restart from the last stored bookmark.
     */
    const from =
      fromBlock ?? Bookmark.from((await walletCache.getSyncInfo()).bookmark);

    let lastBlockHeight = 0n;

    // @ts-ignore
    addressSyncer.addEventListener("synciteration", ({ detail }) => {
      update((currentStore) => ({
        ...currentStore,
        syncStatus: {
          ...currentStore.syncStatus,
          current: detail.blocks.current,
          last: detail.blocks.last,
          progress: detail.progress,
        },
      }));

      lastBlockHeight = detail.blocks.last;
    });

    syncPromise = Promise.resolve(syncController.signal)
      .then(async (signal) => {
        const notesStream = await addressSyncer.notes(store.profiles, {
          from,
          signal,
        });

        for await (const [notesInfo, syncInfo] of notesStream) {
          await walletCache.addUnspentNotes(
            walletCache.toCacheNotes(notesInfo, store.profiles),
            syncInfo
          );
        }

        // updating the last block height in the cache sync info
        await walletCache.setLastBlockHeight(lastBlockHeight);

        // gather all unspent nullifiers in the cache
        const currentUnspentNullifiers =
          await walletCache.getUnspentNotesNullifiers();

        // retrieve the nullifiers that are now spent
        const spentNullifiers = await addressSyncer.spent(
          currentUnspentNullifiers
        );

        // update the cache with the spent nullifiers info
        await walletCache.spendNotes(spentNullifiers);

        // calculate the balance for the current selected address
        const balance = await bookkeeper.balance(store.currentProfile?.address);

        update((currentStore) => ({
          ...currentStore,
          balance: {
            maximum: balance.spendable,
            value: balance.value,
          },
        }));
      })
      .then(() => {
        if (syncController?.signal.aborted) {
          throw new Error("Synchronization aborted");
        }
      })
      .then(() => {
        update((currentStore) => ({
          ...currentStore,
          syncStatus: initialState.syncStatus,
        }));
      })
      .catch((error) => {
        syncController?.abort();

        update((currentStore) => ({
          ...currentStore,
          syncStatus: {
            current: 0n,
            error,
            isInProgress: false,
            last: 0n,
            progress: 0,
          },
        }));
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
