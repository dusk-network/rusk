import { get, writable } from "svelte/store";
import { browser } from "$app/environment";

/** @type {NetworkOption[]}*/
const networks = [
  { label: "Local", value: new URL("/", import.meta.url) },
  {
    label: "Devnet",
    value: new URL(
      `${window.location.protocol}${import.meta.env.VITE_DUSK_DEVNET_NODE}`
    ),
  },
  {
    label: "Testnet",
    value: new URL(
      `${window.location.protocol}${import.meta.env.VITE_DUSK_TESTNET_NODE}`
    ),
  },
  {
    label: "Mainnet",
    value: new URL(
      `${window.location.protocol}${import.meta.env.VITE_DUSK_MAINNET_NODE}`
    ),
  },
];
const maxWidthMediaQuery = window.matchMedia("(max-width: 1024px)");
const browserDefaults = browser
  ? {
      darkMode: window.matchMedia("(prefers-color-scheme: dark)").matches,
    }
  : {
      darkMode: false,
    };
const DEFAULT_FETCH_INTERVAL = 1000;
const DEFAULT_MARKET_FETCH_INTERVAL = 120000;
const DEFAULT_NETWORK_INDEX = 0;
const DEFAULT_STATS_FETCH_INTERVAL = DEFAULT_FETCH_INTERVAL;

function getNetwork() {
  const index =
    Number(import.meta.env.VITE_DEFAULT_NETWORK) || DEFAULT_NETWORK_INDEX;
  return (
    networks[index]?.value.host || networks[DEFAULT_NETWORK_INDEX].value.host
  );
}

/** @type {AppStoreContent} */
const initialState = {
  ...browserDefaults,
  blocksListEntries: Number(import.meta.env.VITE_BLOCKS_LIST_ENTRIES),
  chainInfoEntries: Number(import.meta.env.VITE_CHAIN_INFO_ENTRIES),
  fetchInterval:
    Number(import.meta.env.VITE_REFETCH_INTERVAL) || DEFAULT_FETCH_INTERVAL,
  hasTouchSupport: "ontouchstart" in window || navigator.maxTouchPoints > 0,
  isSmallScreen: maxWidthMediaQuery.matches,
  marketDataFetchInterval:
    Number(import.meta.env.VITE_MARKET_DATA_REFETCH_INTERVAL) ||
    DEFAULT_MARKET_FETCH_INTERVAL,
  network: getNetwork(),
  networks,
  statsFetchInterval:
    Number(import.meta.env.VITE_STATS_REFETCH_INTERVAL) ||
    DEFAULT_STATS_FETCH_INTERVAL,
  transactionsListEntries: Number(
    import.meta.env.VITE_TRANSACTIONS_LIST_ENTRIES
  ),
};
const store = writable(initialState);
const { set, subscribe } = store;

maxWidthMediaQuery.addEventListener("change", (event) => {
  set({
    ...get(store),
    isSmallScreen: event.matches,
  });
});

/** @param {string} network */
const setNetwork = (network) =>
  set({
    ...get(store),
    network,
  });

/** @param {boolean} darkMode */
const setTheme = (darkMode) => {
  set({
    ...get(store),
    darkMode,
  });
};

/** @type {AppStore} */
export default {
  setNetwork,
  setTheme,
  subscribe,
};
