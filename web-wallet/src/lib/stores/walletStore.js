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
    publicBalance: {
      nonce: 0n,
      value: 0n,
    },
    shieldedBalance: {
      spendable: 0n,
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

const walletStore = writable(initialState);
const { set, subscribe, update } = walletStore;

const treasury = new WalletTreasury();
const bookkeeper = new Bookkeeper(treasury);

const getCurrentProfile = () => get(walletStore).currentProfile;

/** @type {(txInfo: TransactionInfo) => void} */
const observeTxRemoval = (txInfo) => {
  networkStore.connect().then((network) =>
    network.transactions
      .withId(txInfo.hash)
      .once.removed()
      .then(() => sync())
      .finally(updateStaticInfo)
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
      setPathIn(currentBalance, "public.nonce", txInfo.nonce)
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

  const shieldedBalance = await bookkeeper.balance(currentProfile.address);
  const publicBalance = await bookkeeper.balance(currentProfile.account);
  const balance = { publicBalance, shieldedBalance };

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

/** @type {() => Promise<void>} */
const updateStakeInfo = async () => {
  const { currentProfile } = get(walletStore);

  if (!currentProfile) {
    return;
  }

  const stakeInfo = await bookkeeper.stakeInfo(currentProfile.account);

  /**
   * We ignore the error as the cached stake info is only
   * a nice to have for the user.
   */
  await walletCache
    .setStakeInfo(currentProfile.account.toString(), stakeInfo)
    .catch(() => {});

  update((currentStore) => ({
    ...currentStore,
    stakeInfo,
  }));
};

const updateStaticInfo = () =>
  Promise.allSettled([updateBalance(), updateStakeInfo()]);

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

/** @type {WalletStoreServices["claimRewards"]} */
const claimRewards = async (amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(
        bookkeeper.as(getCurrentProfile()).withdraw(amount).gas(gas)
      )
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["getTransactionsHistory"]} */
const getTransactionsHistory = async () => transactions;

/** @type {WalletStoreServices["init"]} */
async function init(profileGenerator, syncFromBlock) {
  const currentProfile = await profileGenerator.default;
  const currentAddress = currentProfile.address.toString();
  const cachedBalance = await walletCache.getBalanceInfo(currentAddress);
  const cachedStakeInfo = await walletCache.getStakeInfo(
    currentProfile.account.toString()
  );
  const minimumStake = await bookkeeper.minimumStake;

  treasury.setProfiles([currentProfile]);

  set({
    ...initialState,
    balance: cachedBalance,
    currentProfile,
    initialized: true,
    minimumStake,
    profiles: [currentProfile],
    stakeInfo: cachedStakeInfo,
  });

  sync(syncFromBlock)
    .then(() => {
      settingsStore.update(setKey("userId", currentAddress));
    })
    .finally(updateStaticInfo);
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
        async () => {
          await updateStaticInfo();
        }
      )
    : Promise.reject(
        new Error("The received profile is not in the known list")
      );
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

/** @type {WalletStoreServices["stake"]} */
const stake = async (amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(bookkeeper.as(getCurrentProfile()).stake(amount).gas(gas))
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["sync"]} */
// eslint-disable-next-line max-statements
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

    const { block, bookmark, lastFinalizedBlockHeight } =
      await walletCache.getSyncInfo();

    /** @type {bigint | Bookmark} */
    let from;

    /*
     * Unless the user wants to sync from a specific block height,
     * we try to restart from the last stored bookmark.
     * Before doing that we compare the block hash we have in cache
     * with the hash at the same block height on the network: if
     * they don't match then a block has been rejected, we can't
     * use our bookmark, and our only safe option is to restart
     * from the last finalized block we have cached.
     */
    if (fromBlock) {
      from = fromBlock;
    } else {
      const isLocalCacheValid = await networkStore
        .checkBlock(block.height, block.hash)
        .catch(() => false);

      from = isLocalCacheValid
        ? Bookmark.from(bookmark)
        : lastFinalizedBlockHeight;
    }

    if (from === 0n) {
      await walletCache.clear();
    }

    update((currentStore) => ({
      ...currentStore,
      syncStatus: {
        ...currentStore.syncStatus,
        from: from instanceof Bookmark ? block.height : from,
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
        };

        await treasury.update(from, syncIterationListener, signal);
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
          sync().finally(updateStaticInfo);
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
const unstake = async (gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(bookkeeper.as(getCurrentProfile()).unstake().gas(gas))
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStore} */
export default {
  abortSync,
  claimRewards,
  clearLocalData,
  clearLocalDataAndInit,
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
};
