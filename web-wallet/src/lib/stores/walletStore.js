import { get, writable } from "svelte/store";
import { getKey, setKey, uniquesBy } from "lamb";
import { Gas, Wallet } from "@dusk-network/dusk-wallet-js";
import settingsStore from "./settingsStore";

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
  initialized: false,
  syncStatus: { current: 0, error: null, isInProgress: false, last: 0 },
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

async function updateAfterSync() {
  const store = get(walletStore);

  // @ts-expect-error
  const balance = await walletInstance.getBalance(store.currentAddress);

  set({
    ...store,
    balance,
    syncStatus: initialState.syncStatus,
  });
}

const abortSync = () => {
  syncPromise && syncController?.abort();
  syncPromise = null;
};

/** @type {WalletStoreServices["clearLocalData"]} */
const clearLocalData = async () => walletInstance?.reset();

/** @type {WalletStoreServices["clearLocalDataAndInit"]} */
const clearLocalDataAndInit = (wallet, syncFromBlock) =>
  wallet.reset().then(() => init(wallet, syncFromBlock));

/** @type {WalletStoreServices["getCurrentBlockHeight"]} */
const getCurrentBlockHeight = async () => Wallet.networkBlockHeight;

/** @type {WalletStoreServices["getStakeInfo"]} */
const getStakeInfo = async () =>
  sync()
    // @ts-expect-error
    .then(() => walletInstance.stakeInfo(getCurrentAddress()))
    .then(fixStakeInfo);

/** @type {WalletStoreServices["getTransactionsHistory"]} */
const getTransactionsHistory = async () =>
  sync()
    // @ts-expect-error
    .then(() => walletInstance.history(getCurrentAddress()))
    .then(uniquesById);

/** @type {WalletStoreServices["reset"]} */
function reset() {
  abortSync();
  walletInstance = null;
  set(initialState);
}

/** @type {WalletStoreServices["init"]} */
async function init(wallet, syncFromBlock) {
  walletInstance = wallet;

  const addresses = await walletInstance.getPsks();
  const currentAddress = addresses[0];

  set({
    ...initialState,
    addresses,
    currentAddress,
    initialized: true,
  });

  sync(syncFromBlock).then(() => {
    settingsStore.update(setKey("userId", currentAddress));
  });
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
  syncedAction(() => {
    // @ts-expect-error
    return walletInstance.stake(
      getCurrentAddress(),
      amount,
      new Gas(gasSettings)
    );
  });

/** @type {WalletStoreServices["sync"]} */
function sync(from) {
  if (!walletInstance) {
    throw new Error("No wallet instance to sync");
  }

  if (!syncPromise) {
    const store = get(walletStore);

    set({
      ...store,
      syncStatus: {
        current: 0,
        error: null,
        isInProgress: true,
        last: 0,
      },
    });

    syncController = new AbortController();

    const syncOptions = {
      from,
      /** @type {(current: number, last: number) => void} */
      onblock: (current, last) => {
        set({
          ...store,
          syncStatus: {
            current,
            error: null,
            isInProgress: true,
            last,
          },
        });
      },
      signal: syncController.signal,
    };

    syncPromise = walletInstance
      .sync(syncOptions)
      .then(() => {
        if (syncController.signal.aborted) {
          throw new Error("Synchronization aborted");
        }
      })
      .then(updateAfterSync)
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
  syncedAction(() => {
    // @ts-expect-error
    return walletInstance.transfer(
      getCurrentAddress(),
      to,
      amount,
      new Gas(gasSettings)
    );
  });

/** @type {WalletStoreServices["unstake"]} */
const unstake = async (gasSettings) =>
  syncedAction(() => {
    // @ts-expect-error
    return walletInstance.unstake(getCurrentAddress(), new Gas(gasSettings));
  });

/** @type {WalletStoreServices["withdrawReward"]} */
const withdrawReward = async (gasSettings) =>
  syncedAction(() => {
    // @ts-expect-error
    return walletInstance.withdrawReward(
      getCurrentAddress(),
      new Gas(gasSettings)
    );
  });

/** @type {WalletStore} */
export default {
  abortSync,
  clearLocalData,
  clearLocalDataAndInit,
  getCurrentBlockHeight,
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
