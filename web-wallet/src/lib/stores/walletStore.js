import { get, writable } from "svelte/store";
import { map, setKey } from "lamb";
import {
  Bookkeeper,
  Bookmark,
  ProfileGenerator,
} from "$lib/vendor/w3sper.js/src/mod";
import * as b58 from "$lib/vendor/w3sper.js/src/b58";

import walletCache from "$lib/wallet-cache";
import { nullifiersDifference } from "$lib/wallet";

import { transactions } from "$lib/mock-data";

import networkStore from "./networkStore";
import settingsStore from "./settingsStore";

/** @type {AbortController | null} */
let syncController = null;

/** @type {Promise<void> | null} */
let syncPromise = null;

/** @type {WalletStoreContent} */
const initialState = {
  balance: {
    shielded: {
      spendable: 0n,
      value: 0n,
    },
  },
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

/** @type {<T>(action: (...args: any[]) => Promise<T>) => Promise<T>} */
const effectfulAction = (action) => sync().then(action).finally(updateBalance);

const getCurrentAddress = () => get(walletStore).currentProfile?.address;

/** @type {(...args: any) => Promise<void>} */
const asyncNoopFailure = () => Promise.reject(new Error("Not implemented"));

/** @type {() => Promise<void>} */
const updateBalance = async () => {
  const { currentProfile } = get(walletStore);

  if (!currentProfile) {
    return;
  }

  const shielded = await bookkeeper.balance(currentProfile.address);

  update((currentStore) => ({
    ...currentStore,
    balance: { shielded },
  }));
};

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
    currentProfile,
    initialized: true,
    profiles: [currentProfile],
  });

  sync(syncFromBlock)
    .then(() => {
      settingsStore.update(setKey("userId", currentAddress));
    })
    .finally(updateBalance);
}

/** @type {WalletStoreServices["reset"]} */
function reset() {
  abortSync();
  set(initialState);
}

/** @type {WalletStoreServices["setCurrentProfile"]} */
async function setCurrentProfile(profile) {
  const store = get(walletStore);

  return store.profiles.includes(profile)
    ? Promise.resolve(set({ ...store, currentProfile: profile })).then(
        updateBalance
      )
    : Promise.reject(
        new Error("The received profile is not in the known list")
      );
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

        /**
         * Retrieving the nullifiers that are now spent.
         *
         * Currently `w3sper.js` returns an array of `ArrayBuffer`s
         * instead of one of `Uint8Array`s, but we don't
         * care as `ArrayBuffers` will be written in the
         * database anyway.
         */
        const spentNullifiers = await addressSyncer.spent(
          currentUnspentNullifiers
        );

        // update the cache with the spent nullifiers info
        await walletCache.spendNotes(spentNullifiers);

        // gather all spent nullifiers in the cache
        const currentSpentNullifiers =
          await walletCache.getUnspentNotesNullifiers();

        /**
         * Retrieving the nullifiers that are really spent given our
         * list of spent nullifiers.
         * We make this check because a block can be rejected and
         * we may end up having some notes marked as spent in the
         * cache, while they really aren't.
         *
         * Currently `w3sper.js` returns an array of `ArrayBuffer`s
         * instead of one of `Uint8Array`s.
         * @type {ArrayBuffer[]}
         */
        const reallySpentNullifiers = await addressSyncer.spent(
          currentSpentNullifiers
        );

        /**
         * As the previous `addressSyncer.spent` call returns a subset of
         * our spent nullifiers, we can skip this operation if the lengths
         * are the same.
         */
        if (reallySpentNullifiers.length !== currentSpentNullifiers.length) {
          const nullifiersToUnspend = nullifiersDifference(
            currentSpentNullifiers,
            map(reallySpentNullifiers, (buf) => new Uint8Array(buf))
          );

          await walletCache.unspendNotes(nullifiersToUnspend);
        }
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
const transfer = async (to, amount, gas) =>
  effectfulAction(() =>
    networkStore
      .connect()
      .then((network) => {
        const tx = bookkeeper
          .transfer(amount)
          .from(getCurrentAddress())
          .to(b58.decode(to))
          .gas(gas);

        return network.execute(
          ProfileGenerator.typeOf(to) === "address" ? tx.obfuscated() : tx
        );
      })
      .then(
        /** @type {(txInfo: TransactionInfo) => Promise<TransactionInfo>} */ async (
          txInfo
        ) => {
          /**
           * For now we ignore the possible error while
           * writing the pending notes info, as we'll
           * change soon how they are handled (probably by w3sper directly).
           */
          await walletCache
            .setPendingNoteInfo(txInfo.nullifiers, txInfo.hash)
            .catch(() => {});

          return txInfo;
        }
      )
  );

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
  setCurrentProfile,
  stake,
  subscribe,
  sync,
  transfer,
  unstake,
  withdrawReward,
};
