import { persisted } from "svelte-persisted-store";
import { browser } from "$app/environment";

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
  gasLimit: parseInt(import.meta.env.VITE_GAS_LIMIT_DEFAULT, 10),
  gasPrice: parseInt(import.meta.env.VITE_GAS_PRICE_DEFAULT, 10),
  hideStakingNotice: false,
  minAllowedStake: parseInt(import.meta.env.VITE_MINIMUM_ALLOWED_STAKE, 10),
  network: "testnet",
  userId: "",
};

const settingsStore = persisted(
  `${CONFIG.LOCAL_STORAGE_APP_KEY}-preferences`,
  initialState
);
const { set, subscribe, update } = settingsStore;

function reset() {
  set(initialState);
}

/** @type {SettingsStore} */
export default {
  reset,
  set,
  subscribe,
  update,
};
