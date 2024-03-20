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

const initialState = {
  ...browserDefaults,
  currency: "USD",
  dashboardTransactionLimit: 5,
  gasLimit: parseInt(import.meta.env.VITE_GAS_LIMIT_DEFAULT, 10),
  gasPrice: parseInt(import.meta.env.VITE_GAS_PRICE_DEFAULT, 10),
  hideStakingNotice: false,
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

export default {
  reset,
  set,
  subscribe,
  update,
};
