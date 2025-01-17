import { writable } from "svelte/store";
import { browser } from "$app/environment";

// =========================
// --- Utilities ---
// =========================

export const serializeProperty = (/** @type {any} */ value) =>
  JSON.stringify(value, (_, v) => (typeof v === "bigint" ? `${v}n` : v));

export const deserializeProperty = (/** @type {string} */ value) =>
  JSON.parse(value, (_, v) =>
    typeof v === "string" && /^\d+n$/.test(v) ? BigInt(v.slice(0, -1)) : v
  );

// =========================
// --- End of Utilities ---
// =========================

const browserDefaults = browser
  ? {
      darkMode: window.matchMedia("(prefers-color-scheme: dark)").matches,
      language: navigator.language,
    }
  : {
      darkMode: false,
      language: "en",
    };

/** @type {SettingsStoreContent} */
const initialState = {
  ...browserDefaults,
  currency: "USD",
  dashboardTransactionLimit: 5,
  gasLimit: BigInt(import.meta.env.VITE_GAS_LIMIT_DEFAULT ?? 100_000_000),
  gasPrice: BigInt(import.meta.env.VITE_GAS_PRICE_DEFAULT ?? 1),
  hideStakingNotice: false,
  userId: "",
  walletCreationBlockHeight: 0n,
};

const createPersistedStore = (
  /** @type {string} */ key,
  /** @type {SettingsStoreContent} */ initialValue
) => {
  const load = () => {
    if (!browser) {
      return initialValue;
    }
    const storedValue = localStorage.getItem(key);
    return storedValue ? deserializeProperty(storedValue) : initialValue;
  };

  const store = writable(load());

  if (browser) {
    store.subscribe((value) => {
      localStorage.setItem(key, serializeProperty(value));
    });
  }

  return store;
};

const settingsStore = createPersistedStore(
  `${CONFIG.LOCAL_STORAGE_APP_KEY}-preferences`,
  initialState
);

const { set, subscribe, update } = settingsStore;

// Reset store to initial state
const reset = () => set(initialState);

// Resets only gas settings to their defaults.
const resetGasSettings = () =>
  update((current) => ({
    ...current,
    gasLimit: initialState.gasLimit,
    gasPrice: initialState.gasPrice,
  }));

/** @type {SettingsStore} */
export default {
  reset,
  resetGasSettings,
  set,
  subscribe,
  update,
};
