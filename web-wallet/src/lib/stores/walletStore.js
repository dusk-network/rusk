import { get, writable } from "svelte/store";
import { setKey, setPathIn } from "lamb";
import {
  Bookkeeper,
  Bookmark,
  ProfileGenerator,
} from "$lib/vendor/w3sper.js/src/mod";

import walletCache from "$lib/wallet-cache";
import WalletTreasury from "$lib/wallet-treasury";

import { transactions } from "$lib/mock-data";

import networkStore from "./networkStore";
import settingsStore from "./settingsStore";

const AUTO_SYNC_INTERVAL = 5 * 60 * 1000;

let autoSyncId = 0;

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

const walletStore = writable(initialState);
const { set, subscribe, update } = walletStore;

const treasury = new WalletTreasury();
const bookkeeper = new Bookkeeper(treasury);

const getCurrentProfile = () => get(walletStore).currentProfile;

/** @type {(...args: any) => Promise<void>} */
const asyncNoopFailure = () => Promise.reject(new Error("Not implemented"));

/** @type {(txInfo: TransactionInfo) => void} */
const observeTxRemoval = (txInfo) => {
  networkStore.connect().then((network) =>
    network.transactions
      .withId(txInfo.hash)
      .once.removed()
      .then(() => sync())
      .finally(updateBalance)
  );
};

/** @type {<T>(fn: (v: T) => any) => (a: T) => T} */
const passThruWithEffects = (fn) => (a) => {
  fn(a);

  return a;
};

/** @type {(txInfo: TransactionInfo) => Promise<TransactionInfo>} */
const updateCacheAfterTransaction = async (txInfo) => {
  // we did a phoenix transaction
  if ("nullifiers" in txInfo) {
    /**
     * For now we ignore the possible error while
     * writing the pending notes info, as we'll
     * change soon how they are handled (probably by w3sper directly).
     */
    await walletCache
      .setPendingNoteInfo(txInfo.nullifiers, txInfo.hash)
      .catch(() => {});
  } else {
    const address = String(getCurrentProfile()?.address);
    const currentBalance = await walletCache.getBalanceInfo(address);

    /**
     * We update the stored `nonce` so that if a transaction is made
     * before the sync gives us an updated one, the transaction
     * won't be rejected by reusing the old value.
     */
    await walletCache.setBalanceInfo(
      address,
      setPathIn(currentBalance, "unshielded.nonce", txInfo.nonce)
    );
  }

  return txInfo;
};

/** @type {() => Promise<void>} */
const updateBalance = async () => {
  const { currentProfile } = get(walletStore);

  if (!currentProfile) {
    return;
  }

  const shielded = await bookkeeper.balance(currentProfile.address);
  const unshielded = await bookkeeper.balance(currentProfile.account);
  const balance = { shielded, unshielded };

  /**
   * We ignore the error as the cached balance is only
   * a nice to have for the user.
   */
  await walletCache
    .setBalanceInfo(currentProfile.address.toString(), balance)
    .catch(() => {});

  update((currentStore) => ({
    ...currentStore,
    balance,
  }));
};

/** @type {WalletStoreServices["abortSync"]} */
const abortSync = () => {
  window.clearTimeout(autoSyncId);
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
  const cachedBalance = await walletCache.getBalanceInfo(currentAddress);

  treasury.setProfiles([currentProfile]);

  set({
    ...initialState,
    balance: cachedBalance,
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
        error: null,
        from: 0n,
        isInProgress: true,
        last: 0n,
        progress: 0,
      },
    });

    syncController = new AbortController();

    const walletCacheSyncInfo = await walletCache.getSyncInfo();

    /*
     * Unless the user wants to sync from a specific block height,
     * we restart from the last stored bookmark.
     */
    const from = fromBlock ?? Bookmark.from(walletCacheSyncInfo.bookmark);

    let lastBlockHeight = 0n;

    update((currentStore) => ({
      ...currentStore,
      syncStatus: {
        ...currentStore.syncStatus,
        from: fromBlock ?? walletCacheSyncInfo.blockHeight,
      },
    }));

    syncPromise = Promise.resolve(syncController.signal)
      .then(async (signal) => {
        /** @type {(evt: CustomEvent) => void} */
        const syncIterationListener = ({ detail }) => {
          update((currentStore) => ({
            ...currentStore,
            syncStatus: {
              ...currentStore.syncStatus,
              last: detail.blocks.last,
              progress: detail.progress,
            },
          }));

          lastBlockHeight = detail.blocks.last;
        };

        await treasury.update(from, syncIterationListener, signal);

        // updating the last block height in the cache sync info
        await walletCache.setLastBlockHeight(lastBlockHeight);
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
      .then(() => {
        window.clearTimeout(autoSyncId);
        autoSyncId = window.setTimeout(() => {
          sync().finally(updateBalance);
        }, AUTO_SYNC_INTERVAL);
      })
      .catch((error) => {
        syncController?.abort();

        update((currentStore) => ({
          ...currentStore,
          syncStatus: {
            error,
            from: 0n,
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

/** @type {WalletStoreServices["shield"]} */
const shield = async (amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(
        bookkeeper.as(getCurrentProfile()).shield(amount).gas(gas)
      )
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["transfer"]} */
const transfer = async (to, amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) => {
      const tx = bookkeeper
        .as(getCurrentProfile())
        .transfer(amount)
        .to(to)
        .gas(gas);

      return network.execute(
        // @ts-ignore we don't have access to the AddressTransfer type
        ProfileGenerator.typeOf(to) === "address" ? tx.obfuscated() : tx
      );
    })
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["unshield"]} */
const unshield = async (amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(
        bookkeeper.as(getCurrentProfile()).unshield(amount).gas(gas)
      )
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

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
  shield,
  stake,
  subscribe,
  sync,
  transfer,
  unshield,
  unstake,
  withdrawReward,
};
