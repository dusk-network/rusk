import { get, writable } from "svelte/store";
import { setKey } from "lamb";
import { Bookkeeper, Bookmark, ProfileGenerator } from "@dusk/w3sper";

import WalletTreasury from "$lib/wallet-treasury";

import { transactions } from "$lib/mock-data";

import networkStore from "./networkStore";
import settingsStore from "./settingsStore";

const VITE_SYNC_INTERVAL = import.meta.env.VITE_SYNC_INTERVAL;
const AUTO_SYNC_INTERVAL = !isNaN(Number(VITE_SYNC_INTERVAL))
  ? Number(VITE_SYNC_INTERVAL)
  : 5 * 60 * 1000;

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

function unsafeGetCurrentProfile() {
  const profile = getCurrentProfile();

  if (profile === null) {
    throw new TypeError("Can't retrieve profile: wallet not initialized");
  } else {
    return profile;
  }
}

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
    await treasury
      .updateCachedPendingNotes(txInfo.nullifiers, txInfo.hash)
      .catch(() => {});
  } else {
    const profile = getCurrentProfile();

    /**
     * We update the stored `nonce` so that if a transaction is made
     * before the sync gives us an updated one, the transaction
     * won't be rejected by reusing the old value.
     */
    profile && (await treasury.updateCachedNonce(profile, txInfo.nonce));
  }

  return txInfo;
};

/** @type {() => Promise<void>} */
const updateBalance = async () => {
  const profile = getCurrentProfile();

  if (!profile) {
    return;
  }

  const shielded = /** @type {AddressBalance} */ (
    await bookkeeper.balance(profile.address)
  );
  const unshielded = /** @type {AccountBalance} */ (
    await bookkeeper.balance(profile.account)
  );
  const balance = { shielded, unshielded };

  /**
   * We ignore the error as the cached balance is only
   * a nice to have for the user.
   */
  await treasury.setCachedBalance(profile, balance).catch(() => {});

  update((currentStore) => ({
    ...currentStore,
    balance,
  }));
};

/** @type {() => Promise<void>} */
const updateStakeInfo = async () => {
  const profile = getCurrentProfile();

  if (!profile) {
    return;
  }

  /** @type {StakeInfo} */
  const stakeInfo = await bookkeeper.stakeInfo(profile.account);

  /**
   * We ignore the error as the cached stake info is only
   * a nice to have for the user.
   */
  await treasury.setCachedStakeInfo(profile, stakeInfo).catch(() => {});

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
const clearLocalData = async () => {
  abortSync();

  await treasury.clearCache();
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
        bookkeeper.as(unsafeGetCurrentProfile()).withdraw(amount).gas(gas)
      )
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["getTransactionsHistory"]} */
const getTransactionsHistory = async () => transactions;

/** @type {WalletStoreServices["init"]} */
async function init(profileGeneratorInstance, syncFromBlock) {
  // Create two profiles by default
  const currentProfile = await profileGeneratorInstance.default;
  const secondProfile = await profileGeneratorInstance.next();
  const profiles = [currentProfile, secondProfile];

  const cachedBalance = await treasury.getCachedBalance(currentProfile);
  const cachedStakeInfo = await treasury.getCachedStakeInfo(currentProfile);
  const minimumStake = await bookkeeper.minimumStake;

  treasury.setProfiles(profiles);

  set({
    ...initialState,
    balance: cachedBalance,
    currentProfile,
    initialized: true,
    minimumStake,
    profiles,
    stakeInfo: cachedStakeInfo,
  });

  sync(syncFromBlock)
    .then(() => {
      settingsStore.update(setKey("userId", currentProfile.address.toString()));
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
        bookkeeper.as(unsafeGetCurrentProfile()).shield(amount).gas(gas)
      )
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["stake"]} */
const stake = async (amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(
        bookkeeper.as(unsafeGetCurrentProfile()).stake(amount).gas(gas)
      )
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
      await treasury.getCachedSyncInfo();

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
      await treasury.clearCache();
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
const transfer = async (to, amount, memo, gas) =>
  sync()
    .then(networkStore.connect)
    .then(async (network) => {
      const tx = bookkeeper
        .as(unsafeGetCurrentProfile())
        .transfer(amount)
        .to(to)
        .memo(memo)
        .gas(gas);

      if (ProfileGenerator.typeOf(to) === "address") {
        // @ts-ignore
        tx.obfuscated();
      }

      return await network.execute(tx);
    })
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["depositEvmFunctionCall"]} */
const depositEvmFunctionCall = async (
  address,
  amount,
  contractId,
  wasmPath,
  gas
) =>
  sync()
    .then(networkStore.connect)
    .then(async (network) => {
      network.dataDrivers.register(contractId, () =>
        fetch(wasmPath).then((r) => r.arrayBuffer())
      );

      const payloadAmount = Number(amount);
      const profile = unsafeGetCurrentProfile();
      const bookentry = bookkeeper.as(profile);
      const bridgeContract = bookentry.contract(contractId, network);

      /* eslint-disable camelcase */
      const payload = {
        amount: payloadAmount,
        extra_data: "",
        fee: 500000,
        to: address,
      };
      /* eslint-enable camelcase */

      const builder = await bridgeContract.tx.deposit(payload);

      return await network.execute(
        builder
          .to(profile.account)
          .deposit(BigInt(payloadAmount + payload.fee))
          .gas(gas)
      );
    })
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["finalizeWithdrawalEvmFunctionCall"]} */
const finalizeWithdrawalEvmFunctionCall = async (
  contractId,
  withdrawalId,
  wasmPath
) =>
  sync()
    .then(networkStore.connect)
    .then(async (network) => {
      network.dataDrivers.register(contractId, () =>
        fetch(wasmPath).then((r) => r.arrayBuffer())
      );

      const profile = unsafeGetCurrentProfile();
      const bookentry = bookkeeper.as(profile);
      const contract = bookentry.contract(contractId, network);
      const builder = await contract.tx.finalize_withdrawal(withdrawalId);

      return await network.execute(builder.to(profile.account));
    })
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["unshield"]} */
const unshield = async (amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(
        bookkeeper.as(unsafeGetCurrentProfile()).unshield(amount).gas(gas)
      )
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["unstake"]} */
const unstake = async (amount, gas) =>
  sync()
    .then(networkStore.connect)
    .then((network) =>
      network.execute(
        bookkeeper.as(unsafeGetCurrentProfile()).unstake(amount).gas(gas)
      )
    )
    .then(updateCacheAfterTransaction)
    .then(passThruWithEffects(observeTxRemoval));

/** @type {WalletStoreServices["useContract"]} */
const useContract = async (contractId, wasmPath) =>
  networkStore.connect().then(async (network) => {
    network.dataDrivers.register(contractId, () =>
      fetch(wasmPath).then((r) => r.arrayBuffer())
    );

    const profile = unsafeGetCurrentProfile();
    const bookentry = bookkeeper.as(profile);
    const contract = bookentry.contract(contractId, network);

    return contract;
  });

/** @type {WalletStore} */
export default {
  abortSync,
  claimRewards,
  clearLocalData,
  clearLocalDataAndInit,
  depositEvmFunctionCall,
  finalizeWithdrawalEvmFunctionCall,
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
  useContract,
};
