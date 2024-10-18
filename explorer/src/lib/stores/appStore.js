import { get, writable } from "svelte/store";
import { browser } from "$app/environment";

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
const DEFAULT_STATS_FETCH_INTERVAL = DEFAULT_FETCH_INTERVAL;

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
  nodeInfo: {
    /* eslint-disable camelcase */
    bootstrapping_nodes: [],
    chain_id: undefined,
    kadcast_address: "",
    version: "",
    version_build: "",
    /* eslint-enable camelcase */
  },
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

/** @param {NodeInfo} nodeInfo */
const setNodeInfo = (nodeInfo) => {
  set({
    ...get(store),
    nodeInfo,
  });
};

/** @param {boolean} darkMode */
const setTheme = (darkMode) => {
  set({
    ...get(store),
    darkMode,
  });
};

/** @type {AppStore} */
export default {
  setNodeInfo,
  setTheme,
  subscribe,
};
